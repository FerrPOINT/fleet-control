use crate::{configuration_files, reject_symlink_components};
use domain::{Agent, AgentConfigRevision, AgentKind};
use shared::{AppConfig, AppError};
use std::{collections::HashSet, io::ErrorKind, path::Path};
use tokio::io::AsyncReadExt;

fn mismatch() -> AppError {
    AppError::conflict("effective configuration readback failed")
}

/// Compare against the persisted snapshot, never against paths/hashes supplied by a marker.
/// This is a fresh filesystem observation, not a runtime lease or an admission receipt.
pub(crate) async fn verify(
    agent: &Agent,
    config: &AppConfig,
    revision: &AgentConfigRevision,
) -> Result<(), AppError> {
    if agent.kind != AgentKind::Hermes
        || revision.agent_id != agent.id
        || revision.revision < 1
        || revision.state != "active"
        || !revision.is_effective
        || revision.draining
        || !revision.validation_errors.is_empty()
    {
        return Err(mismatch());
    }
    let root = Path::new(&config.fleet.agents_root);
    let agent_root = crate::safe_agent_root(root, &agent.name).map_err(|_| mismatch())?;
    let marker = agent_root.join(".fleet-agent.json");
    reject_symlink_components(root, &marker)
        .await
        .map_err(|_| mismatch())?;
    let metadata = tokio::fs::symlink_metadata(&marker)
        .await
        .map_err(|_| mismatch())?;
    if !metadata.is_file() || metadata.len() > 16_384 {
        return Err(mismatch());
    }
    let files = configuration_files(agent, config, revision)
        .await
        .map_err(|_| mismatch())?;
    let workspace = Path::new(&agent.paths.workspace);
    let expected_workspace = root.join(&agent.name).join("workspace");
    if crate::normalize_path(workspace).map_err(|_| mismatch())?
        != crate::normalize_path(&expected_workspace).map_err(|_| mismatch())?
    {
        return Err(mismatch());
    }
    reject_symlink_components(root, workspace)
        .await
        .map_err(|_| mismatch())?;
    if !tokio::fs::symlink_metadata(workspace)
        .await
        .map_err(|_| mismatch())?
        .is_dir()
    {
        return Err(mismatch());
    }
    for (path, expected) in files {
        reject_symlink_components(root, &path)
            .await
            .map_err(|_| mismatch())?;
        let metadata = match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(_) => return Err(mismatch()),
        };
        if expected.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md") {
            if metadata.is_some() {
                return Err(mismatch());
            }
            continue;
        }
        let metadata = metadata.ok_or_else(mismatch)?;
        if !metadata.is_file() || metadata.len() != expected.len() as u64 {
            return Err(mismatch());
        }
        // A changed/oversized file cannot cause unbounded allocation during readiness.
        let file = tokio::fs::File::open(&path).await.map_err(|_| mismatch())?;
        let mut actual = Vec::new();
        file.take(expected.len() as u64 + 1)
            .read_to_end(&mut actual)
            .await
            .map_err(|_| mismatch())?;
        if actual != expected.as_bytes() {
            return Err(mismatch());
        }
    }
    let skills_root = Path::new(&agent.paths.config).join("skills");
    reject_symlink_components(root, &skills_root)
        .await
        .map_err(|_| mismatch())?;
    let known: HashSet<_> = revision
        .snapshot
        .skills
        .iter()
        .map(|skill| skill.name.as_str())
        .collect();
    let mut entries = match tokio::fs::read_dir(&skills_root).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(mismatch()),
    };
    while let Some(entry) = entries.next_entry().await.map_err(|_| mismatch())? {
        let name = entry.file_name();
        if !name.to_str().is_some_and(|name| known.contains(name)) {
            return Err(mismatch());
        }
        reject_symlink_components(root, &entry.path())
            .await
            .map_err(|_| mismatch())?;
        if !entry.file_type().await.map_err(|_| mismatch())?.is_dir() {
            return Err(mismatch());
        }
    }
    Ok(())
}
