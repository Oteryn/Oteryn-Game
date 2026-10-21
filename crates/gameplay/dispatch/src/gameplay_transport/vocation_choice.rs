//! D1103: Dawnport NPC confirmation through the existing Character build writer.
//!
//! The request builder preserves cumulative progress. The native caller uses the admitted
//! per-session NPC conversation, saves actual pending training, reads current build/stance,
//! retains one occurrence and commits through the Character revision slot. A historical
//! receipt never substitutes for independently current session/actor authority.
//! Nonempty stance fit and Dawnport caps remain specifically unqualified; no fit, cap,
//! departure destination or world-start coordinate is inferred here.

use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildFormula, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::character_progression::CharacterProgressionError;

/// Always derive `after` using the same cumulative-progress conversion the writer verifies
/// (SKILLS-0 §3.3), never by copying raw levels/tries or resetting progress. A formula's `None`
/// keeps its existing meaning, unreachable; it is not a vocation-admission decision.
///
/// `pruned_stance` is the owner's already-qualified decision: `None` leaves the stored stance
/// alone, `Some` names the stored key to prune in this same build receipt. No extra stance
/// receipt is created. The occurrence is preserved for the writer's existing replay contract.
#[allow(
    dead_code,
    reason = "D1220: A2 native NPC caller needs the cumulative conversion request; D1173 caller integration is pending."
)]
pub fn vocation_choice_request(
    occurrence: BuildOccurrence,
    before: DurableBuildState,
    selected_vocation: &str,
    formula: &dyn BuildFormula,
    pruned_stance: Option<String>,
) -> Result<BuildChangeRequest, CharacterProgressionError> {
    let after = convert_vocation(formula, &before, selected_vocation)?;
    Ok(BuildChangeRequest {
        occurrence,
        cause: BuildCause::VocationChoice,
        before,
        after,
        pruned_stance,
    })
}

// D1103 native owner composition in gameplay_transport.
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_build::{BuildCommitOutcome, CommittedBuildChange, NO_VOCATION};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_revision_sequencer::RevisionSlot;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1,
};
use crate::gameplay_transport::actor_spell::TrainingSave;
use crate::gameplay_transport::chat_intent::ChatRuntime;
use crate::gameplay_transport::npc_command::NpcTalkOutcome;
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::gameplay_transport::{ComposedFreshAdmission, SecureIdentifiers};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent, TALK_RANGE_TILES};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use oteryn_gameplay_npc::vocation_choice::answer;

pub const DAWNPORT_CHOICE_NPC: &str = "oteryn:npc.oressa";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedNpcVocation {
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub fence: CurrentCharacterGameplayFence,
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub request: BuildChangeRequest,
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub formula_digest: [u8; 32],
}

pub type ConfirmedNpcVocation = oteryn_gameplay_npc::vocation_choice::ConfirmedNpcVocation<
    BuildOccurrence,
    RetainedNpcVocation,
>;
pub type NpcVocationConversation = oteryn_gameplay_npc::vocation_choice::NpcVocationConversation<
    BuildOccurrence,
    RetainedNpcVocation,
>;

impl oteryn_gameplay_npc::vocation_choice::NpcDialogueOwner for ChatRuntime {
    fn converse<R>(
        &self,
        session: GameSessionId,
        f: impl FnOnce(&mut Option<ExactActorRef>) -> R,
    ) -> Option<R> {
        ChatRuntime::converse(self, session, f)
    }

    fn engaged(&self, session: GameSessionId) -> Option<ExactActorRef> {
        ChatRuntime::engaged(self, session)
    }

    fn end_engagement(&self, session: GameSessionId, npc: ExactActorRef) {
        ChatRuntime::end_engagement(self, session, npc);
    }
}

