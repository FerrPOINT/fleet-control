use super::*;
use serde_json::{Value, json};

// Synthetic content only: never embed private Base instructions in public tests.
fn package() -> (Value, BTreeMap<String, Vec<u8>>) {
    let names: Vec<_> = std::iter::once("project-workflow-executor".to_owned())
        .chain(('a'..='m').map(|letter| format!("fixture-{letter}")))
        .collect();
    let mut files = BTreeMap::new();
    let mut hashes = BTreeMap::new();
    for name in &names {
        let text = format!(
            "---\nname: {name}\ndescription: Synthetic test fixture\n---\nTest-only content\n"
        );
        hashes.insert(name.clone(), digest(text.as_bytes()).unwrap());
        files.insert(
            format!("agent-skills/skills/{name}/SKILL.md"),
            text.into_bytes(),
        );
    }
    let mut roles = BTreeMap::new();
    for role in ROLES {
        let text = format!("Synthetic role {role}\n");
        files.insert(
            format!("agent-skills/roles/{role}.md"),
            text.as_bytes().to_vec(),
        );
        roles.insert(role, json!({"namespace": namespace(role), "profile": profile(role), "modes": modes(role),
            "physicalSkills": names, "roleInstruction": {"path": format!("roles/{role}.md"), "sha256": digest(text.as_bytes()).unwrap()}}));
    }
    (
        json!({"schema": "base-hermes-role-skills/v1", "status": "candidate-not-installed",
        "catalogAuthority": {"purpose": "physical-skill-and-profile-validation-only", "selectionAuthority": "task-tracker-backend-assignment",
            "ownsRouting": false, "ownsModeSelection": false, "ownsWorkspaceSelection": false, "ownsPrioritySelection": false},
        "sources": {"native": {"repository": REPOSITORY, "revision": "SELF", "revisionMeaning": "containing commit", "hashAlgorithm": "sha256-normalized-lf-utf8", "skills": hashes}},
        "provenance": {"adaptation": "synthetic", "donorSkillsRevision": "a".repeat(40), "skillsHubRevision": "b".repeat(40), "runtimeDependencyOnDonor": false}, "roles": roles}),
        files,
    )
}

fn verify(
    manifest: &Value,
    files: &BTreeMap<String, Vec<u8>>,
    role: SdlcRole,
) -> Result<VerifiedRolePackage, AppError> {
    VerifiedRolePackage::verify(
        &serde_json::to_vec(manifest).unwrap(),
        files.keys().cloned().collect(),
        role,
        |path| files.get(path).cloned().ok_or_else(invalid),
    )
}

#[test]
fn validates_all_roles_without_serializing_private_content() {
    let (manifest, files) = package();
    for role in [
        SdlcRole::ProjectManager,
        SdlcRole::Analyst,
        SdlcRole::Architect,
        SdlcRole::Developer,
        SdlcRole::Reviewer,
        SdlcRole::Tester,
        SdlcRole::DevOps,
    ] {
        let package = verify(&manifest, &files, role).unwrap();
        assert_eq!(package.proof.role, role_key(role));
        let proof = serde_json::to_string(package.proof()).unwrap();
        assert!(!proof.contains("Synthetic role"));
        assert!(!proof.contains("Test-only content"));
        assert_eq!(package.proof.commit, BASE_PACKAGE_COMMIT);
    }
}

#[test]
fn rejects_schema_authority_pin_mode_scope_and_path_drift() {
    let (manifest, files) = package();
    for (pointer, replacement) in [
        ("/schema", json!("other/v1")),
        ("/status", json!("installed")),
        ("/catalogAuthority/ownsRouting", json!(true)),
        ("/sources/native/revision", json!("HEAD")),
        (
            "/sources/native/repository",
            json!("https://example.invalid/other.git"),
        ),
        ("/provenance/runtimeDependencyOnDonor", json!(true)),
        ("/provenance/donorSkillsRevision", json!("z".repeat(40))),
        ("/roles/developer/modes", json!(["delivery", "aggregate"])),
        ("/roles/devops/namespace", json!("hermes-dev_ops")),
        ("/roles/tester/roleInstruction/path", json!("../private.md")),
        ("/roles/analyst/physicalSkills", json!(["fixture-a"])),
    ] {
        let mut changed = manifest.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            verify(&changed, &files, SdlcRole::Developer).is_err(),
            "{pointer}"
        );
    }
    let mut changed = manifest.clone();
    changed["unexpected"] = json!(true);
    assert!(verify(&changed, &files, SdlcRole::Developer).is_err());
}

