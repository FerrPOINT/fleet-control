//! Initial generation only. Unknown delivery can never acquire another create permit.
use super::*;
use app::container_runtime::{
    ContainerBinding, ContainerPreparationClaim, MappedContainer, PreparedContainer,
};
use container_control::{ContainerControl, ContainerLaunchFiles, ControlSource, canonical_hash};
use container_lifecycle::{UTILITY_SHA256, private_file, private_root};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::path::Path;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub(super) agent_id: Uuid,
    pub(super) generation: Uuid,
    pub(super) operation_id: Uuid,
    pub(super) paths: domain::AgentPaths,
    pub(super) api_port: Option<i32>,
    pub(super) configuration_revision: Option<i64>,
    pub(super) configuration_sha256: Option<String>,
    pub(super) local_policy: Value,
    pub(super) policy: Value,
    pub(super) process: Value,
    pub(super) source_sha256: [String; 3],
    pub(super) context: String,
    pub(super) mapped: Option<MappedContainer>,
    pub(super) files_sha256: std::collections::BTreeMap<String, String>,
}

fn held() -> AppError {
    AppError::Unavailable(
        "Original preparation requires readback; no duplicate namespace or credential rotation"
            .into(),
    )
}

impl Intent {
    pub(super) fn claim(&self) -> Result<ContainerPreparationClaim, AppError> {
        Ok(ContainerPreparationClaim {
            agent_id: self.agent_id,
            generation: self.generation,
            operation_id: self.operation_id,
            paths: self.paths.clone(),
            api_port: self.api_port,
            configuration_revision: self.configuration_revision,
            configuration_sha256: self.configuration_sha256.clone(),
            intent_sha256: canonical_hash(self)?,
        })
    }

    pub(super) fn files(&self, root: &Path) -> ContainerLaunchFiles {
        let name = format!("{}.{}", self.agent_id, self.generation);
        ContainerLaunchFiles {
            policy: self.policy.clone(),
            compose: root.join(format!("{name}.compose.json")),
            journal: root.join(format!("{name}.launch.sqlite")),
            stop_journal: root.join(format!("{name}.stop.sqlite")),
            mapped: self.mapped.clone(),
            recovery: None,
        }
    }
}

async fn immutable_intent(root: &Path, path: &Path, intent: &Intent) -> Result<(), AppError> {
    crate::reject_symlink_components(root, path)
        .await
        .map_err(|_| held())?;
    let bytes = serde_json::to_vec(intent).map_err(|_| held())?;
    if path.parent() != Some(root) || bytes.len() > 65_536 {
        return Err(held());
    }
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    // A racing writer or a partial crash file stays held; never overwrite it.
    let mut file = options.open(path).await.map_err(|_| held())?;
    file.write_all(&bytes).await.map_err(|_| held())?;
    file.sync_all().await.map_err(|_| held())?;
    tokio::fs::File::open(root)
        .await
        .map_err(|_| held())?
        .sync_all()
        .await
        .map_err(|_| held())
}

pub(super) fn mapped_policy(
    local: &Value,
    mapped: &MappedContainer,
    trusted: &shared::config::MappingControllerConfig,
    root: &str,
) -> Result<Value, AppError> {
    let mut policy = local.clone();
    policy["contract_version"] = json!(3);
    policy["mounts"] = json!(mapped.mapping.mounts.iter().map(|m| {
        let source = Path::new(&m.source);
        let agent = source.parent().and_then(Path::file_name).and_then(|s| s.to_str()).ok_or_else(held)?;
        let area = source.file_name().and_then(|s| s.to_str()).ok_or_else(held)?;
        Ok::<_, AppError>(json!({"type":"volume","source":mapped.mapping.volume_name,
            "subpath":format!("{agent}/{area}"),"destination":m.destination,"read_only":m.read_only}))
    }).collect::<Result<Vec<_>, _>>()?);
    if container_mapping::local_policy(&policy, &mapped.mapping, trusted, root)? != *local {
        return Err(held());
    }
    Ok(policy)
}