/// Called only after the existing NPC owner applies rate, exact NPC-generation and range checks.
#[allow(
    clippy::too_many_arguments,
    reason = "native NPC owner context after its rate/generation/range admission"
)]
pub fn dawnport_choice_dialogue(
    topic: &mut Option<NpcVocationConversation>,
    chat: &ChatRuntime,
    actor: ExactActorRef,
    session: GameSessionId,
    npc: EntityRef,
    npc_actor: ExactActorRef,
    text: &str,
) -> NpcTalkOutcome {
    oteryn_gameplay_npc::vocation_choice::dawnport_choice_dialogue(
        topic,
        chat,
        actor,
        session,
        npc,
        npc_actor,
        text,
        || {
            SecureIdentifiers::random::<16>().and_then(|mut bytes| {
                bytes[6] = (bytes[6] & 15) | 0x70;
                bytes[8] = (bytes[8] & 63) | 0x80;
                BuildOccurrence::from_bytes(bytes).ok()
            })
        },
    )
}

#[allow(
    dead_code,
    reason = "D1220: A2 native NPC caller must recheck current actor and NPC ownership; D1173 integration is pending."
)]
fn current_npc_owner(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    choice: &ConfirmedNpcVocation,
    fence: &CurrentCharacterGameplayFence,
) -> bool {
    let binding = runtime.binding();
    if choice.actor != actor
        || fence.game_session_id != session
        || runtime.owner_fence().is_err()
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || fence.runtime_scope
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || fence.scope_ownership_generation != binding.scope_generation()
    {
        return false;
    }
    let visible = runtime.visible_entities();
    let Some(npc) = visible.npcs.iter().find(|npc| {
        npc.actor.placement_identity() == choice.npc.identity
            && npc.generation == choice.npc.generation
    }) else {
        return false;
    };
    let Some(player) = visible.players.iter().find(|player| player.actor == actor) else {
        return false;
    };
    runtime.npc_key(npc.actor) == Some(DAWNPORT_CHOICE_NPC)
        && owner.chat.engaged(session) == Some(npc.actor)
        && player.position.floor == npc.position.floor
        && player.position.x.abs_diff(npc.position.x) <= TALK_RANGE_TILES
        && player.position.y.abs_diff(npc.position.y) <= TALK_RANGE_TILES
}

#[allow(
    dead_code,
    reason = "D1220: A2 native NPC commit must compare current session lease and scope; D1173 integration is pending."
)]
fn same_authority(a: &CurrentCharacterGameplayFence, b: &CurrentCharacterGameplayFence) -> bool {
    a.character_id == b.character_id
        && a.game_session_id == b.game_session_id
        && a.connection_generation == b.connection_generation
        && a.character_lease_generation == b.character_lease_generation
        && a.runtime_scope == b.runtime_scope
        && a.scope_ownership_generation == b.scope_ownership_generation
}

