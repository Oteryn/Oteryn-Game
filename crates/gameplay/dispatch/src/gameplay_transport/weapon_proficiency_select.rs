//! P08 command 378: select, change or clear one Weapon Proficiency perk through the existing
//! `perk_selection` writer, then refresh the session projection so the perk applies to the
//! next attack. Crystal `src/server/network/protocol/protocolgame.cpp:11873` (APPLY_PERKS);
//! the protection-zone rule follows English TibiaWiki `Weapon_Proficiency` (Global wins).
use std::sync::Arc;

#[path = "weapon_proficiency_select/catalyst.rs"]
pub(super) mod catalyst;
#[path = "weapon_proficiency_select/modify.rs"]
pub(super) mod modify;

use sha2::{Digest, Sha256};

use super::ComposedFreshAdmission;
use super::actor_spell::TrainingSave;
use super::connection::UseCommand;
use super::item_ref_admission::pinned_index;
use super::weapon_proficiency::{LiveProficiencyTrack, WeaponProficiencyDispatch};
use crate::durability::character_proficiency::{
    CommittedProficiencyChange, DurableProficiencyState, ProficiencyCause,
    ProficiencyChangeRequest, ProficiencyCommitOutcome, ProficiencyDefinitions,
    ProficiencyLineCandidate, ProficiencyOccurrence, ProficiencyWriteRefusal,
    StoredProficiencyTrack,
};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    ProficiencyError, ProficiencySelectionCommand, ProficiencySelectionContext,
    ProficiencySelectionPlan, plan_proficiency_selection,
};
use oteryn_protocol_oteryn::weapon_proficiency::{
    SelectPerkReply, SelectPerkRequest, SelectPerkResult,
};