#[test]
fn rejects_tampered_extra_missing_and_invalid_utf8_blobs() {
    let (manifest, files) = package();
    let path = "agent-skills/skills/fixture-a/SKILL.md";
    for bytes in [Some(b"modified".to_vec()), Some(vec![0xff]), None] {
        let mut changed = files.clone();
        match bytes {
            Some(bytes) => {
                changed.insert(path.into(), bytes);
            }
            None => {
                changed.remove(path);
            }
        }
        assert!(verify(&manifest, &changed, SdlcRole::Developer).is_err());
    }
    let mut changed = files.clone();
    changed.insert(
        "agent-skills/skills/extra/SKILL.md".into(),
        b"extra".to_vec(),
    );
    assert!(verify(&manifest, &changed, SdlcRole::Developer).is_err());
}

#[test]
fn normalizes_crlf_but_not_arbitrary_content() {
    let (manifest, files) = package();
    let crlf = files
        .into_iter()
        .map(|(path, bytes)| {
            (
                path,
                String::from_utf8(bytes)
                    .unwrap()
                    .replace('\n', "\r\n")
                    .into_bytes(),
            )
        })
        .collect();
    assert!(verify(&manifest, &crlf, SdlcRole::Developer).is_ok());
    assert_eq!(
        digest(b"one\r\ntwo\r\n").unwrap(),
        digest(b"one\ntwo\n").unwrap()
    );
    assert_ne!(
        digest(b"one\rtwo\r").unwrap(),
        digest(b"one\ntwo\n").unwrap()
    );
}

#[tokio::test]
async fn real_git_pin_smoke_when_explicitly_configured() {
    let Some(checkout) = std::env::var_os("FLEET_TEST_BASE_PACKAGE_CHECKOUT") else {
        return;
    };
    for role in [
        SdlcRole::ProjectManager,
        SdlcRole::Analyst,
        SdlcRole::Architect,
        SdlcRole::Developer,
        SdlcRole::Reviewer,
        SdlcRole::Tester,
        SdlcRole::DevOps,
    ] {
        let package = VerifiedRolePackage::read(Path::new(&checkout), role)
            .await
            .unwrap();
        assert_eq!(package.proof.role, role_key(role));
        assert_eq!(
            package
                .proof
                .modes
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            modes(role_key(role))
        );
    }
}

