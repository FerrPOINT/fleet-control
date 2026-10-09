//! Private, non-secret activation custody. Native effects never precede their journal step.
use crate::container_runtime::ContainerLaunch;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::AppError;
use uuid::Uuid;

/// Fresh readback, not a transferable permit. Repository checks the exact current lease.
#[derive(Clone)]
pub struct RecoveredProof {
    pub lease: crate::container_runtime::ContainerRecoveryCommand,
    pub observation: Value,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    pub generation: Uuid,
    pub operation_id: Uuid,
    pub stop_id: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub controller_id: Uuid,
    pub revision: i64,
    pub previous_revision: Option<i64>,
    pub configuration_sha256: String,
    pub previous_configuration_sha256: Option<String>,
    pub files_sha256: String,
    pub previous_files_sha256: String,
    pub intent_sha256: String,
    pub candidate_intent_sha256: String,
    pub rollback_intent_sha256: String,
    pub previous: ContainerLaunch,
    pub candidate: Generation,
    pub rollback: Generation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Planned,
    StoppingPrevious,
    PreviousStopped,
    ApplyingCandidate,
    PreparingCandidate,
    CandidatePrepared,
    StartingCandidate,
    CandidateRunning,
    CandidateReady,
    Committed,
    StoppingCandidate,
    CandidateStopped,
    ApplyingRollback,
    PreparingRollback,
    RollbackPrepared,
    StartingRollback,
    RollbackRunning,
    RollbackReady,
    RolledBack,
}

