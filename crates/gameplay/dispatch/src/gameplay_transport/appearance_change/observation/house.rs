//! Observation-only adaptation of the existing appearance owner to native House custody.
use super::*;
use crate::durability::{DurabilityError, character_progression::CharacterProgressionError};
use crate::foundation::HouseHirelingEntity;

pub(super) async fn read_state(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    grant_keys: Vec<String>,
    failure: &mut Option<ReadFailure>,
) -> Option<CurrentCharacterAppearance> {
    let result = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { .. } => {
            admission
                .root
                .read_current_character_appearance_state(
                    admission.character,
                    admission.holder,
                    fence,
                    grant_keys,
                )
                .await
        }
        RuntimeScopeRefV1::Instance { .. } => {
            admission
                .root
                .read_current_house_character_appearance_state(
                    admission.character,
                    admission.holder,
                    fence,
                    *admission.channel_id.as_bytes(),
                    grant_keys,
                )
                .await
        }
    };
    result
        .inspect_err(|error| *failure = Some(read_failure(error)))
        .ok()
}

// [DBG-NODEBOOT]: classify the existing refusal without rendering its error payload.
fn read_failure(error: &AppearanceWriteError) -> ReadFailure {
    let (class, subclass) = match error {
        AppearanceWriteError::InvalidInput => ("invalid_input", "none"),
        AppearanceWriteError::StateMismatch => ("state_mismatch", "none"),
        AppearanceWriteError::Rejected(_) => ("rejected", "none"),
        AppearanceWriteError::Fence(error) => match error {
            CharacterProgressionError::InvalidInput => ("fence", "invalid_input"),
            CharacterProgressionError::AuthorityRejected => ("fence", "authority_rejected"),
            CharacterProgressionError::MissingProgressionState => {
                ("fence", "missing_progression_state")
            }
            CharacterProgressionError::CharacterRevisionMismatch => {
                ("fence", "character_revision_mismatch")
            }
            CharacterProgressionError::ProgressionContextMismatch => {
                ("fence", "progression_context_mismatch")
            }
            CharacterProgressionError::ConflictingOccurrence => ("fence", "conflicting_occurrence"),
            CharacterProgressionError::RespawnPending => ("fence", "respawn_pending"),
            CharacterProgressionError::HeldBlessingsMismatch => {
                ("fence", "held_blessings_mismatch")
            }
            CharacterProgressionError::BuildStateMismatch => ("fence", "build_state_mismatch"),
            CharacterProgressionError::StanceStateMismatch => ("fence", "stance_state_mismatch"),
            CharacterProgressionError::FamiliarStateMismatch => {
                ("fence", "familiar_state_mismatch")
            }
            CharacterProgressionError::Calculation(_) => ("fence", "calculation"),
            CharacterProgressionError::Unavailable(error) => {
                ("fence_unavailable", durability_class(error))
            }
        },
        AppearanceWriteError::Unavailable(error) => ("unavailable", durability_class(error)),
    };
    ReadFailure { class, subclass }
}

fn durability_class(error: &DurabilityError) -> &'static str {
    match error {
        DurabilityError::AdmissionGameSessionLedgerExhausted => "ledger_exhausted",
        DurabilityError::SessionUseAuthorization(_) => "session_use_authorization",
        DurabilityError::Unavailable => "unavailable",
        DurabilityError::Database(_) => "database",
        DurabilityError::Migration(_) => "migration",
        DurabilityError::SchemaIncompatible(_) => "schema_incompatible",
        DurabilityError::InvalidConfiguration => "invalid_configuration",
        DurabilityError::RootUnavailable => "root_unavailable",
        DurabilityError::RootPassDeadlineExceeded => "pass_deadline",
        DurabilityError::CommitRejected => "commit_rejected",
        DurabilityError::CommitOutcomeUnknown => "commit_unknown",
        DurabilityError::RootTaskFailed => "root_task_failed",
        DurabilityError::InvalidStoredState => "invalid_stored_state",
    }
}

pub(super) fn current_runtime_for_read(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: &CurrentCharacterGameplayFence,
    content: &NativeCosmeticsView,
) -> bool {
    if matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
        return current_runtime(runtime, actor, session, fence, content);
    }
    runtime.house_actor_state(actor, session).is_ok_and(|state| {
        state.scope == fence.runtime_scope && state.generation == fence.scope_ownership_generation
            && state.lease.character_id().as_bytes() == fence.character_id.as_bytes()
            && state.lease.generation() == fence.character_lease_generation
    }) && runtime.house_player_control_facts(actor, session).is_ok_and(|facts| facts.control_loss.is_none())
        && runtime.content_pin().server_artifact_digest() == content.source_digest()
}

type ActorCensus = Vec<(ExactActorRef, Option<GameSessionId>)>;
pub(super) fn census(
    runtime: &ChannelRuntimeV1,
    observer: ExactActorRef,
    session: GameSessionId,
) -> Option<(ActorCensus, Vec<HouseHirelingEntity>)> {
    if runtime.house_actor_state(observer, session).is_ok() {
        let players = runtime.house_visible_players(observer, session).ok()?.into_iter().map(|entry| {
            let (session, _) = runtime.house_observer(entry.actor).ok()??;
            Some((entry.actor, Some(session)))
        }).collect::<Option<Vec<_>>>()?;
        Some((players, runtime.house_hireling_entities(observer, session).ok()?))
    } else {
        Some((runtime.positioned_melee_census().ok()?.into_iter().map(|(actor, _, session)| (actor, session)).collect(), Vec::new()))
    }
}