/// Shared by the actual NPC caller and its PostgreSQL qualification. Only real owners accepted.
/// Retained expected revision and intent are never rebuilt from a newer build on replay.
#[allow(
    clippy::too_many_arguments,
    reason = "existing real writer inputs plus retained intent and per-character slot"
)]
#[allow(
    dead_code,
    reason = "D1220: A2 native NPC caller needs this actual retained-intent build writer; D1173 integration is pending."
)]
pub async fn commit_retained_npc_vocation(
    slot: &mut RevisionSlot,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    retained: &RetainedNpcVocation,
    formula: &dyn BuildFormula,
) -> Result<CommittedBuildChange, CharacterProgressionError> {
    if formula.digest() != retained.formula_digest {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let receipt = match slot
        .commit_build(
            root,
            authority,
            node,
            retained.fence,
            retained.request.clone(),
            formula,
            Some(retained.fence.expected_character_revision),
        )
        .await
    {
        Ok(
            BuildCommitOutcome::Committed(receipt) | BuildCommitOutcome::AlreadyCommitted(receipt),
        ) => receipt,
        Err(error @ CharacterProgressionError::Unavailable(_)) => match root
            .reconcile_character_build(authority, retained.request.occurrence)
            .await
        {
            Ok(Some(receipt)) => receipt,
            _ => return Err(error),
        },
        Err(error) => return Err(error),
    };
    if receipt.character_id != retained.fence.character_id
        || receipt.occurrence != retained.request.occurrence
        || receipt.cause != BuildCause::VocationChoice
        || receipt.original_character_revision != retained.fence.expected_character_revision
        || receipt.before != retained.request.before
        || receipt.after != retained.request.after
        || receipt.pruned_stance != retained.request.pruned_stance
    {
        return Err(CharacterProgressionError::ConflictingOccurrence);
    }
    Ok(receipt)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC finish path needs this independent current fence read; D1173 integration is pending."
    )]
    async fn npc_vocation_fence(
        &self,
        session: GameSessionId,
    ) -> Option<CurrentCharacterGameplayFence> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = store.current_session_at(session).await.ok()?;
        if current.session_state() != GameSessionState::Active {
            return None;
        }
        let character_id =
            CharacterId::from_bytes(*current.commit().character_id().as_bytes()).ok()?;
        let root = self
            .root
            .read_current_character(self.character, character_id)
            .await
            .ok()?;
        Some(CurrentCharacterGameplayFence {
            character_id,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: root.revision,
        })
    }

    /// Runs only after native NPC admission and explicit confirmation. The CP caller hunk invokes
    /// this after dropping the runtime lock. No runtime/spell/NPC lock crosses the build write.
    #[allow(
        dead_code,
        reason = "D1220: A2 NPC dispatch is held under D1173; preserve the real native finish operation."
    )]
    pub async fn finish_npc_vocation_choice(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &NpcTalkIntent,
        fallback: NpcTalkOutcome,
    ) -> NpcTalkOutcome {
        use NpcIntentDisposition as D;
        let Some(choice) = self.npc_conversations.dawnport_choice(session) else {
            return fallback;
        };
        let reject = |line: &str| answer(choice.npc, D::Rejected, line);
        if choice.actor != actor || choice.npc != intent.npc_actor {
            return reject("The confirmed NPC is no longer current.");
        }
        let Some(formula) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
        else {
            return reject("Vocation conversion is temporarily unavailable.");
        };
        let Some(initial_fence) = self.npc_vocation_fence(session).await else {
            return reject("Your session is no longer current.");
        };
        let (retained, _slot) = if let Some(retained) = choice.retained.clone() {
            let mut slot = self
                .revision_sequencer
                .acquire(retained.fence.character_id)
                .await;
            if let Err(error) = commit_retained_npc_vocation(
                &mut slot,
                self.root,
                self.character,
                self.holder,
                &retained,
                formula,
            )
            .await
            {
                if matches!(error, CharacterProgressionError::CharacterRevisionMismatch) {
                    self.npc_conversations
                        .rollback_dawnport_request(session, choice.occurrence);
                }
                return reject("The choice is still being checked. Please try again.");
            }
            (retained, slot)
        } else {
            match self.save_spell_training(actor, session, true).await {
                TrainingSave::Saved | TrainingSave::NotApplicable => {}
                TrainingSave::Unknown => {
                    return reject("Your training is still being saved. Please try again.");
                }
                TrainingSave::FencedOut => return reject("Your session is no longer current."),
            }
            let Some(fence) = self.npc_vocation_fence(session).await else {
                return reject("Your session is no longer current.");
            };
            if !same_authority(&initial_fence, &fence) {
                return reject("Your session changed while training was saved.");
            }
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let Ok(before) = self
                .root
                .read_character_build_state(self.character, fence.character_id)
                .await
            else {
                return reject("Your build is temporarily unavailable.");
            };
            let Ok(stance) = self
                .root
                .read_character_stance(self.character, fence.character_id)
                .await
            else {
                return reject("Your stance is temporarily unavailable.");
            };
            let Ok(Some(progress)) = self
                .root
                .read_character_progression(self.character, fence.character_id)
                .await
            else {
                return reject("Your progress is temporarily unavailable.");
            };
            let Some(current) = self.npc_vocation_fence(session).await else {
                return reject("Your session is no longer current.");
            };
            if !same_authority(&fence, &current)
                || current.expected_character_revision != fence.expected_character_revision
                || progress.character_revision > current.expected_character_revision
                || progress.level < 8
                || before.vocation() != NO_VOCATION
            {
                return reject("Reach level 8 before making your first vocation choice.");
            }
            // No stance fit is guessed. A nonempty stance is the specifically blocked subcase.
            if stance.key().is_some() {
                return reject("This choice needs a qualified stance-fit rule.");
            }
            {
                let runtime = self.runtime.lock().await;
                let states = self.spell_states.lock().await;
                if !current_npc_owner(self, &runtime, actor, session, &choice, &current)
                    || states.has_pending_spell_commit(actor, session)
                {
                    return reject("Your current action must finish first.");
                }
                // Native admission currently creates no PlayerSpellState for NoVocation.
                // Never overwrite an existing state or drop newly accrued training here.
                if states.get(&runtime, actor, session).is_some() {
                    return reject("The existing live build must be settled by its owner first.");
                }
            }
            let Ok(request) =
                vocation_choice_request(choice.occurrence, before, choice.selected, formula, None)
            else {
                return reject("This vocation cannot be converted with the current content.");
            };
            let retained = RetainedNpcVocation {
                fence: current,
                request,
                formula_digest: formula.digest(),
            };
            if !self.npc_conversations.retain_dawnport_choice(
                session,
                choice.occurrence,
                retained.clone(),
            ) {
                return reject("The confirmed choice is no longer current.");
            }
            let receipt = commit_retained_npc_vocation(
                &mut slot,
                self.root,
                self.character,
                self.holder,
                &retained,
                formula,
            )
            .await;
            if let Err(error) = receipt {
                if matches!(error, CharacterProgressionError::CharacterRevisionMismatch) {
                    self.npc_conversations
                        .rollback_dawnport_request(session, choice.occurrence);
                }
                return reject("The choice is still being checked. Please try again.");
            }
            (retained, slot)
        };
        let CastFactsLoad::Ready {
            facts,
            character_revision,
        } = load_character_cast_facts(self.root, self.character, retained.fence.character_id).await
        else {
            return reject("Your committed choice is waiting for current character facts.");
        };
        let Ok(build) = self
            .root
            .read_character_build_state(self.character, retained.fence.character_id)
            .await
        else {
            return reject("Your committed build is temporarily unavailable.");
        };
        let Ok(stance) = self
            .root
            .read_character_stance(self.character, retained.fence.character_id)
            .await
        else {
            return reject("Your committed stance is temporarily unavailable.");
        };
        let Some(current) = self.npc_vocation_fence(session).await else {
            return reject("The choice committed; your session is no longer current.");
        };
        if !same_authority(&retained.fence, &current)
            || current.expected_character_revision != character_revision
            || build != retained.request.after
            || stance.key().is_some()
        {
            return reject("The choice committed; current character state must be reloaded.");
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if !current_npc_owner(self, &runtime, actor, session, &choice, &current) {
            return reject("The choice committed; current actor authority changed.");
        }
        if let Some(state) = states.get(&runtime, actor, session) {
            // An exact already-installed replay is harmless; a different state is never reset.
            if state.character_facts() != facts || state.has_training_to_flush() {
                return reject("Your live build must be settled by its owner first.");
            }
        } else if states
            .initialize(&runtime, actor, session, facts, (0, 0), self.owner_now())
            .is_none()
        {
            return reject("The choice committed; your actor is waiting for initialization.");
        }
        if !states.load_owned_build(&runtime, actor, session, &build)
            || !states.load_owned_stance(&runtime, actor, session, &stance, self.spells)
            || states
                .get_mut(&runtime, actor, session)
                .is_none_or(|state| state.enable_owned_training(&build, formula).is_err())
        {
            return reject("The choice committed; your actor is waiting for its owned build.");
        }
        self.npc_conversations
            .complete_dawnport_choice(session, choice.occurrence);
        answer(
            choice.npc,
            D::Ok,
            &format!("Your vocation is now {}.", choice.selected),
        )
    }
}

