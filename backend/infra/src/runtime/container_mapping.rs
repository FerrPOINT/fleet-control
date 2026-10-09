//! Local paths are custody evidence, never daemon bind-mount authority.
use super::container_control::canonical_hash;
use app::container_runtime::ContainerMapping;
use serde_json::{Value, json};
use shared::{AppError, config::MappingControllerConfig};

fn held() -> AppError {
    AppError::Unavailable("Original named-volume mapping requires reconciliation".into())
}

fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn directory(s: &str) -> bool {
    s.starts_with('/')
        && s != "/"
        && s.len() <= 4096
        && !s.contains(['\\', '\0'])
        && s[1..].split('/').all(|p| !matches!(p, "" | "." | ".."))
}

pub(super) fn local_policy(
    policy: &Value,
    m: &ContainerMapping,
    controller: &MappingControllerConfig,
    root: &str,
) -> Result<Value, AppError> {
    if policy["contract_version"] != 3
        || !policy["network"].is_object()
        || m.state != "resolved"
        || m.local_root != root
        || !directory(root)
        || m.controller.container_id != controller.container_id
        || m.controller.image_id != controller.image_id
        || m.controller.service != controller.service
        || !hash(&controller.container_id)
        || !controller
            .image_id
            .strip_prefix("sha256:")
            .is_some_and(hash)
        || !matches!(
            controller.service.as_str(),
            "fleet-backend" | "fleet-control-backend"
        )
        || m.snapshot.container_id != controller.container_id
        || m.snapshot.init_pid == 0
        || m.snapshot.started_at.starts_with("0001-")
        || chrono::DateTime::parse_from_rfc3339(&m.snapshot.started_at).is_err()
        || [
            &m.snapshot.inventory_sha256,
            &m.volume_sha256,
            &m.input_policy_sha256,
        ]
        .iter()
        .any(|s| !hash(s))
        || [
            &m.engine.id,
            &m.engine.kernel_version,
            &m.engine.server_version,
        ]
        .iter()
        .any(|s| s.is_empty() || s.len() > 256 || !s.is_ascii())
        || m.volume_name.is_empty()
        || m.volume_name.len() > 128
        || !m.volume_name.as_bytes()[0].is_ascii_alphanumeric()
        || !m
            .volume_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        || m.mounts.len() != 4
    {
        return Err(held());
    }
    let mounts = policy["mounts"]
        .as_array()
        .filter(|v| v.len() == 4)
        .ok_or_else(held)?;
    let mut agent = None;
    let mut base = None;
    let mut local = policy.clone();
    let mut expected = Vec::new();
    for ((mount, projected), area) in
        mounts
            .iter()
            .zip(&m.mounts)
            .zip(["runtime", "config", "workspace", "logs"])
    {
        let (parent, leaf) = projected.source.rsplit_once('/').ok_or_else(held)?;
        let (daemon_root, name) = parent.rsplit_once('/').ok_or_else(held)?;
        let ordinal = name.strip_prefix("agent").ok_or_else(held)?;
        if !directory(&projected.source)
            || !directory(daemon_root)
            || ordinal.is_empty()
            || ordinal.starts_with('0')
            || !ordinal.bytes().all(|b| b.is_ascii_digit())
            || leaf != area
            || projected.mount_type != "bind"
            || projected.destination != format!("/{area}")
            || projected.read_only != (area == "runtime")
            || agent.is_some_and(|a| a != name)
            || base.is_some_and(|b| b != daemon_root)
            || *mount
                != json!({"type":"volume","source":m.volume_name,"subpath":format!("{name}/{area}"),
                "destination":format!("/{area}"),"read_only":area=="runtime"})
        {
            return Err(held());
        }
        agent = Some(name);
        base = Some(daemon_root);
        expected.push(json!({"type":"bind","source":format!("{root}/{name}/{area}"),"destination":format!("/{area}"),"read_only":area=="runtime"}));
    }
    local["contract_version"] = json!(2);
    local["network"]["id"] = json!("0".repeat(64));
    local["mounts"] = json!(expected);
    if canonical_hash(&local)? != m.input_policy_sha256 {
        return Err(held());
    }
    Ok(local)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use app::container_runtime::*;

    pub(crate) fn fixture() -> (Value, ContainerMapping, MappingControllerConfig) {
        let controller = MappingControllerConfig {
            container_id: "a".repeat(64),
            image_id: format!("sha256:{}", "b".repeat(64)),
            service: "fleet-backend".into(),
        };
        let mut local = json!({"contract_version":2,"network":{"id":"0".repeat(64)},"mounts":[]});
        local["mounts"] = json!(["runtime","config","workspace","logs"].map(|a| json!({"type":"bind","source":format!("/agents/agent1/{a}"),"destination":format!("/{a}"),"read_only":a=="runtime"})));
        let mapping = ContainerMapping {
            state: "resolved".into(),
            controller: MappingController {
                container_id: controller.container_id.clone(),
                image_id: controller.image_id.clone(),
                service: controller.service.clone(),
            },
            snapshot: ControllerSnapshot {
                container_id: controller.container_id.clone(),
                started_at: "2026-10-09T12:00:00Z".into(),
                init_pid: 50,
                inventory_sha256: "c".repeat(64),
            },
            engine: ContainerEngineIdentity {
                id: "engine".into(),
                kernel_version: "kernel".into(),
                server_version: "29".into(),
            },
            local_root: "/agents".into(),
            volume_name: "sdlc1_fleet_agents".into(),
            volume_sha256: "d".repeat(64),
            input_policy_sha256: canonical_hash(&local).unwrap(),
            mounts: ["runtime", "config", "workspace", "logs"]
                .map(|a| ProjectedMount {
                    mount_type: "bind".into(),
                    source: format!("/var/lib/docker/volumes/sdlc1_fleet_agents/_data/agent1/{a}"),
                    destination: format!("/{a}"),
                    read_only: a == "runtime",
                })
                .to_vec(),
        };
        let mut mapped = local;
        mapped["contract_version"] = json!(3);
        mapped["network"]["id"] = json!("e".repeat(64));
        mapped["mounts"] = json!(["runtime","config","workspace","logs"].map(|a| json!({"type":"volume","source":mapping.volume_name,"subpath":format!("agent1/{a}"),"destination":format!("/{a}"),"read_only":a=="runtime"})));
        (mapped, mapping, controller)
    }

    #[test]
    fn mapped_policy_retains_original_local_recipe_and_volume_subpaths() {
        let (p, m, c) = fixture();
        let local = local_policy(&p, &m, &c, "/agents").unwrap();
        assert_eq!(canonical_hash(&local).unwrap(), m.input_policy_sha256);
        assert_eq!(local["mounts"][0]["source"], "/agents/agent1/runtime");
        assert_ne!(local["mounts"], p["mounts"]);
    }

    #[test]
    fn unicode_local_root_matches_original_base_policy_hash() {
        let (p, mut m, c) = fixture();
        let root = "/agents-\u{430}-\u{1f600}";
        m.local_root = root.into();
        // Base canonical JSON, not a hash generated by the Rust implementation under test.
        m.input_policy_sha256 =
            "2342a8f0739124a22b0242dbe7be01c44f312d6f92ca9b1ea66047192ced4bd2".into();
        let local = local_policy(&p, &m, &c, root).unwrap();
        assert_eq!(
            local["mounts"][0]["source"],
            format!("{root}/agent1/runtime")
        );
        assert_eq!(canonical_hash(&local).unwrap(), m.input_policy_sha256);
        let mut changed = m.clone();
        changed.local_root.push('x');
        assert!(local_policy(&p, &changed, &c, &changed.local_root).is_err());
    }

    #[test]
    fn controller_root_image_and_service_are_not_adopted_from_mapping() {
        let (p, m, mut c) = fixture();
        assert!(local_policy(&p, &m, &c, "/other").is_err());
        c.container_id = "f".repeat(64);
        assert!(local_policy(&p, &m, &c, "/agents").is_err());
        c = fixture().2;
        c.image_id = format!("sha256:{}", "f".repeat(64));
        assert!(local_policy(&p, &m, &c, "/agents").is_err());
        c = fixture().2;
        c.service = "other-backend".into();
        assert!(local_policy(&p, &m, &c, "/agents").is_err());
    }

    #[test]
    fn cross_agent_volume_alias_and_writable_runtime_are_rejected() {
        let (p, m, c) = fixture();
        for (key, value) in [
            ("subpath", json!("agent2/config")),
            ("source", json!("other_volume")),
            ("read_only", json!(false)),
        ] {
            let mut changed = p.clone();
            changed["mounts"][0][key] = value;
            assert!(local_policy(&changed, &m, &c, "/agents").is_err());
        }
        let mut changed = m.clone();
        changed.mounts[1].source = "/var/lib/docker/volumes/other/_data/agent2/config".into();
        assert!(local_policy(&p, &changed, &c, "/agents").is_err());
    }

    #[test]
    fn traversal_missing_custody_and_changed_input_hash_are_rejected() {
        let (p, m, c) = fixture();
        for path in ["/agents/../private", "/agents//agent1", "relative", "/"] {
            let mut changed = m.clone();
            changed.local_root = path.into();
            assert!(local_policy(&p, &changed, &c, path).is_err());
        }
        let mut changed = m.clone();
        changed.input_policy_sha256 = "f".repeat(64);
        assert!(local_policy(&p, &changed, &c, "/agents").is_err());
        changed = m.clone();
        changed.snapshot.init_pid = 0;
        assert!(local_policy(&p, &changed, &c, "/agents").is_err());
    }

    #[test]
    fn private_mapping_is_closed_and_requires_original_volume_fingerprint() {
        let (_, m, _) = fixture();
        let mut value = json!(m);
        value["endpoint"] = json!("http://arbitrary");
        assert!(serde_json::from_value::<ContainerMapping>(value).is_err());
        let mut value = json!(m);
        value.as_object_mut().unwrap().remove("volume_sha256");
        assert!(serde_json::from_value::<ContainerMapping>(value).is_err());
    }
}
