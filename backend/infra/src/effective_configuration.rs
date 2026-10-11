use crate::{configuration_files, reject_symlink_components};
use domain::{Agent, AgentConfigRevision, AgentKind};
use shared::{AppConfig, AppError};
use std::{collections::BTreeSet, io::ErrorKind, path::Path};
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
    if revision
        .snapshot
        .config
        .config_json
        .get("fleet_sdlc_package")
        .is_some()
    {
        let role = agent.sdlc_role.ok_or_else(mismatch)?;
        if config.fleet.base_package_checkout.is_empty() {
            return Err(mismatch());
        }
        let package = crate::base_package::VerifiedRolePackage::read(
            Path::new(&config.fleet.base_package_checkout),
            role,
        )
        .await
        .map_err(|_| mismatch())?;
        package
            .verify_snapshot(agent, &revision.snapshot)
            .map_err(|_| mismatch())?;
        let expected = package.proof().skill_sha256.keys().cloned().collect();
        verify_skill_files(
            root,
            &Path::new(&agent.paths.config).join("skills"),
            &expected,
        )
        .await?;
    }
    // This attests HOME files, not plugin/project/external discovery, loaded
    // model/tool settings, or a frozen assignment. Native admission stays separate.
    Ok(())
}

/// Do not trust a bundled manifest or a flat DB inventory to exclude extra skills.
async fn verify_skill_files(
    agents_root: &Path,
    skills_root: &Path,
    expected: &BTreeSet<String>,
) -> Result<(), AppError> {
    let mut pending = vec![(skills_root.to_path_buf(), 0usize)];
    let mut seen = BTreeSet::new();
    let mut entries = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        reject_symlink_components(agents_root, &directory)
            .await
            .map_err(|_| mismatch())?;
        if !tokio::fs::symlink_metadata(&directory)
            .await
            .map_err(|_| mismatch())?
            .is_dir()
        {
            return Err(mismatch());
        }
        let mut contents = tokio::fs::read_dir(&directory)
            .await
            .map_err(|_| mismatch())?;
        while let Some(entry) = contents.next_entry().await.map_err(|_| mismatch())? {
            entries += 1;
            if entries > 4096 {
                return Err(mismatch());
            }
            let path = entry.path();
            reject_symlink_components(agents_root, &path)
                .await
                .map_err(|_| mismatch())?;
            let metadata = tokio::fs::symlink_metadata(&path)
                .await
                .map_err(|_| mismatch())?;
            if metadata.is_dir() {
                if depth >= 16 {
                    return Err(mismatch());
                }
                pending.push((path, depth + 1));
            } else if !metadata.is_file() || entry.file_name() != "SKILL.md" {
                return Err(mismatch());
            } else {
                // The pinned package materializes SKILL.md only. Unattested
                // support/scripts are callable through native skill_view too.
                let relative = path.strip_prefix(skills_root).map_err(|_| mismatch())?;
                let components: Vec<_> = relative.components().collect();
                if components.len() != 2 {
                    return Err(mismatch());
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.nlink() != 1 {
                        return Err(mismatch());
                    }
                }
                let name = components[0].as_os_str().to_str().ok_or_else(mismatch)?;
                if !expected.contains(name) || !seen.insert(name.to_string()) {
                    return Err(mismatch());
                }
            }
        }
    }
    if seen != *expected {
        return Err(mismatch());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture() -> (std::path::PathBuf, std::path::PathBuf, BTreeSet<String>) {
        let root =
            std::env::temp_dir().join(format!("fleet-skill-readback-{}", uuid::Uuid::new_v4()));
        let skills = root.join("agent1/config/skills");
        tokio::fs::create_dir_all(skills.join("allowed"))
            .await
            .unwrap();
        tokio::fs::write(skills.join("allowed/SKILL.md"), "# allowed\n")
            .await
            .unwrap();
        (root, skills, BTreeSet::from(["allowed".into()]))
    }

    #[tokio::test]
    async fn base_package_skill_readback_excludes_unlisted_and_nested_files() {
        let (root, skills, expected) = fixture().await;
        verify_skill_files(&root, &skills, &expected).await.unwrap();
        for relative in [
            "bundled/SKILL.md",
            "category/allowed/SKILL.md",
            "allowed/skill.md",
            "allowed.md",
            "allowed.MD",
            "unlisted.md",
            "category/allowed.md",
            "allowed/references/guide.md",
            "allowed/scripts/helper.py",
            "allowed/templates/config.yaml",
            "allowed/assets/fixture.bin",
            "allowed/extra.txt",
            "allowed/.hidden",
            ".metadata.json",
            "foreign/notes.txt",
        ] {
            let path = skills.join(relative);
            tokio::fs::create_dir_all(path.parent().unwrap())
                .await
                .unwrap();
            tokio::fs::write(&path, "unexpected skill").await.unwrap();
            assert!(verify_skill_files(&root, &skills, &expected).await.is_err());
            assert_eq!(
                tokio::fs::read_to_string(&path).await.unwrap(),
                "unexpected skill"
            );
            tokio::fs::remove_file(path).await.unwrap();
        }
        verify_skill_files(&root, &skills, &expected).await.unwrap();
        tokio::fs::remove_file(skills.join("allowed/SKILL.md"))
            .await
            .unwrap();
        assert!(verify_skill_files(&root, &skills, &expected).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn base_package_skill_readback_rejects_hardlinked_instruction_without_mutation() {
        let (root, skills, expected) = fixture().await;
        let instruction = skills.join("allowed/SKILL.md");
        let alias = root.join("outside-skill.md");
        tokio::fs::hard_link(&instruction, &alias).await.unwrap();
        assert!(verify_skill_files(&root, &skills, &expected).await.is_err());
        for path in [&instruction, &alias] {
            assert_eq!(
                tokio::fs::read_to_string(path).await.unwrap(),
                "# allowed\n"
            );
        }
        tokio::fs::remove_file(alias).await.unwrap();
        verify_skill_files(&root, &skills, &expected).await.unwrap();
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn base_package_skill_readback_bounds_directory_depth() {
        let (root, skills, expected) = fixture().await;
        let mut deep = skills.clone();
        for _ in 0..17 {
            deep.push("nested");
        }
        tokio::fs::create_dir_all(&deep).await.unwrap();
        assert!(verify_skill_files(&root, &skills, &expected).await.is_err());
        assert!(deep.exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn base_package_skill_readback_rejects_linked_and_special_entries() {
        let (root, skills, expected) = fixture().await;
        let outside = root.join("outside");
        tokio::fs::create_dir_all(&outside).await.unwrap();
        let link = skills.join("hidden-link");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(verify_skill_files(&root, &skills, &expected).await.is_err());
        tokio::fs::remove_file(link).await.unwrap();
        let socket = skills.join("socket");
        let _listener = std::os::unix::net::UnixListener::bind(socket).unwrap();
        assert!(verify_skill_files(&root, &skills, &expected).await.is_err());
        assert!(outside.exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