macro_rules! authority_methods {
    () => {
        fn weapon_proficiency_select(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _request: oteryn_protocol_oteryn::weapon_proficiency::SelectPerkRequest,
        ) -> impl std::future::Future<
            Output = Option<oteryn_protocol_oteryn::weapon_proficiency::SelectPerkReply>,
        > {
            async {
                Some(oteryn_protocol_oteryn::weapon_proficiency::SelectPerkReply {
                    result: oteryn_protocol_oteryn::weapon_proficiency::SelectPerkResult::Rejected,
                    selections: Vec::new(),
                })
            }
        }

        /// P08: a Proficiency Catalyst used on a weapon; `None` when the source is no catalyst.
        fn use_proficiency_catalyst(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _catalyst: $crate::gameplay_transport::item_view::ItemKey,
            _weapon: $crate::gameplay_transport::item_view::ItemKey,
        ) -> impl std::future::Future<
            Output = Option<$crate::gameplay_transport::connection::UseOutcome>,
        > {
            async { None }
        }

        /// P08: one perk modification command (PROFICIENCY-1B §11.1).
        fn weapon_proficiency_modify(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _request: oteryn_protocol_oteryn::weapon_proficiency::ModificationRequest,
        ) -> impl std::future::Future<
            Output = Option<oteryn_protocol_oteryn::weapon_proficiency::ModificationReply>,
        > {
            async {
                Some(oteryn_protocol_oteryn::weapon_proficiency::ModificationReply {
                    result:
                        oteryn_protocol_oteryn::weapon_proficiency::ModificationResult::NotAdmitted,
                    slot: None,
                })
            }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn weapon_proficiency_select(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            request: oteryn_protocol_oteryn::weapon_proficiency::SelectPerkRequest,
        ) -> Option<oteryn_protocol_oteryn::weapon_proficiency::SelectPerkReply> {
            $crate::gameplay_transport::weapon_proficiency_select::execute(
                self, actor, command, request,
            )
            .await
        }

        async fn use_proficiency_catalyst(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            catalyst: $crate::gameplay_transport::item_view::ItemKey,
            weapon: $crate::gameplay_transport::item_view::ItemKey,
        ) -> Option<$crate::gameplay_transport::connection::UseOutcome> {
            $crate::gameplay_transport::weapon_proficiency_select::catalyst::use_catalyst(
                self, actor, command, catalyst, weapon,
            )
            .await
        }

        async fn weapon_proficiency_modify(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            request: oteryn_protocol_oteryn::weapon_proficiency::ModificationRequest,
        ) -> Option<oteryn_protocol_oteryn::weapon_proficiency::ModificationReply> {
            $crate::gameplay_transport::weapon_proficiency_select::modify::execute(
                self, actor, command, request,
            )
            .await
        }
    };
}
pub(super) use composed_methods;

fn reply(result: SelectPerkResult, selections: &[Option<u8>]) -> Option<SelectPerkReply> {
    Some(SelectPerkReply {
        result,
        selections: selections.to_vec(),
    })
}

/// The planner's refusals as command results.
fn refusal(error: ProficiencyError) -> SelectPerkResult {
    match error {
        ProficiencyError::InvalidSelectionLevel | ProficiencyError::SelectionNotUnlocked => {
            SelectPerkResult::NotUnlocked
        }
        ProficiencyError::InvalidSelectionPerk => SelectPerkResult::InvalidPerk,
        ProficiencyError::ProtectionZoneRequired => SelectPerkResult::NotInProtectionZone,
        ProficiencyError::StaleTrackRevision
        | ProficiencyError::InvalidStoredSelections
        | ProficiencyError::InvalidPerkLevels
        | ProficiencyError::InvalidInfluenceStacks
        | ProficiencyError::InvalidPerksPerLevel => SelectPerkResult::Busy,
    }
}

/// The occurrence of one selection, bound to its CommandRef and actor (PROFICIENCY-0 §4.3).
fn occurrence(
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
) -> Option<ProficiencyOccurrence> {
    let digest = Sha256::new()
        .chain_update(b"oteryn:proficiency-select-occurrence:v1")
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    ProficiencyOccurrence::from_bytes(bytes).ok()
}

/// A receipt of this occurrence answers the command again only if it is the same selection.
fn replayed(
    receipt: &CommittedProficiencyChange,
    item_key: &str,
    request: SelectPerkRequest,
) -> SelectPerkResult {
    let same = receipt.cause == ProficiencyCause::PerkSelection
        && receipt.lines.len() == 1
        && receipt.lines.first().is_some_and(|line| {
            line.after().item_key() == item_key
                && line.after().selections().get(usize::from(request.level))
                    == Some(&request.perk)
        });
    if same {
        SelectPerkResult::Accepted
    } else {
        SelectPerkResult::Conflict
    }
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: SelectPerkRequest,
) -> Option<SelectPerkReply> {
    let session = command.game_session_id;
    let rejected = || reply(SelectPerkResult::Rejected, &[]);
    let Some(catalogue) = owner.proficiency_catalogue.as_ref() else {
        return rejected();
    };
    let Some(binding) = owner.progression_binding(session) else {
        return rejected();
    };
    let definitions =
        catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;
    let resolved = {
        let runtime = owner.runtime.lock().await;
        runtime.player_control_facts(actor, session).ok()?;
        pinned_index(&runtime, owner.active_generation)
            .and_then(|index| index.definition_at(request.definition))
            .and_then(|(key, _)| definitions.resolve_weapon(key))
    };
    let Some(resolved) = resolved else {
        return rejected();
    };
    let item_key = resolved.canonical_item_key;
    let occurrence = occurrence(actor, session, command.command_id)?;
    // A level reached by an uncheckpointed kill is committed before it is selected. The saver
    // takes its own revision slot, so it runs before this command acquires one.
    if owner.save_weapon_proficiency(actor, session, false).await == TrainingSave::FencedOut {
        return None;
    }
    let current = owner.current_quest_fence(session).await.ok()??;
    let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
    let root = owner
        .root
        .read_current_character(owner.character, current.character_id)
        .await
        .ok()?;
    let fence = CurrentCharacterGameplayFence {
        expected_character_revision: root.revision,
        ..current
    };
    let (tracks, receipt) = owner
        .root
        .read_character_proficiency_with_definitions(
            owner.character,
            fence.character_id,
            Some(occurrence),
            Arc::clone(&definitions),
        )
        .await
        .ok()?;
    let stored = tracks
        .into_iter()
        .find(|track| track.state.item_key() == item_key);
    let selections = stored
        .as_ref()
        .map_or_else(Vec::new, |track| track.state.selections().to_vec());
    if let Some(receipt) = receipt {
        return reply(replayed(&receipt, &item_key, request), &selections);
    }
    let Some(stored) = stored else {
        return reply(SelectPerkResult::UnknownTrack, &[]);
    };
    let in_protection_zone = {
        let mut runtime = owner.runtime.lock().await;
        if runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
            || fence.runtime_scope
                != crate::foundation::RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
            || fence.scope_ownership_generation != runtime.binding().scope_generation()
        {
            return None;
        }
        super::actor_spell::qualified_protection_zone(&mut runtime, owner.movement_cells, actor)
            == Some(true)
    };
    let plan = plan_proficiency_selection(
        resolved.shape,
        &selections,
        ProficiencySelectionContext {
            committed_track_revision: stored.committed_character_revision,
            committed_progress: u32::try_from(stored.state.progress()).ok()?,
            in_protection_zone,
        },
        ProficiencySelectionCommand {
            expected_track_revision: stored.committed_character_revision,
            level: request.level,
            perk_index: request.perk,
        },
    );
    match plan {
        Err(error) => return reply(refusal(error), &selections),
        Ok(ProficiencySelectionPlan::NoChange) => {
            return reply(SelectPerkResult::Accepted, &selections);
        }
        Ok(ProficiencySelectionPlan::Changed { .. }) => {}
    }
    let mut after_selections = selections.clone();
    *after_selections.get_mut(usize::from(request.level))? = request.perk;
    let after = DurableProficiencyState::new(
        item_key.clone(),
        stored.state.definition_key(),
        stored.state.definition_revision(),
        stored.state.progress(),
        after_selections,
    )
    .ok()?;
    let change = ProficiencyChangeRequest::new(
        occurrence,
        ProficiencyCause::PerkSelection,
        vec![
            ProficiencyLineCandidate::new(
                ProficiencyCause::PerkSelection,
                stored.state.clone(),
                after,
            )
            .ok()?,
        ],
        Some(stored.committed_character_revision),
        definitions.digest(),
    )
    .ok()?;
    let outcome = slot
        .commit_proficiency(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            change.clone(),
            Arc::clone(&definitions),
            Some(fence.expected_character_revision),
        )
        .await;
    let receipt = match outcome {
        Ok(
            ProficiencyCommitOutcome::Committed(receipt)
            | ProficiencyCommitOutcome::AlreadyCommitted(receipt),
        ) => receipt,
        Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::ModifiedLevel)) => {
            return reply(SelectPerkResult::ModifiedLevel, &selections);
        }
        Ok(ProficiencyCommitOutcome::Refused(_))
        | Err(CharacterProgressionError::CharacterRevisionMismatch) => {
            return reply(SelectPerkResult::Busy, &selections);
        }
        Err(CharacterProgressionError::ConflictingOccurrence) => {
            return reply(SelectPerkResult::Conflict, &selections);
        }
        // An unknown outcome is reconciled by the occurrence; still unknown disconnects.
        Err(_) => owner
            .root
            .reconcile_character_proficiency_with_definitions(
                owner.character,
                fence.character_id,
                fence.expected_character_revision,
                change,
                Some(Arc::clone(&definitions)),
            )
            .await
            .ok()??,
    };
    drop(slot);
    let committed = StoredProficiencyTrack {
        state: receipt.lines.first()?.after().clone(),
        committed_character_revision: receipt.committed_character_revision,
        last_proficiency_occurrence_id: receipt.occurrence_id,
    };
    if let Some(track) = catalogue.live_track(&committed)
        && let Ok(mut states) = owner.states.proficiencies.lock()
    {
        states
            .entry(session)
            .or_default()
            .install_selections(committed.state.item_key(), track);
    }
    reply(SelectPerkResult::Accepted, committed.state.selections())
}

