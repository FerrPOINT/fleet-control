//! Read-only pinned Base consumption. Preparation never activates files or starts a model.
use domain::{
    Agent, AgentConfigurationSnapshot, AgentKind, AgentSkill, AgentStatus, SdlcRole, SkillState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::AppError;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use uuid::Uuid;

pub const BASE_PACKAGE_COMMIT: &str = "4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58";
#[cfg(test)]
#[path = "base_package_tests.rs"]
mod tests;
const REPOSITORY: &str = "https://github.com/FerrPOINT/services-base.git";
const MAX_BLOB_BYTES: usize = 262_144;
const ROLES: [&str; 7] = [
    "project_manager",
    "analyst",
    "architect",
    "developer",
    "reviewer",
    "tester",
    "devops",
];

fn invalid() -> AppError {
    AppError::validation("pinned Base package is unavailable or inconsistent")
}

fn role_key(role: SdlcRole) -> &'static str {
    // Preserve the legacy Fleet API spelling; the Base/Workflow role is devops.
    if role == SdlcRole::DevOps {
        "devops"
    } else {
        role.as_str()
    }
}

fn modes(role: &str) -> &[&str] {
    match role {
        "project_manager" => &["draft"],
        "analyst" => &["analysis"],
        "architect" => &["decomposition"],
        "developer" => &["initial", "rework"],
        _ => &["delivery", "integration"],
    }
}

fn profile(role: &str) -> String {
    let suffix = match role {
        "project_manager" => "project-manager",
        "tester" => "quality",
        "devops" => "operations",
        value => value,
    };
    format!("hermes-sdlc-{suffix}")
}

fn namespace(role: &str) -> String {
    format!(
        "hermes-{}",
        if role == "project_manager" {
            "project-manager"
        } else {
            role
        }
    )
}

fn digest(bytes: &[u8]) -> Result<String, AppError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    Ok(hex::encode(Sha256::digest(
        text.replace("\r\n", "\n").as_bytes(),
    )))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && is_hex(value)
}