impl LocalRuntimeSupervisor {
    async fn preparation_inputs(
        &self,
        agent: &Agent,
    ) -> Result<std::collections::BTreeMap<String, String>, AppError> {
        let agents = Path::new(&self.config.fleet.agents_root);
        crate::reject_symlink_components(Path::new("/"), agents)
            .await
            .map_err(|_| held())?;
        let root = crate::safe_agent_root(agents, &agent.name).map_err(|_| held())?;
        if agent.name != format!("agent{}", agent.ordinal)
            || agent.ordinal < 1
            || crate::inspect_agent_marker(&root, agent)
                .await
                .map_err(|_| held())?
                != (true, true)
        {
            return Err(held());
        }
        for (area, path) in [
            ("runtime", &agent.paths.runtime),
            ("config", &agent.paths.config),
            ("workspace", &agent.paths.workspace),
            ("logs", &agent.paths.logs),
        ] {
            if Path::new(path) != root.join(area) {
                return Err(held());
            }
            crate::reject_symlink_components(agents, Path::new(path))
                .await
                .map_err(|_| held())?;
            if !tokio::fs::symlink_metadata(path)
                .await
                .map_err(|_| held())?
                .is_dir()
            {
                return Err(held());
            }
        }
        let revision = self.repo.get_container_configuration(agent.id).await?;
        if let Some(r) = &revision {
            // This observes existing effective files; it never activates or repairs them.
            crate::effective_configuration::verify(agent, &self.config, r)
                .await
                .map_err(|_| held())?;
        }
        let config = Path::new(&agent.paths.config);
        let mut hashes = std::collections::BTreeMap::new();
        for name in ["config.yaml", "SOUL.md", ".env"] {
            let bytes = private_file(config, &config.join(name), 65_536).await?;
            if revision.is_none()
                && name == ".env"
                && bytes != crate::provisioned_hermes_env(agent, &self.config)?.as_bytes()
            {
                return Err(held());
            }
            hashes.insert(name.into(), hex::encode(sha2::Sha256::digest(&bytes)));
        }
        hashes.insert(
            "agent-marker".into(),
            hex::encode(sha2::Sha256::digest(
                private_file(&root, &root.join(".fleet-agent.json"), 16_384).await?,
            )),
        );
        Ok(hashes)
    }

    async fn preparation_intent(
        &self,
        agent: &Agent,
        generation: Uuid,
        operation_id: Uuid,
    ) -> Result<Intent, AppError> {
        let control = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let recipe = control.provisioning.as_ref().ok_or_else(held)?;
        let port = agent
            .api_port
            .filter(|p| (1024..=65535).contains(p))
            .ok_or_else(held)?;
        let revision = self.repo.get_container_configuration(agent.id).await?;
        // Base validates all process limits, identifiers, immutable image and closed recipe
        // before effects. Fleet additionally restricts the project and all mount sources.
        if !matches!(recipe.project.as_str(), "sdlc1" | "sdlc2")
            && !recipe.project.starts_with("sdlc-qa-")
            || generation.is_nil()
            || operation_id.is_nil()
        {
            return Err(held());
        }
        let service = format!("{}-runtime-{}", agent.name, generation.simple());
        let local = json!({"contract_version":2,"project":recipe.project,"service":service,
            "resource_id":agent.id,"generation":generation,"image_id":recipe.image_id,
            "task":recipe.task,"purpose":recipe.purpose,
            "network":{"id":"0".repeat(64),"name":format!("{}-{service}",recipe.project),"internal":recipe.network_internal},
            "mounts":[
                {"type":"bind","source":agent.paths.runtime,"destination":"/runtime","read_only":true},
                {"type":"bind","source":agent.paths.config,"destination":"/config","read_only":false},
                {"type":"bind","source":agent.paths.workspace,"destination":"/workspace","read_only":false},
                {"type":"bind","source":agent.paths.logs,"destination":"/logs","read_only":false}]});
        Ok(Intent {
            agent_id: agent.id,
            generation,
            operation_id,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            configuration_revision: revision.as_ref().map(|r| r.revision),
            configuration_sha256: revision
                .as_ref()
                .map(|r| canonical_hash(&r.snapshot))
                .transpose()?,
            local_policy: local.clone(),
            policy: local,
            process: json!({"user":recipe.user,"entrypoint":recipe.entrypoint,
                "command":["serve","--host","0.0.0.0","--port",port.to_string()],"working_dir":"/workspace",
                "pids_limit":recipe.pids_limit,"memory_bytes":recipe.memory_bytes,"nano_cpus":recipe.nano_cpus,
                "environment":{"HOME":"/config","HERMES_HOME":"/config","HERMES_SERVE_HEADLESS":"1",
                    "API_SERVER_ENABLED":"true","API_SERVER_HOST":"0.0.0.0","API_SERVER_PORT":port.to_string(),
                    "API_SERVER_KEY":crate::agent_runtime_token(&self.config,agent.id)?,
                    "API_SERVER_CORS_ORIGINS":self.config.server.cors_allowed_origins.join(","),"PYTHONDONTWRITEBYTECODE":"1"}}),
            source_sha256: UTILITY_SHA256.map(str::to_owned),
            context: control.context.clone(),
            mapped: None,
            files_sha256: self.preparation_inputs(agent).await?,
        })
    }