impl WeaponProficiencyDispatch {
    /// Replace one track's committed selections and effects. A selection never moves progress,
    /// so live progress and any pending checkpoint stay the session's.
    fn install_selections(&mut self, item_key: &str, track: LiveProficiencyTrack) {
        match self.tracks.get_mut(item_key) {
            Some(live) => {
                live.selections = track.selections;
                live.effects = track.effects;
            }
            None => self.install_track(item_key, track),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "unit fixtures construct valid tracks")]
mod tests {
    use super::*;
    use std::num::NonZeroU32;

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70 | (seed & 15);
        bytes[8] = 0x80 | (seed & 63);
        bytes
    }

    fn state(progress: u64, selections: Vec<Option<u8>>) -> DurableProficiencyState {
        DurableProficiencyState::new(
            "oteryn:item.tibia.i3264",
            "oteryn:proficiency.tibia.p1",
            "definition-r1",
            progress,
            selections,
        )
        .expect("state")
    }

    fn receipt(cause: ProficiencyCause, after: Vec<Option<u8>>) -> CommittedProficiencyChange {
        // A selection line keeps progress and changes a choice; a training line the reverse.
        let before = match cause {
            ProficiencyCause::PerkSelection => state(1_000, vec![None; after.len()]),
            _ => state(500, after.clone()),
        };
        CommittedProficiencyChange {
            character_id: crate::domain::CharacterId::from_bytes(uuid_v7(1)).expect("id"),
            occurrence_id: [2; 16],
            original_character_revision: crate::domain::CharacterRevision::new(1).expect("revision"),
            committed_character_revision: crate::domain::CharacterRevision::new(2)
                .expect("revision"),
            cause,
            command_binding: Vec::new(),
            policy_digest: [0; 32],
            lines: vec![ProficiencyLineCandidate::new(cause, before, state(1_000, after)).expect("line")],
        }
    }