fn is_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Manifest {
    schema: String,
    status: String,
    catalog_authority: Authority,
    sources: Sources,
    provenance: Provenance,
    roles: BTreeMap<String, Role>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Authority {
    purpose: String,
    selection_authority: String,
    owns_routing: bool,
    owns_mode_selection: bool,
    owns_workspace_selection: bool,
    owns_priority_selection: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sources {
    native: Source,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Source {
    repository: String,
    revision: String,
    revision_meaning: String,
    hash_algorithm: String,
    skills: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Provenance {
    adaptation: String,
    donor_skills_revision: String,
    skills_hub_revision: String,
    runtime_dependency_on_donor: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Role {
    namespace: String,
    profile: String,
    modes: Vec<String>,
    physical_skills: Vec<String>,
    role_instruction: Instruction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Instruction {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageProof {
    pub schema: &'static str,
    pub repository: &'static str,
    pub commit: &'static str,
    pub manifest_sha256: String,
    pub role: String,
    pub namespace: String,
    pub profile: String,
    pub modes: Vec<String>,
    pub role_instruction_sha256: String,
    pub skill_sha256: BTreeMap<String, String>,
}

/// Private content is intentionally neither Debug nor Serialize. Only proof metadata is public.
pub struct VerifiedRolePackage {
    proof: PackageProof,
    instruction: String,
    skills: BTreeMap<String, String>,
}

fn git(checkout: &Path, args: &[&str]) -> Result<Vec<u8>, AppError> {
    let output = Command::new("git")
        .arg("--no-replace-objects")
        .arg("-c")
        .arg("core.fsmonitor=false")
        .args(args)
        .current_dir(checkout)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "")
        .output()
        .map_err(|_| invalid())?;
    if !output.status.success() || output.stdout.len() > MAX_BLOB_BYTES {
        return Err(invalid());
    }
    Ok(output.stdout)
}

fn blob(checkout: &Path, path: &str) -> Result<Vec<u8>, AppError> {
    let tree = git(
        checkout,
        &["ls-tree", "-z", BASE_PACKAGE_COMMIT, "--", path],
    )?;
    let tree = std::str::from_utf8(&tree).map_err(|_| invalid())?;
    let (metadata, actual_path) = tree
        .strip_suffix('\0')
        .and_then(|line| line.split_once('\t'))
        .ok_or_else(invalid)?;
    let fields: Vec<_> = metadata.split(' ').collect();
    if actual_path != path
        || fields.len() != 3
        || !matches!(fields[0], "100644" | "100755")
        || fields[1] != "blob"
    {
        return Err(invalid());
    }
    let size = git(checkout, &["cat-file", "-s", fields[2]])?;
    let size = std::str::from_utf8(&size)
        .map_err(|_| invalid())?
        .trim()
        .parse::<usize>()
        .map_err(|_| invalid())?;
    if size == 0 || size > MAX_BLOB_BYTES {
        return Err(invalid());
    }
    let bytes = git(checkout, &["cat-file", "blob", fields[2]])?;
    if bytes.len() != size {
        return Err(invalid());
    }
    Ok(bytes)
}

fn package_blobs(checkout: &Path) -> Result<BTreeMap<String, Vec<u8>>, AppError> {
    let inventory = git(
        checkout,
        &[
            "ls-tree",
            "-r",
            "-l",
            "-z",
            BASE_PACKAGE_COMMIT,
            "--",
            "agent-skills/roles",
            "agent-skills/skills",
        ],
    )?;
    let mut entries = Vec::new();
    for record in std::str::from_utf8(&inventory)
        .map_err(|_| invalid())?
        .split_terminator('\0')
    {
        let (metadata, path) = record.split_once('\t').ok_or_else(invalid)?;
        let fields: Vec<_> = metadata.split_whitespace().collect();
        if fields.len() != 4
            || !matches!(fields[0], "100644" | "100755")
            || fields[1] != "blob"
            || fields[2].len() != 40
            || !is_hex(fields[2])
        {
            return Err(invalid());
        }
        let size: usize = fields[3].parse().map_err(|_| invalid())?;
        if size == 0 || size > MAX_BLOB_BYTES {
            return Err(invalid());
        }
        entries.push((path.to_owned(), fields[2].to_owned(), size));
    }
    if entries.len() != 21 {
        return Err(invalid());
    }
    // Git's bounded binary batch protocol avoids one process per private blob.
    let mut child = Command::new("git")
        .args([
            "--no-replace-objects",
            "-c",
            "core.fsmonitor=false",
            "cat-file",
            "--batch",
        ])
        .current_dir(checkout)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ALLOW_PROTOCOL", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| invalid())?;
    let input = entries
        .iter()
        .map(|(_, hash, _)| format!("{hash}\n"))
        .collect::<String>();
    let write = child
        .stdin
        .take()
        .ok_or_else(invalid)?
        .write_all(input.as_bytes());
    if write.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(invalid());
    }
    let output = child.wait_with_output().map_err(|_| invalid())?;
    if !output.status.success() || output.stdout.len() > 21 * (MAX_BLOB_BYTES + 100) {
        return Err(invalid());
    }
    decode_batch(&entries, &output.stdout)
}

fn decode_batch(
    entries: &[(String, String, usize)],
    mut bytes: &[u8],
) -> Result<BTreeMap<String, Vec<u8>>, AppError> {
    let mut files = BTreeMap::new();
    for (path, hash, size) in entries {
        let newline = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(invalid)?;
        if bytes[..newline] != *format!("{hash} blob {size}").as_bytes() {
            return Err(invalid());
        }
        bytes = &bytes[newline + 1..];
        if bytes.len() <= *size || bytes[*size] != b'\n' {
            return Err(invalid());
        }
        if files
            .insert(path.clone(), bytes[..*size].to_vec())
            .is_some()
        {
            return Err(invalid());
        }
        bytes = &bytes[*size + 1..];
    }
    if !bytes.is_empty() {
        return Err(invalid());
    }
    Ok(files)
}

impl VerifiedRolePackage {
    /// No network, HEAD, worktree files, donor access or fallback. The operator supplies a checkout.
    pub async fn read(checkout: &Path, role: SdlcRole) -> Result<Self, AppError> {
        let checkout = checkout.to_owned();
        tokio::task::spawn_blocking(move || {
            let origin = git(&checkout, &["config", "--get", "remote.origin.url"])?;
            let origin = std::str::from_utf8(&origin).map_err(|_| invalid())?.trim();
            if !matches!(
                origin,
                REPOSITORY | "git@github.com:FerrPOINT/services-base.git"
            ) {
                return Err(invalid());
            }
            if git(&checkout, &["cat-file", "-t", BASE_PACKAGE_COMMIT])? != b"commit\n" {
                return Err(invalid());
            }
            let manifest = blob(&checkout, "agent-skills/manifest.json")?;
            let files = package_blobs(&checkout)?;
            Self::verify(&manifest, files.keys().cloned().collect(), role, |path| {
                files.get(path).cloned().ok_or_else(invalid)
            })
        })
        .await
        .map_err(|_| invalid())?
    }

    fn verify(
        manifest: &[u8],
        inventory: BTreeSet<String>,
        role: SdlcRole,
        read: impl Fn(&str) -> Result<Vec<u8>, AppError>,
    ) -> Result<Self, AppError> {
        let parsed: Manifest = serde_json::from_slice(manifest).map_err(|_| invalid())?;
        let authority = &parsed.catalog_authority;
        let source = &parsed.sources.native;
        let provenance = &parsed.provenance;
        if parsed.schema != "base-hermes-role-skills/v1"
            || parsed.status != "candidate-not-installed"
            || authority.purpose != "physical-skill-and-profile-validation-only"
            || authority.selection_authority != "task-tracker-backend-assignment"
            || authority.owns_routing
            || authority.owns_mode_selection
            || authority.owns_workspace_selection
            || authority.owns_priority_selection
            || source.repository != REPOSITORY
            || source.revision != "SELF"
            || source.revision_meaning.is_empty()
            || source.hash_algorithm != "sha256-normalized-lf-utf8"
            || source.skills.len() != 14
            || provenance.runtime_dependency_on_donor
            || provenance.adaptation.is_empty()
            || provenance.donor_skills_revision.len() != 40
            || !is_hex(&provenance.donor_skills_revision)
            || provenance.skills_hub_revision.len() != 40
            || !is_hex(&provenance.skills_hub_revision)
            || parsed
                .roles
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != ROLES.into_iter().collect()
        {
            return Err(invalid());
        }
        let mut expected = BTreeSet::new();
        let mut used = BTreeSet::new();
        let mut instructions = BTreeMap::new();
        let mut skills = BTreeMap::new();
        for (key, definition) in &parsed.roles {
            let unique: BTreeSet<_> = definition.physical_skills.iter().cloned().collect();
            if definition.namespace != namespace(key)
                || definition.profile != profile(key)
                || definition
                    .modes
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    != modes(key)
                || unique.len() != definition.physical_skills.len()
                || !unique.contains("project-workflow-executor")
                || definition.role_instruction.path != format!("roles/{key}.md")
                || !is_digest(&definition.role_instruction.sha256)
            {
                return Err(invalid());
            }
            let path = format!("agent-skills/{}", definition.role_instruction.path);
            let text = read(&path)?;
            if digest(&text)? != definition.role_instruction.sha256 {
                return Err(invalid());
            }
            instructions.insert(
                key.clone(),
                String::from_utf8(text)
                    .map_err(|_| invalid())?
                    .replace("\r\n", "\n"),
            );
            expected.insert(path);
            for skill in unique {
                if !source.skills.contains_key(&skill) {
                    return Err(invalid());
                }
                used.insert(skill);
            }
        }
        if used != source.skills.keys().cloned().collect() {
            return Err(invalid());
        }
        for (name, expected_hash) in &source.skills {
            if name.is_empty()
                || !name.bytes().all(|c| c.is_ascii_lowercase() || c == b'-')
                || !is_digest(expected_hash)
            {
                return Err(invalid());
            }
            let path = format!("agent-skills/skills/{name}/SKILL.md");
            let bytes = read(&path)?;
            if digest(&bytes)? != *expected_hash {
                return Err(invalid());
            }
            let text = String::from_utf8(bytes)
                .map_err(|_| invalid())?
                .replace("\r\n", "\n");
            if !text.starts_with(&format!("---\nname: {name}\ndescription: ")) {
                return Err(invalid());
            }
            skills.insert(name.clone(), text);
            expected.insert(path);
        }
        if inventory != expected {
            return Err(invalid());
        }
        let key = role_key(role);
        let definition = parsed.roles.get(key).ok_or_else(invalid)?;
        let skills: BTreeMap<_, _> = skills
            .into_iter()
            .filter(|(name, _)| definition.physical_skills.contains(name))
            .collect();
        Ok(Self {
            proof: PackageProof {
                schema: "base-sdlc/package-proof/v1",
                repository: REPOSITORY,
                commit: BASE_PACKAGE_COMMIT,
                manifest_sha256: digest(manifest)?,
                role: key.to_owned(),
                namespace: definition.namespace.clone(),
                profile: definition.profile.clone(),
                modes: definition.modes.clone(),
                role_instruction_sha256: definition.role_instruction.sha256.clone(),
                skill_sha256: definition
                    .physical_skills
                    .iter()
                    .map(|name| (name.clone(), source.skills[name].clone()))
                    .collect(),
            },
            instruction: instructions.remove(key).ok_or_else(invalid)?,
            skills,
        })
    }

    pub fn proof(&self) -> &PackageProof {
        &self.proof
    }

    /// A client-editable proof field is not evidence: compare the complete draft to Git.
    pub fn verify_snapshot(
        &self,
        agent: &Agent,
        snapshot: &AgentConfigurationSnapshot,
    ) -> Result<(), AppError> {
        if agent.kind != AgentKind::Hermes
            || agent.status == AgentStatus::Archived
            || agent
                .sdlc_role
                .is_none_or(|role| role_key(role) != self.proof.role)
            || agent.namespace_id.as_deref() != Some(self.proof.namespace.as_str())
            || snapshot.config.config_json.get("fleet_sdlc_package")
                != Some(&serde_json::to_value(&self.proof).map_err(|_| invalid())?)
            || digest(snapshot.config.soul_md.as_bytes())? != self.proof.role_instruction_sha256
        {
            return Err(invalid());
        }
        let names: BTreeSet<_> = snapshot
            .skills
            .iter()
            .map(|skill| skill.name.as_str())
            .collect();
        if names.len() != snapshot.skills.len()
            || snapshot
                .skills
                .iter()
                .any(|skill| skill.agent_id != agent.id)
        {
            return Err(invalid());
        }
        let enabled: BTreeSet<_> = snapshot
            .skills
            .iter()
            .filter(|skill| skill.state == SkillState::Enabled)
            .map(|skill| skill.name.clone())
            .collect();
        if enabled != self.skills.keys().cloned().collect() {
            return Err(invalid());
        }
        for skill in &snapshot.skills {
            if skill.state == SkillState::Enabled
                && (skill.source != format!("base-sdlc:{BASE_PACKAGE_COMMIT}")
                    || digest(skill.content.as_deref().ok_or_else(invalid)?.as_bytes())?
                        != self.proof.skill_sha256[&skill.name])
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    /// Derive a draft for the existing config lifecycle; never mutate an active revision.
    pub fn prepare_snapshot(
        &self,
        agent: &Agent,
        mut snapshot: AgentConfigurationSnapshot,
    ) -> Result<AgentConfigurationSnapshot, AppError> {
        if agent.kind != AgentKind::Hermes
            || agent.status == AgentStatus::Archived
            || agent
                .sdlc_role
                .is_none_or(|role| role_key(role) != self.proof.role)
            || agent.namespace_id.as_deref() != Some(self.proof.namespace.as_str())
            || !snapshot.config.config_json.is_object()
        {
            return Err(invalid());
        }
        let names: BTreeSet<_> = snapshot
            .skills
            .iter()
            .map(|skill| skill.name.as_str())
            .collect();
        if names.len() != snapshot.skills.len()
            || snapshot
                .skills
                .iter()
                .any(|skill| skill.agent_id != agent.id)
        {
            return Err(invalid());
        }
        snapshot.config.soul_md = self.instruction.clone();
        snapshot.config.config_json["fleet_sdlc_package"] =
            serde_json::to_value(&self.proof).map_err(|_| invalid())?;
        for old in &mut snapshot.skills {
            if !self.skills.contains_key(&old.name) {
                old.state = SkillState::Disabled;
                old.content = None;
            }
        }
        for (name, content) in &self.skills {
            let source = format!("base-sdlc:{BASE_PACKAGE_COMMIT}");
            if let Some(old) = snapshot.skills.iter_mut().find(|skill| skill.name == *name) {
                if old.agent_id != agent.id {
                    return Err(invalid());
                }
                old.state = SkillState::Enabled;
                old.source = source;
                old.content = Some(content.clone());
            } else {
                snapshot.skills.push(AgentSkill {
                    id: Uuid::new_v4(),
                    agent_id: agent.id,
                    name: name.clone(),
                    title: name.clone(),
                    state: SkillState::Enabled,
                    source,
                    content: Some(content.clone()),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
        Ok(snapshot)
    }
}
