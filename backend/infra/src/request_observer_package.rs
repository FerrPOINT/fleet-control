//! Exact immutable producer objects, not working-tree plugin discovery.
use domain::UpdateAgentConfigRequest;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::AppError;
use std::{collections::BTreeMap, path::Path, time::Duration};

pub(crate) const KEY: &str = "fleet_request_observer";
pub(crate) const PLUGIN: &str = "fleet-hermes-request-observer";
pub(crate) const COMMIT: &str = "43b365fd97e8955821312cbc2e68b4d83e5bd240";
const REPOSITORY: &str = "https://github.com/FerrPOINT/services-base.git";
pub(crate) const FILES: [(&str, &str); 4] = [
    (
        "__init__.py",
        "6a5ea8b412a75dcf3803843be89409c931471c07188370e06a684443d44789a3",
    ),
    (
        "plugin.py",
        "1e21d8c0e6fcac136b966b09d4e5c433bc9ff1e4ed3be6cb667fee9718d3f6de",
    ),
    (
        "store.py",
        "c33cd12cf59d6c9bf1d4e2aade98743d56de4d4e9a924df00e7a0cb302f20170",
    ),
    (
        "plugin.yaml",
        "ed39d83e75c2ecfeab0182beb1eee1e7ce69623680fe3d4b6b76240a13ba985a",
    ),
];

fn invalid() -> AppError {
    AppError::validation("pinned request observer configuration is unavailable or inconsistent")
}

fn proof(enabled: bool) -> Value {
    json!({"enabled": enabled, "repository": REPOSITORY, "revision": COMMIT,
        "sha256": FILES.into_iter().collect::<BTreeMap<_, _>>()})
}

pub(crate) fn enabled(config: &Value, renderer: u32) -> Result<Option<bool>, AppError> {
    let Some(value) = config.get(KEY) else {
        return if renderer == 3 {
            Err(invalid())
        } else {
            Ok(None)
        };
    };
    let enabled = value
        .get("enabled")
        .and_then(Value::as_bool)
        .ok_or_else(invalid)?;
    if renderer != 3 || value != &proof(enabled) {
        return Err(invalid());
    }
    let names = config
        .pointer("/plugins/enabled")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if names.len() > 64
        || names.iter().any(|name| name.as_str().is_none())
        || names
            .iter()
            .filter(|name| name.as_str() == Some(PLUGIN))
            .count()
            != if enabled { 1 } else { 0 }
    {
        return Err(invalid());
    }
    Ok(Some(enabled))
}

pub(crate) async fn read(checkout: &Path) -> Result<BTreeMap<String, String>, AppError> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let origin =
            crate::base_package::git(checkout, &["config", "--get", "remote.origin.url"]).await?;
        if !matches!(
            std::str::from_utf8(&origin).map_err(|_| invalid())?.trim(),
            REPOSITORY | "git@github.com:FerrPOINT/services-base.git"
        ) || crate::base_package::git(checkout, &["cat-file", "-t", COMMIT]).await?
            != b"commit\n"
        {
            return Err(invalid());
        }
        let paths: Vec<_> = FILES
            .iter()
            .map(|(name, _)| format!("deploy/hermes-request-observer/{name}"))
            .collect();
        let references: Vec<_> = paths.iter().map(String::as_str).collect();
        let objects =
            crate::base_package::pinned_blobs(checkout, COMMIT, &references, FILES.len()).await?;
        let mut files = BTreeMap::new();
        for (name, expected) in FILES {
            let bytes = objects
                .get(&format!("deploy/hermes-request-observer/{name}"))
                .ok_or_else(invalid)?;
            if hex::encode(Sha256::digest(bytes)) != expected {
                return Err(invalid());
            }
            files.insert(
                name.to_owned(),
                String::from_utf8(bytes.clone()).map_err(|_| invalid())?,
            );
        }
        Ok(files)
    })
    .await
    .map_err(|_| invalid())?
}