// Exercise the existing request cases against this actual private adapter.
#[cfg(test)]
#[path = "../../../../../apps/game-server/tests/support/dawnport_vocation_cases.rs"]
mod dawnport_vocation_cases;

#[cfg(test)]
mod dawnport_native_receipt_postgres {
    //! Durable qualification of the existing choice adapter, not native NPC admission.
    //! The fixture has no stance; no unknown stance-fit, island or departure rule is inferred.
    //! Training is committed through the real sequencer before current build/stance are read.

    use crate::{domain, durability};

    use crate::bestiary_postgres_harness;

    use crate::gameplay_transport::vocation_choice::{
        RetainedNpcVocation, commit_retained_npc_vocation, vocation_choice_request,
    };

    use bestiary_postgres_harness::{
        CHARACTER, Harness, TestResult, configured_admin, debug, fence, id, runtime,
    };
    use domain::CharacterId;
    use durability::DurabilityRoot;
    use durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
        DurableBuildState, MAX_BUILD_LEVEL, MIN_SKILL_LEVEL, NO_VOCATION, cumulative_progress,
    };
    use durability::character_progression::CharacterProgressionError;
    use durability::character_revision_sequencer::CharacterRevisionSequencer;

    /// Synthetic formula data exercises re-levelling of all eight families. It is not a
    /// qualified Reference formula or evidence for native Dawnport availability.
    struct Table;

    impl BuildFormula for Table {
        fn required(&self, vocation: &str, family: usize, level: u16) -> Option<u64> {
            let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
            if family > 7 || !(floor + 1..=MAX_BUILD_LEVEL).contains(&level) {
                return None;
            }
            let base = match vocation {
                NO_VOCATION => 100,
                "knight" => 50,
                _ => return None,
            };
            Some(base * (family as u64 + 1))
        }

        fn digest(&self) -> [u8; 32] {
            [7; 32]
        }

        fn content_revision(&self) -> &str {
            "content-1"
        }
    }

    fn assert_progress_preserved(before: &DurableBuildState, after: &DurableBuildState) {
        let old = std::iter::once(before.magic()).chain(before.skills());
        let new = std::iter::once(after.magic()).chain(after.skills());
        for (family, ((old_level, old_progress), (new_level, new_progress))) in
            old.zip(new).enumerate()
        {
            let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
            assert_eq!(
                cumulative_progress(old_level, old_progress, floor, |level| {
                    Table.required(before.vocation(), family, level)
                }),
                cumulative_progress(new_level, new_progress, floor, |level| {
                    Table.required(after.vocation(), family, level)
                }),
                "family {family}"
            );
        }
    }

    #[test]
    fn native_npc_receipt_core_preserves_training_and_replays_after_reopen() -> TestResult {
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        runtime()?.block_on(async move {
            let harness = Harness::create(admin, "dawnport_choice", true).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
            let sequencer = CharacterRevisionSequencer::new();
            let mut slot = sequencer.acquire(character).await;
            let seed = harness
                .root
                .read_character_build_state(&authority, character)
                .await
                .map_err(debug)?;
            let trained = DurableBuildState::new(NO_VOCATION, (3, 17), [(12, 3); 7]).map_err(debug)?;
            let training = BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(80)).map_err(debug)?,
                cause: BuildCause::Training,
                before: seed.clone(),
                after: trained.clone(),
                pruned_stance: None,
            };
            let outcome = slot
                .commit_build(
                    &harness.root,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    training,
                    &Table,
                    None,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, BuildCommitOutcome::Committed(_)));

            // A pre-flush build cannot discard the newly committed training, even with the
            // current revision. This negative changes only the build predecessor.
            let occurrence = BuildOccurrence::from_bytes(id(81)).map_err(debug)?;
            let stale =
                vocation_choice_request(occurrence, seed, "knight", &Table, None).map_err(debug)?;
            let outcome = slot
                .commit_build(
                    &harness.root,
                    &authority,
                    &harness.node,
                    fence(2)?,
                    stale,
                    &Table,
                    None,
                )
                .await;
            assert!(matches!(
                outcome,
                Err(CharacterProgressionError::BuildStateMismatch)
            ));
            assert_eq!(harness.root_revision().await?, "2");
            assert_eq!(harness.count("game_character_build_receipts").await?, 1);

            let before = harness
                .root
                .read_character_build_state(&authority, character)
                .await
                .map_err(debug)?;
            let stance = harness
                .root
                .read_character_stance(&authority, character)
                .await
                .map_err(debug)?;
            assert_eq!(before, trained);
            assert_eq!(stance, Default::default());
            let choice =
                vocation_choice_request(occurrence, before, "knight", &Table, None).map_err(debug)?;
            let retained = RetainedNpcVocation {
                fence: fence(2)?,
                request: choice.clone(),
                formula_digest: Table.digest(),
            };
            let receipt = commit_retained_npc_vocation(
                &mut slot,
                &harness.root,
                &authority,
                &harness.node,
                &retained,
                &Table,
            )
            .await
            .map_err(debug)?;
            assert_eq!(receipt.original_character_revision.get(), 2);
            assert_eq!(receipt.committed_character_revision.get(), 3);
            assert_eq!(receipt.before, trained);
            assert_eq!(receipt.after, choice.after);
            assert_ne!(receipt.before.magic(), receipt.after.magic());
            assert_progress_preserved(&receipt.before, &receipt.after);
            drop(slot);
            drop(authority);

            // A new root and sequencer have no cached cursor or choice state. Admission verifies
            // the persisted chain; reconcile recovers a response lost after the original commit.
            let restarted = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
            assert!(restarted.maintain_ready_once().await?);
            let reopened = restarted
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            assert_eq!(
                restarted
                    .reconcile_character_build(&reopened, occurrence)
                    .await
                    .map_err(debug)?,
                Some(receipt.clone())
            );
            let sequencer = CharacterRevisionSequencer::new();
            let mut slot = sequencer.acquire(character).await;
            let replay = commit_retained_npc_vocation(
                &mut slot,
                &restarted,
                &reopened,
                &harness.node,
                &retained,
                &Table,
            )
            .await
            .map_err(debug)?;
            assert_eq!(replay, receipt);
            assert_eq!(
                restarted
                    .read_character_build_state(&reopened, character)
                    .await
                    .map_err(debug)?,
                receipt.after
            );
            // The same occurrence with a changed intent conflicts, including after reopen.
            let mut conflicting = choice;
            conflicting.pruned_stance = Some("guard".into());
            let conflicting = RetainedNpcVocation {
                request: conflicting,
                ..retained
            };
            let outcome = commit_retained_npc_vocation(
                &mut slot,
                &restarted,
                &reopened,
                &harness.node,
                &conflicting,
                &Table,
            )
            .await;
            assert!(matches!(
                outcome,
                Err(CharacterProgressionError::ConflictingOccurrence)
            ));
            assert_eq!(harness.root_revision().await?, "3");
            assert_eq!(harness.count("game_character_build_receipts").await?, 2);
            let choices: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_build_receipts WHERE cause = 'vocation_choice'",
            )
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(choices, 1);
            assert_eq!(harness.count("game_character_stance_receipts").await?, 0);
            assert_eq!(
                restarted
                    .read_character_stance(&reopened, character)
                    .await
                    .map_err(debug)?,
                stance
            );
            drop(slot);
            drop(reopened);
            drop(seal);
            harness.cleanup().await
        })
    }
}