#[test]
fn preparation_changes_a_draft_only_and_disables_skills_outside_allowlist() {
    let (manifest, files) = package();
    let package = verify(&manifest, &files, SdlcRole::Developer).unwrap();
    let mut agent = crate::tests::test_agent(
        Path::new("unused-test-root"),
        Uuid::new_v4(),
        AgentStatus::Stopped,
    );
    agent.namespace_id = Some("hermes-developer".into());
    let old = AgentSkill {
        id: Uuid::new_v4(),
        agent_id: agent.id,
        name: "old-skill".into(),
        title: "Old".into(),
        state: SkillState::Enabled,
        source: "test".into(),
        content: Some("Old content".into()),
        updated_at: agent.updated_at.clone(),
    };
    let snapshot = AgentConfigurationSnapshot {
        config: domain::UpdateAgentConfigRequest {
            config_json: json!({"model": "test-model"}),
            soul_md: "Previous SOUL".into(),
            env_json: json!({"PROVIDER_API_KEY": {"secret_ref": "TEST_PROVIDER"}}),
        },
        skills: vec![old],
    };
    let prepared = package.prepare_snapshot(&agent, snapshot.clone()).unwrap();
    assert_eq!(snapshot.config.soul_md, "Previous SOUL");
    assert_eq!(prepared.config.config_json["model"], "test-model");
    assert_eq!(prepared.config.env_json, snapshot.config.env_json);
    assert_eq!(prepared.skills[0].id, snapshot.skills[0].id);
    assert_eq!(prepared.skills[0].state, SkillState::Disabled);
    assert!(prepared.skills[0].content.is_none());
    assert_eq!(
        prepared
            .skills
            .iter()
            .filter(|skill| skill.state == SkillState::Enabled)
            .count(),
        14
    );
    assert!(prepared.config.input_errors().is_empty());
    assert!(package.verify_snapshot(&agent, &prepared).is_ok());
    let mut forged = prepared.clone();
    forged.config.soul_md.push_str(" changed");
    assert!(package.verify_snapshot(&agent, &forged).is_err());
    let mut forged = prepared.clone();
    forged.config.config_json["fleet_sdlc_package"]["commit"] = json!("HEAD");
    assert!(package.verify_snapshot(&agent, &forged).is_err());
    let mut forged = prepared.clone();
    forged.skills[1].content = Some("changed".into());
    assert!(package.verify_snapshot(&agent, &forged).is_err());
    let mut forged = prepared.clone();
    forged.skills[1].state = SkillState::Disabled;
    assert!(package.verify_snapshot(&agent, &forged).is_err());
    let mut forged = prepared.clone();
    forged.skills[0].state = SkillState::Enabled;
    assert!(package.verify_snapshot(&agent, &forged).is_err());
    assert_eq!(
        package
            .prepare_snapshot(&agent, prepared.clone())
            .unwrap()
            .skills
            .len(),
        prepared.skills.len()
    );
    let mut foreign = snapshot.clone();
    foreign.skills[0].agent_id = Uuid::new_v4();
    assert!(package.prepare_snapshot(&agent, foreign).is_err());
    let mut duplicate = snapshot.clone();
    duplicate.skills.push(duplicate.skills[0].clone());
    assert!(package.prepare_snapshot(&agent, duplicate).is_err());
    agent.kind = AgentKind::JavaAgent;
    assert!(package.prepare_snapshot(&agent, snapshot.clone()).is_err());
    agent.kind = AgentKind::Hermes;
    agent.sdlc_role = Some(SdlcRole::Tester);
    assert!(package.prepare_snapshot(&agent, snapshot.clone()).is_err());
    agent.sdlc_role = Some(SdlcRole::Developer);
    agent.namespace_id = Some("unrelated".into());
    assert!(package.prepare_snapshot(&agent, snapshot.clone()).is_err());
    agent.namespace_id = Some("hermes-developer".into());
    agent.status = AgentStatus::Archived;
    assert!(package.prepare_snapshot(&agent, snapshot).is_err());
}

#[test]
fn batch_decoder_rejects_truncated_substituted_and_surplus_data() {
    let hash = "a".repeat(40);
    let entries = vec![("test.md".to_owned(), hash.clone(), 3)];
    let payload = format!("{hash} blob 3\nabc\n");
    assert_eq!(
        decode_batch(&entries, payload.as_bytes()).unwrap()["test.md"],
        b"abc"
    );
    for bad in [
        format!("{hash} blob 3\nab"),
        format!("{hash} blob 4\nabc\n"),
        format!("{hash} tree 3\nabc\n"),
        format!("{} blob 3\nabc\n", "b".repeat(40)),
        format!("{payload}surplus"),
    ] {
        assert!(decode_batch(&entries, bad.as_bytes()).is_err());
    }
}

#[tokio::test]
async fn bounded_git_io_closes_stdin_and_checks_success() {
    let mut command = Command::new("git");
    command
        .args(["hash-object", "--stdin"])
        .current_dir(std::env::temp_dir());
    assert_eq!(
        bounded_output(&mut command, b"abc", 41, Duration::from_secs(5))
            .await
            .unwrap(),
        b"f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f\n"
    );
    let mut command = Command::new("git");
    command
        .args(["hash-object", "--stdin"])
        .current_dir(std::env::temp_dir());
    assert!(
        bounded_output(&mut command, b"abc", 40, Duration::from_secs(5))
            .await
            .is_err()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn bounded_git_io_denies_overflow_and_wait_timeout() {
    for (script, limit) in [("exec yes", 256), ("exec 1>&-; exec sleep 30", 256)] {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        let started = std::time::Instant::now();
        assert!(
            bounded_output(&mut command, &[], limit, Duration::from_millis(50))
                .await
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}

#[cfg(unix)]
#[tokio::test]
async fn bounded_git_io_discards_stderr_and_failed_output() {
    let mut command = Command::new("sh");
    command.args([
        "-c",
        "printf 'private-path-and-role-content' >&2; printf 'partial'; exit 7",
    ]);
    let error = bounded_output(&mut command, &[], 256, Duration::from_secs(5))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), invalid().to_string());
    assert!(!error.to_string().contains("private-path"));
    assert!(!error.to_string().contains("partial"));
}