pub(crate) async fn prepare(
    checkout: &Path,
    mut request: UpdateAgentConfigRequest,
) -> Result<UpdateAgentConfigRequest, AppError> {
    if let Some(value) = request.config_json.get(KEY) {
        let enabled = value
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(invalid)?;
        if value != &json!({"enabled": enabled}) && value != &proof(enabled) {
            return Err(invalid());
        }
        read(checkout).await?;
        request.config_json[KEY] = proof(enabled);
        let plugins = request
            .config_json
            .as_object_mut()
            .ok_or_else(invalid)?
            .entry("plugins")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(invalid)?;
        let names = plugins
            .entry("enabled")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(invalid)?;
        if names.iter().any(|name| name.as_str().is_none()) {
            return Err(invalid());
        }
        names.retain(|name| name.as_str() != Some(PLUGIN));
        if enabled {
            names.push(json!(PLUGIN));
        }
        self::enabled(&request.config_json, 3)?;
    }
    Ok(request)
}

pub(crate) fn observer_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| FILES.iter().any(|(known, _)| *known == name))
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == PLUGIN)
        && path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|name| name == "plugins")
}

pub(crate) fn absent(path: &Path, body: &str) -> bool {
    body.is_empty()
        && (path.file_name().is_some_and(|name| name == "SKILL.md") || observer_path(path))
}

pub(crate) async fn verify_inventory(config_root: &Path) -> Result<(), AppError> {
    let path = config_root.join("plugins").join(PLUGIN);
    let mut entries = match tokio::fs::read_dir(&path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(invalid()),
    };
    let mut count = 0;
    while let Some(entry) = entries.next_entry().await.map_err(|_| invalid())? {
        count += 1;
        if count > FILES.len() || !observer_path(&entry.path()) {
            return Err(invalid());
        }
        let metadata = tokio::fs::symlink_metadata(entry.path())
            .await
            .map_err(|_| invalid())?;
        if !metadata.is_file() || metadata.len() > 262_144 {
            return Err(invalid());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 1 {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_is_fixed_and_requires_explicit_new_renderer() {
        for on in [false, true] {
            let mut config = json!({"fleet_request_observer": proof(on),
                "plugins":{"enabled": if on { vec![PLUGIN] } else { vec![] }}});
            assert_eq!(enabled(&config, 3).unwrap(), Some(on));
            for version in [1, 2] {
                assert!(enabled(&config, version).is_err());
            }
            config[KEY]["revision"] = json!("HEAD");
            assert!(enabled(&config, 3).is_err());
        }
        assert!(enabled(&json!({}), 3).is_err());
        assert_eq!(enabled(&json!({}), 2).unwrap(), None);
        let good = json!({"fleet_request_observer": proof(true), "plugins":{"enabled":[PLUGIN]}});
        for (pointer, value) in [
            (
                "/fleet_request_observer/sha256/store.py",
                json!("a".repeat(64)),
            ),
            (
                "/fleet_request_observer/repository",
                json!("https://example.test/base.git"),
            ),
            ("/plugins/enabled", json!([])),
            ("/plugins/enabled", json!([PLUGIN, PLUGIN])),
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(enabled(&bad, 3).is_err());
        }
    }

    #[tokio::test]
    async fn foreign_inventory_is_preserved_and_denied() {
        let root =
            std::env::temp_dir().join(format!("fleet-observer-inventory-{}", uuid::Uuid::new_v4()));
        let directory = root.join("plugins").join(PLUGIN);
        tokio::fs::create_dir_all(&directory).await.unwrap();
        for (name, _) in FILES {
            tokio::fs::write(directory.join(name), b"fixture")
                .await
                .unwrap();
        }
        verify_inventory(&root).await.unwrap();
        let extra = directory.join("foreign.py");
        tokio::fs::write(&extra, b"preserve").await.unwrap();
        assert!(verify_inventory(&root).await.is_err());
        assert_eq!(tokio::fs::read(&extra).await.unwrap(), b"preserve");
        tokio::fs::remove_file(extra).await.unwrap();
        tokio::fs::remove_file(directory.join("plugin.py"))
            .await
            .unwrap();
        tokio::fs::create_dir(directory.join("plugin.py"))
            .await
            .unwrap();
        assert!(verify_inventory(&root).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[test]
    fn absence_is_limited_to_managed_instruction_or_exact_observer_files() {
        assert!(absent(
            Path::new("agent1/config/plugins/fleet-hermes-request-observer/store.py"),
            ""
        ));
        assert!(!absent(
            Path::new("agent1/config/plugins/foreign/store.py"),
            ""
        ));
        assert!(!absent(
            Path::new("agent1/config/plugins/fleet-hermes-request-observer/extra.py"),
            ""
        ));
        assert!(!absent(Path::new("agent1/config/.env"), ""));
    }
}