impl Phase {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack)
    }

    pub fn permits(self, next: Self) -> bool {
        use Phase::*;
        matches!(
            (self, next),
            (Planned, StoppingPrevious)
                | (StoppingPrevious, PreviousStopped)
                | (PreviousStopped, ApplyingCandidate)
                | (ApplyingCandidate, PreparingCandidate)
                | (PreparingCandidate, CandidatePrepared)
                | (CandidatePrepared, StartingCandidate)
                | (StartingCandidate, CandidateRunning)
                | (CandidateRunning, CandidateReady)
                | (CandidateReady, Committed)
                | (CandidateRunning, StoppingCandidate)
                | (StoppingCandidate, CandidateStopped)
                | (CandidateStopped, ApplyingRollback)
                | (ApplyingCandidate, ApplyingRollback)
                | (ApplyingRollback, PreparingRollback)
                | (PreparingRollback, RollbackPrepared)
                | (RollbackPrepared, StartingRollback)
                | (StartingRollback, RollbackRunning)
                | (RollbackRunning, RollbackReady)
                | (RollbackReady, RolledBack)
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Readiness {
    pub generation: Uuid,
    pub files_sha256: String,
    pub capabilities_sha256: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub claim: Claim,
    pub phase: Phase,
    pub previous_stop: Option<Value>,
    pub candidate: Option<ContainerLaunch>,
    pub candidate_stop: Option<Value>,
    pub rollback: Option<ContainerLaunch>,
    pub readiness: Option<Readiness>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    OriginalCustodyRequired,
    RecoveredGenerationChangeUnsupported,
    UnknownOriginalEffect,
    EvidenceUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    RecoverOriginalCustody,
    ReconcileOriginalCommand,
    ResumeOrRollbackOriginalPlanWithCompatibleBase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    Held,
}

/// A durable recovery request, not an execution permit or successful activation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryHold {
    pub state: RecoveryState,
    pub reason: RecoveryReason,
    pub action: RecoveryAction,
    pub controller_id: Uuid,
    pub original_controller_id: Uuid,
    pub custody_generation: Uuid,
    pub generation: Uuid,
    pub operation_id: Uuid,
    pub stop_id: Uuid,
    pub activation_id: Option<Uuid>,
    pub phase: Option<Phase>,
    pub intent_sha256: Option<String>,
    pub recovery_id: Option<Uuid>,
    pub recovery_command_sha256: Option<String>,
    pub readback_sha256: Option<String>,
}

impl RecoveryHold {
    pub fn new(
        controller: Uuid,
        launch: &ContainerLaunch,
        activation: Option<&Activation>,
    ) -> Self {
        let unknown = activation.is_some_and(|a| {
            matches!(
                a.phase,
                Phase::StoppingPrevious
                    | Phase::PreparingCandidate
                    | Phase::StartingCandidate
                    | Phase::StoppingCandidate
                    | Phase::PreparingRollback
                    | Phase::StartingRollback
            )
        });
        let original = &launch.prepared.container.registration;
        let command = activation
            .map(|a| match a.phase {
                Phase::ApplyingCandidate
                | Phase::PreparingCandidate
                | Phase::CandidatePrepared
                | Phase::StartingCandidate
                | Phase::CandidateRunning
                | Phase::CandidateReady
                | Phase::StoppingCandidate
                | Phase::CandidateStopped => a.claim.candidate.clone(),
                Phase::ApplyingRollback
                | Phase::PreparingRollback
                | Phase::RollbackPrepared
                | Phase::StartingRollback
                | Phase::RollbackRunning
                | Phase::RollbackReady => a.claim.rollback.clone(),
                _ => Generation {
                    generation: a.claim.previous.prepared.container.registration.generation,
                    operation_id: a
                        .claim
                        .previous
                        .prepared
                        .container
                        .registration
                        .operation_id,
                    stop_id: a.claim.previous.stop_id,
                },
            })
            .unwrap_or(Generation {
                generation: original.generation,
                operation_id: original.operation_id,
                stop_id: launch.stop_id,
            });
        Self {
            state: RecoveryState::Held,
            reason: if unknown {
                RecoveryReason::UnknownOriginalEffect
            } else {
                RecoveryReason::OriginalCustodyRequired
            },
            action: if unknown {
                RecoveryAction::ReconcileOriginalCommand
            } else {
                RecoveryAction::RecoverOriginalCustody
            },
            controller_id: controller,
            original_controller_id: launch.controller_id,
            custody_generation: original.generation,
            generation: command.generation,
            operation_id: command.operation_id,
            stop_id: command.stop_id,
            activation_id: activation.map(|a| a.claim.id),
            phase: activation.map(|a| a.phase),
            intent_sha256: activation.map(|a| a.claim.intent_sha256.clone()),
            recovery_id: None,
            recovery_command_sha256: None,
            readback_sha256: None,
        }
    }
}

pub fn held() -> AppError {
    AppError::Unavailable("Original Docker activation is held; no replacement, rollback or credential rotation without proof".into())
}

fn same<T: Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

pub fn stopped(launch: &ContainerLaunch, proof: &Value, snapshot_hash: &str) -> bool {
    let r = &launch.prepared.container.registration;
    proof["state"] == "observed"
        && proof["observation"] == "namespace_exited"
        && proof["operation_id"] == launch.stop_id.to_string()
        && proof["generation"] == r.generation.to_string()
        && proof["resource_id"] == r.resource_id.to_string()
        && proof["container_id"] == r.container_id
        && proof["contract_version"] == r.contract_version
        && proof["snapshot_sha256"] == snapshot_hash
}

impl Activation {
    pub fn tracks_launch(&self, launch: &ContainerLaunch) -> bool {
        std::iter::once(&self.claim.previous)
            .chain(self.candidate.iter())
            .chain(self.rollback.iter())
            .any(|original| {
                original.controller_id == launch.controller_id
                    && original.stop_id == launch.stop_id
                    && same(&original.prepared, &launch.prepared)
                    && original.snapshot == launch.snapshot
                    && original.origin == launch.origin
            })
    }

    pub fn planned(claim: Claim) -> Self {
        Self {
            claim,
            phase: Phase::Planned,
            previous_stop: None,
            candidate: None,
            candidate_stop: None,
            rollback: None,
            readiness: None,
        }
    }

    pub fn validate_next(&self, next: &Self) -> Result<(), AppError> {
        use Phase::*;
        // Each receipt has one admission point. Later phases cannot inject earlier proof.
        for (old, new, allowed) in [
            (&self.previous_stop, &next.previous_stop, PreviousStopped),
            (&self.candidate_stop, &next.candidate_stop, CandidateStopped),
        ] {
            if !same(old, new) && (old.is_some() || next.phase != allowed) {
                return Err(held());
            }
        }
        if !same(&self.readiness, &next.readiness)
            && (self.readiness.is_some() || !matches!(next.phase, CandidateReady | RollbackReady))
        {
            return Err(held());
        }
        for (old, new, allowed) in [
            (
                &self.candidate,
                &next.candidate,
                matches!(
                    next.phase,
                    CandidatePrepared | CandidateRunning | StoppingCandidate | CandidateStopped
                ),
            ),
            (
                &self.rollback,
                &next.rollback,
                matches!(next.phase, RollbackPrepared | RollbackRunning),
            ),
        ] {
            if !same(old, new) && !allowed {
                return Err(held());
            }
        }
        if !same(&self.claim, &next.claim)
            || !self.phase.permits(next.phase)
            || self
                .previous_stop
                .as_ref()
                .is_some_and(|v| next.previous_stop.as_ref() != Some(v))
            || self
                .candidate_stop
                .as_ref()
                .is_some_and(|v| next.candidate_stop.as_ref() != Some(v))
            || self
                .readiness
                .as_ref()
                .is_some_and(|v| !same(&Some(v), &next.readiness.as_ref()))
        {
            return Err(held());
        }
        for (old, new, identity, revision, hash) in [
            (
                &self.candidate,
                &next.candidate,
                &self.claim.candidate,
                Some(self.claim.revision),
                Some(&self.claim.configuration_sha256),
            ),
            (
                &self.rollback,
                &next.rollback,
                &self.claim.rollback,
                self.claim.previous_revision,
                self.claim.previous_configuration_sha256.as_ref(),
            ),
        ] {
            if let Some(l) = new {
                let p = &l.prepared;
                if l.controller_id != self.claim.controller_id
                    || l.stop_id != identity.stop_id
                    || p.agent_id != self.claim.agent_id
                    || p.container.registration.resource_id != p.agent_id
                    || p.container.registration.generation != identity.generation
                    || p.container.registration.operation_id != identity.operation_id
                    || p.configuration_revision != revision
                    || p.configuration_sha256.as_ref() != hash
                    || !same(&p.paths, &self.claim.previous.prepared.paths)
                    || p.api_port != self.claim.previous.prepared.api_port
                    || p.container.policy["image_id"]
                        != self.claim.previous.prepared.container.policy["image_id"]
                {
                    return Err(held());
                }
                if let Some(old) = old {
                    if !same(&old.prepared, &l.prepared)
                        || old
                            .snapshot
                            .as_ref()
                            .is_some_and(|s| l.snapshot.as_ref() != Some(s))
                        || old
                            .origin
                            .as_ref()
                            .is_some_and(|s| l.origin.as_ref() != Some(s))
                    {
                        return Err(held());
                    }
                }
            } else if old.is_some() {
                return Err(held());
            }
        }
        if !matches!(next.phase, Planned | StoppingPrevious) && next.previous_stop.is_none() {
            return Err(held());
        }
        let required = match next.phase {
            CandidatePrepared | StartingCandidate => Some((&next.candidate, "claimed")),
            CandidateRunning | CandidateReady | Committed => Some((&next.candidate, "running")),
            StoppingCandidate => Some((&next.candidate, "stopping")),
            CandidateStopped => Some((&next.candidate, "exited")),
            RollbackPrepared | StartingRollback => Some((&next.rollback, "claimed")),
            RollbackRunning | RollbackReady | RolledBack => Some((&next.rollback, "running")),
            _ => None,
        };
        if let Some((launch, state)) = required {
            let l = launch.as_ref().ok_or_else(held)?;
            if l.state != state
                || state == "running" && (l.snapshot.is_none() || l.origin.is_none())
                || state == "claimed" && (l.snapshot.is_some() || l.origin.is_some())
            {
                return Err(held());
            }
        }
        if matches!(
            next.phase,
            ApplyingRollback
                | PreparingRollback
                | RollbackPrepared
                | StartingRollback
                | RollbackRunning
                | RollbackReady
                | RolledBack
        ) && next.candidate.is_some()
            && (next.candidate_stop.is_none()
                || next.candidate.as_ref().is_none_or(|l| l.state != "exited"))
        {
            return Err(held());
        }
        if matches!(
            next.phase,
            CandidateReady | Committed | RollbackReady | RolledBack
        ) {
            let rollback = matches!(next.phase, RollbackReady | RolledBack);
            let ready = next.readiness.as_ref().ok_or_else(held)?;
            if ready.generation
                != if rollback {
                    self.claim.rollback.generation
                } else {
                    self.claim.candidate.generation
                }
                || ready.files_sha256
                    != if rollback {
                        &self.claim.previous_files_sha256
                    } else {
                        &self.claim.files_sha256
                    }
                    .as_str()
                || ready.capabilities_sha256.len() != 64
            {
                return Err(held());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Phase::*;

    #[test]
    fn unknown_effects_have_no_rollback_or_duplicate_delivery_edge() {
        for unknown in [
            StoppingPrevious,
            PreparingCandidate,
            StartingCandidate,
            StoppingCandidate,
            PreparingRollback,
            StartingRollback,
        ] {
            assert!(!unknown.permits(ApplyingRollback));
            assert!(!unknown.permits(PreparingCandidate));
            assert!(!unknown.permits(Committed));
        }
    }

    #[test]
    fn activation_and_rollback_require_full_proof_sequence() {
        let success = [
            Planned,
            StoppingPrevious,
            PreviousStopped,
            ApplyingCandidate,
            PreparingCandidate,
            CandidatePrepared,
            StartingCandidate,
            CandidateRunning,
            CandidateReady,
            Committed,
        ];
        let rollback = [
            CandidateRunning,
            StoppingCandidate,
            CandidateStopped,
            ApplyingRollback,
            PreparingRollback,
            RollbackPrepared,
            StartingRollback,
            RollbackRunning,
            RollbackReady,
            RolledBack,
        ];
        for path in [success.as_slice(), rollback.as_slice()] {
            for pair in path.windows(2) {
                assert!(pair[0].permits(pair[1]));
            }
            assert!(path.last().unwrap().terminal());
        }
        assert!(!CandidateRunning.permits(Committed));
        assert!(!RollbackRunning.permits(RolledBack));
        assert!(!Committed.permits(Planned));
    }
}