    pub(super) async fn prepare_container(
        &self,
        agent: &Agent,
    ) -> Result<PreparedContainer, AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let root = private_root(&config.controller_root).await?;
        let agents =
            crate::normalize_path(Path::new(&self.config.fleet.agents_root)).map_err(|_| held())?;
        let source = crate::normalize_path(Path::new(&config.base_root)).map_err(|_| held())?;
        if root.starts_with(&agents)
            || agents.starts_with(&root)
            || source.starts_with(&agents)
            || agents.starts_with(&source)
            || root.starts_with(&source)
            || source.starts_with(&root)
        {
            return Err(held());
        }
        let control = ContainerControl::new(
            config.python.clone().into(),
            ControlSource {
                root: source,
                sha256: container_lifecycle::CONTROL_SHA256.map(str::to_owned),
            },
            config.context.clone(),
        )?;
        let path = root.join(format!("{}.container-intent.json", agent.id));
        let record = self.repo.get_container_preparation(agent.id).await?;
        let previous: Option<Intent> = match tokio::fs::symlink_metadata(&path).await {
            Ok(_) => Some(
                serde_json::from_slice(&private_file(&root, &path, 65_536).await?)
                    .map_err(|_| held())?,
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && record.is_none() => None,
            Err(_) => return Err(held()),
        };
        if previous.is_some() && record.is_none() {
            // A disk/DB mismatch is not proof that native delivery never happened.
            return Err(held());
        }
        let mut intent = self
            .preparation_intent(
                agent,
                previous
                    .as_ref()
                    .map_or_else(Uuid::new_v4, |i| i.generation),
                previous
                    .as_ref()
                    .map_or_else(Uuid::new_v4, |i| i.operation_id),
            )
            .await?;
        if let Some(trusted) = &config.mapping_controller {
            let mapping = if let Some(old) = &previous {
                // Never resolve a new mapping after unknown preparation.
                old.mapped.as_ref().ok_or_else(held)?.mapping.clone()
            } else {
                control
                    .resolve_mounts(
                        &intent.files(&root),
                        trusted,
                        &self.config.fleet.agents_root,
                    )
                    .await?
            };
            let name = format!("{}.{}", agent.id, intent.generation);
            let mapped = MappedContainer {
                mapping,
                mapping_file: root
                    .join(format!("{name}.mapping.json"))
                    .to_string_lossy()
                    .into_owned(),
                attachment_journal: root
                    .join(format!("{name}.attach.sqlite"))
                    .to_string_lossy()
                    .into_owned(),
                recovery_journal: root
                    .join(format!("{name}.recovery.sqlite"))
                    .to_string_lossy()
                    .into_owned(),
            };
            intent.policy = mapped_policy(
                &intent.local_policy,
                &mapped,
                trusted,
                &self.config.fleet.agents_root,
            )?;
            intent.mapped = Some(mapped);
        }
        if let Some(old) = previous {
            if canonical_hash(&old)? != canonical_hash(&intent)? {
                return Err(held());
            }
        } else {
            immutable_intent(&root, &path, &intent).await?;
        }
        let claim = intent.claim()?;
        let record = self.repo.claim_container_preparation(&claim).await?;
        if let Some(receipt) = record.receipt {
            self.checked_prepared(agent, &receipt).await?;
            return Ok(receipt);
        }
        let first = self
            .repo
            .claim_container_preparation_delivery(&claim)
            .await?;
        // Re-read the exact private bytes and inputs after the durable delivery CAS.
        let saved: Intent = serde_json::from_slice(&private_file(&root, &path, 65_536).await?)
            .map_err(|_| held())?;
        if canonical_hash(&saved)? != claim.intent_sha256
            || self.preparation_inputs(agent).await? != intent.files_sha256
        {
            return Err(held());
        }
        let files = intent.files(&root);
        let name = format!("{}.{}", agent.id, intent.generation);
        let receipt = control
            .preparation(
                &files,
                &intent.process,
                intent.operation_id,
                &root.join(format!("{name}.create.json")),
                &root.join(format!("{name}.create.sqlite")),
                first,
            )
            .await?;
        let prepared = PreparedContainer {
            agent_id: agent.id,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            configuration_revision: intent.configuration_revision,
            configuration_sha256: intent.configuration_sha256,
            container: ContainerBinding {
                registration: receipt.registration,
                policy: receipt.policy,
                compose: files.compose.to_string_lossy().into_owned(),
                journal: files.journal.to_string_lossy().into_owned(),
                stop_journal: files.stop_journal.to_string_lossy().into_owned(),
                source_sha256: intent.source_sha256,
                context: intent.context,
                mapped: intent.mapped,
            },
        };
        self.checked_prepared(agent, &prepared).await?;
        self.container_files(&prepared.container).await?;
        if self.preparation_inputs(agent).await? != intent.files_sha256 {
            return Err(held());
        }
        self.repo
            .acknowledge_container_preparation(&claim, &prepared)
            .await?;
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> Intent {
        serde_json::from_value(json!({"agent_id":Uuid::new_v4(),"generation":Uuid::new_v4(),"operation_id":Uuid::new_v4(),
            "paths":{"runtime":"/agents/agent1/runtime","config":"/agents/agent1/config","workspace":"/agents/agent1/workspace","logs":"/agents/agent1/logs"},
            "api_port":24003,"configuration_revision":1,"configuration_sha256":"a".repeat(64),"local_policy":{"image_id":"sha256:original"},
            "policy":{"image_id":"sha256:original"},"process":{"environment":{"API_SERVER_KEY":"original-secret"}},
            "source_sha256":UTILITY_SHA256,"context":"desktop-linux","mapped":null,"files_sha256":{".env":"b".repeat(64)}})).unwrap()
    }

    #[test]
    fn immutable_intent_hash_seals_credentials_config_recipe_and_generation() {
        let original = intent();
        let claim = original.claim().unwrap();
        assert!(
            !serde_json::to_string(&claim)
                .unwrap()
                .contains("original-secret")
        );
        for field in [
            "credential",
            "configuration",
            "image",
            "mapping",
            "generation",
            "local",
            "context",
            "file",
        ] {
            let mut changed = original.clone();
            match field {
                "credential" => changed.process["environment"]["API_SERVER_KEY"] = json!("rotated"),
                "configuration" => changed.configuration_revision = Some(2),
                "image" => changed.policy["image_id"] = json!("sha256:replacement"),
                "mapping" => {
                    changed.policy["mounts"] = json!([{"source":"foreign-volume"}]);
                }
                "generation" => changed.generation = Uuid::new_v4(),
                "local" => changed.local_policy["image_id"] = json!("foreign"),
                "context" => changed.context = "foreign".into(),
                _ => {
                    changed.files_sha256.insert(".env".into(), "foreign".into());
                }
            }
            assert_ne!(changed.claim().unwrap().intent_sha256, claim.intent_sha256);
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn intent_is_private_durable_and_never_overwritten_after_crash() {
        let root =
            std::env::temp_dir().join(format!("fleet-preparation-intent-{}", Uuid::new_v4()));
        tokio::fs::create_dir(&root).await.unwrap();
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        let root = private_root(root.to_str().unwrap()).await.unwrap();
        let path = root.join("intent.json");
        let original = intent();
        immutable_intent(&root, &path, &original).await.unwrap();
        let bytes = private_file(&root, &path, 65_536).await.unwrap();
        assert_eq!(
            canonical_hash(&serde_json::from_slice::<Intent>(&bytes).unwrap()).unwrap(),
            original.claim().unwrap().intent_sha256
        );
        assert!(immutable_intent(&root, &path, &intent()).await.is_err());
        assert_eq!(private_file(&root, &path, 65_536).await.unwrap(), bytes);
        let partial = root.join("partial.json");
        tokio::fs::write(&partial, b"{").await.unwrap();
        assert!(immutable_intent(&root, &partial, &original).await.is_err());
        assert_eq!(tokio::fs::read(&partial).await.unwrap(), b"{");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