    fn request(level: u8, perk: Option<u8>) -> SelectPerkRequest {
        SelectPerkRequest {
            definition: NonZeroU32::MIN,
            level,
            perk,
        }
    }

    #[test]
    fn planner_refusals_map_to_closed_command_results() {
        assert_eq!(
            refusal(ProficiencyError::SelectionNotUnlocked),
            SelectPerkResult::NotUnlocked
        );
        assert_eq!(
            refusal(ProficiencyError::InvalidSelectionLevel),
            SelectPerkResult::NotUnlocked
        );
        assert_eq!(
            refusal(ProficiencyError::InvalidSelectionPerk),
            SelectPerkResult::InvalidPerk
        );
        assert_eq!(
            refusal(ProficiencyError::ProtectionZoneRequired),
            SelectPerkResult::NotInProtectionZone
        );
        assert_eq!(
            refusal(ProficiencyError::StaleTrackRevision),
            SelectPerkResult::Busy
        );
    }

    #[test]
    fn a_replayed_occurrence_is_accepted_only_for_the_same_selection() {
        let committed = receipt(ProficiencyCause::PerkSelection, vec![Some(1), None]);
        let key = "oteryn:item.tibia.i3264";
        assert_eq!(
            replayed(&committed, key, request(0, Some(1))),
            SelectPerkResult::Accepted
        );
        assert_eq!(
            replayed(&committed, key, request(0, Some(0))),
            SelectPerkResult::Conflict
        );
        assert_eq!(
            replayed(&committed, "oteryn:item.tibia.i3265", request(0, Some(1))),
            SelectPerkResult::Conflict
        );
        assert_eq!(
            replayed(
                &receipt(ProficiencyCause::Training, vec![Some(1), None]),
                key,
                request(0, Some(1))
            ),
            SelectPerkResult::Conflict
        );
    }

    #[test]
    fn the_occurrence_is_bound_to_the_command_and_is_a_uuid_v7() {
        let actor = ExactActorRef::transport_fixture(
            crate::foundation::WorldId::decode(&uuid_v7(3)).expect("world"),
            crate::foundation::ChannelId::decode(&uuid_v7(4)).expect("channel"),
        );
        let session = GameSessionId::decode(&uuid_v7(5)).expect("session");
        let first = occurrence(actor, session, 7).expect("occurrence");
        assert_eq!(Some(first), occurrence(actor, session, 7));
        assert_ne!(Some(first), occurrence(actor, session, 8));
        assert_eq!(first.as_bytes()[6] >> 4, 7);
        assert_eq!(first.as_bytes()[8] >> 6, 2);
    }
}
