//! Private Base4 transport. Business phases remain in the original activation machine.
use super::container_control::{
    ContainerControl, ContainerLaunchFiles, ContainerRegistration, canonical_hash,
};
use app::container_activation::held;
use serde::Deserialize;
use serde_json::{Value, json};
use shared::AppError;
use std::path::PathBuf;

#[derive(Clone)]
pub(super) struct Replacement {
    pub anchor: Value,
    pub command: Value,
    pub journal: PathBuf,
    // A new permit is issued only after this worker's successful original phase CAS.
    pub first: bool,
    pub stop_first: bool,
}

impl std::fmt::Debug for Replacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Replacement")
            .field("first", &self.first)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    protocol_version: u8,
    action: String,
    result: Value,
    command_sha256: String,
    intent_sha256: String,
    anchor_sha256: String,
}

impl Replacement {
    fn action(&self, action: &str) -> Result<&'static str, AppError> {
        Ok(match action {
            "prepare" if self.first => "prepare_replacement",
            "prepare" | "reconcile_preparation" => "reconcile_replacement",
            "attach_controller" if self.first => "attach_replacement",
            "attach_controller" => "reconcile_replacement_attachment",
            "start" if self.first => "start_replacement",
            "start" | "observe" => "observe_replacement",
            "endpoint_attached" => "endpoint_replacement",
            "stop" if self.first || self.stop_first => "stop_replacement",
            "stop" => "reconcile_replacement_stop",
            _ => return Err(held()),
        })
    }

    fn validate(&self, bytes: &[u8], action: &str) -> Result<Value, AppError> {
        let e: Envelope = serde_json::from_slice(bytes).map_err(|_| held())?;
        let mut identity = self.anchor.clone();
        let object = identity.as_object_mut().ok_or_else(held)?;
        for name in ["recovery", "action", "protocol_version"] {
            object.remove(name);
        }
        if e.protocol_version != 4
            || e.action != action
            || e.command_sha256 != canonical_hash(&self.command)?
            || Some(e.intent_sha256.as_str()) != self.command["intent_sha256"].as_str()
            || e.anchor_sha256 != canonical_hash(&identity)?
        {
            return Err(held());
        }
        Ok(e.result)
    }

    pub async fn call(
        &self,
        control: &ContainerControl,
        files: &ContainerLaunchFiles,
        action: &str,
        extra: &Value,
    ) -> Result<(i32, Value), AppError> {
        self.validate_inputs(files, action, extra)?;
        let action = self.action(action)?;
        let request = json!({"protocol_version":4,"action":action,"anchor":self.anchor,
            "replacement_journal":self.journal,"command":self.command});
        let (status, bytes) = control.execute_request(&request).await?;
        if status != 0 {
            return Err(held());
        }
        Ok((status, self.validate(&bytes, action)?))
    }

    fn validate_inputs(
        &self,
        files: &ContainerLaunchFiles,
        action: &str,
        extra: &Value,
    ) -> Result<(), AppError> {
        let mut policy = files.policy.clone();
        policy["network"]["id"] = json!("0".repeat(64));
        if policy != self.command["policy"]
            || json!(files.compose) != self.command["compose"]
            || json!(files.journal) != self.command["journal"]
            || json!(files.stop_journal) != self.command["stop_journal"]
            || files.recovery.is_some()
            || json!(files.mapped.as_ref().ok_or_else(held)?.mapping)
                != self.command["mount_mapping"]
        {
            return Err(held());
        }
        if let Some(r) = extra.get("registration") {
            if r["operation_id"] != self.command["operation_id"]
                || r["generation"] != self.command["policy"]["generation"]
                || r["resource_id"] != self.command["policy"]["resource_id"]
            {
                return Err(held());
            }
        }
        if action == "stop" && extra["operation_id"] != self.command["stop_id"] {
            return Err(held());
        }
        if matches!(action, "prepare" | "reconcile_preparation")
            && (extra["operation_id"] != self.command["operation_id"]
                || extra["process"] != self.command["process"]
                || extra["creation_compose"] != self.command["creation_compose"]
                || extra["creation_journal"] != self.command["creation_journal"])
        {
            return Err(held());
        }
        Ok(())
    }

    pub fn validate_attachment(
        &self,
        value: &Value,
        r: &ContainerRegistration,
        policy: &Value,
    ) -> Result<(), AppError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Attachment {
            state: String,
            registration_sha256: String,
            controller_id: String,
            controller_sha256: String,
            network_id: String,
            original_attachment_sha256: String,
        }
        let a: Attachment = serde_json::from_value(value.clone()).map_err(|_| held())?;
        let witness = &self.anchor["recovery"]["request"]["controller_snapshot"];
        if a.state != "attached"
            || a.registration_sha256 != canonical_hash(r)?
            || Some(a.controller_id.as_str()) != witness["container_id"].as_str()
            || a.controller_sha256 != canonical_hash(witness)?
            || Some(a.network_id.as_str()) != policy["network"]["id"].as_str()
            || a.original_attachment_sha256.len() != 64
            || !a
                .original_attachment_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(held());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mismatched_launch_inputs_are_rejected_before_native_transport() {
        let (_, mapping, _) = super::super::container_mapping::tests::fixture();
        let files = ContainerLaunchFiles {
            policy: json!({"network":{"id":"0".repeat(64)}}),
            compose: "/private/child.compose".into(),
            journal: "/private/child.sqlite".into(),
            stop_journal: "/private/child-stop.sqlite".into(),
            recovery: None,
            mapped: Some(app::container_runtime::MappedContainer {
                mapping,
                mapping_file: "/private/map".into(),
                attachment_journal: "/private/attach".into(),
                recovery_journal: "/private/recover".into(),
            }),
        };
        let mut r = fixture(true);
        r.command = json!({"policy":files.policy,"compose":files.compose,"journal":files.journal,"stop_journal":files.stop_journal,
            "mount_mapping":files.mapped.as_ref().unwrap().mapping,"stop_id":"original-stop"});
        assert!(r.validate_inputs(&files, "observe", &json!({})).is_ok());
        let mut wrong = files.clone();
        wrong.compose = "/private/foreign.compose".into();
        assert!(r.validate_inputs(&wrong, "observe", &json!({})).is_err());
        assert!(
            r.validate_inputs(&files, "stop", &json!({"operation_id":"foreign-stop"}))
                .is_err()
        );
        assert!(
            r.validate_inputs(
                &files,
                "prepare",
                &json!({"operation_id":"foreign-prepare"})
            )
            .is_err()
        );
        let mut unknown = json!({"protocol_version":4,"action":"observe_replacement","result":{},"command_sha256":"a".repeat(64),
            "intent_sha256":"a".repeat(64),"anchor_sha256":"a".repeat(64)});
        unknown["receipt"] = json!({"origin":"http://foreign"});
        assert!(
            r.validate(
                &serde_json::to_vec(&unknown).unwrap(),
                "observe_replacement"
            )
            .is_err()
        );
    }
    fn fixture(first: bool) -> Replacement {
        Replacement {
            anchor: json!({"protocol_version":3,"action":"observe","recovery":{"lease_version":1},"registration":{"generation":"original"}}),
            command: json!({"intent_sha256":"a".repeat(64)}),
            journal: "/private/replacement.sqlite".into(),
            first,
            stop_first: false,
        }
    }
    #[test]
    fn recovered_reentry_has_no_new_native_permits() {
        let replay = fixture(false);
        for (input, expected) in [
            ("prepare", "reconcile_replacement"),
            ("attach_controller", "reconcile_replacement_attachment"),
            ("start", "observe_replacement"),
            ("stop", "reconcile_replacement_stop"),
        ] {
            assert_eq!(replay.action(input).unwrap(), expected);
        }
        assert!(replay.action("resolve_mounts").is_err());
        assert_eq!(
            fixture(true).action("prepare").unwrap(),
            "prepare_replacement"
        );
        let mut stopped = replay.clone();
        stopped.stop_first = true;
        assert_eq!(stopped.action("stop").unwrap(), "stop_replacement");
        assert_eq!(stopped.action("start").unwrap(), "observe_replacement");
        assert_eq!(stopped.action("prepare").unwrap(), "reconcile_replacement");
        assert_eq!(
            stopped.action("attach_controller").unwrap(),
            "reconcile_replacement_attachment"
        );
        let mut secret = stopped;
        secret.command = json!({"process":{"environment":{"API_SERVER_KEY":"private-credential"}}});
        secret.anchor = json!({"private":"private-anchor"});
        let debug = format!("{secret:?}");
        assert!(!debug.contains("private-credential") && !debug.contains("private-anchor"));
    }
    #[test]
    fn response_requires_original_command_and_anchor_hashes() {
        let r = fixture(false);
        let mut identity = r.anchor.clone();
        for key in ["recovery", "action", "protocol_version"] {
            identity.as_object_mut().unwrap().remove(key);
        }
        let value = json!({"protocol_version":4,"action":"observe_replacement","result":{},
            "command_sha256":canonical_hash(&r.command).unwrap(),"intent_sha256":r.command["intent_sha256"],"anchor_sha256":canonical_hash(&identity).unwrap()});
        assert!(
            r.validate(&serde_json::to_vec(&value).unwrap(), "observe_replacement")
                .is_ok()
        );
        for key in ["command_sha256", "intent_sha256", "anchor_sha256"] {
            let mut bad = value.clone();
            bad[key] = json!("b".repeat(64));
            assert!(
                r.validate(&serde_json::to_vec(&bad).unwrap(), "observe_replacement")
                    .is_err()
            );
        }
        let mut heartbeat = r.clone();
        heartbeat.anchor["recovery"]["lease_version"] = json!(2);
        assert!(
            heartbeat
                .validate(&serde_json::to_vec(&value).unwrap(), "observe_replacement")
                .is_ok()
        );
    }
}
