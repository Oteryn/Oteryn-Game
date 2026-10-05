//! Durable stance casting joins the existing Character writer to the actual
//! Channel-owned player state. One retained occurrence survives unknown outcomes.
use super::super::ComposedFreshAdmission;
use super::{ChannelSpellStates, SpellCastIntent, SpellCastOutcome};
use crate::domain::CharacterId;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::character_stance::{CommittedCharacterStance, StanceChangeOutcome};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1,
};
use crate::spell::stance_execution::PreparedStance;
use crate::spell::{Execution, OperationalCastFacts, SpellDefinition};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};

fn stance_spell(spell: &SpellDefinition) -> bool {
    matches!(&spell.execution, Execution::NativeProfile(profile)
        if profile.spell()["execution"]["native_behavior"]["key"] == "stance_toggle")
}
fn current_owner(
    runtime: &ChannelRuntimeV1,
    fence: &CurrentCharacterGameplayFence,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    let binding = runtime.binding();
    fence.game_session_id == session
        && fence.runtime_scope
            == RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        && fence.scope_ownership_generation == binding.scope_generation()
        && runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
}
fn pending(
    states: &ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<PreparedStance> {
    states
        .get(runtime, actor, session)?
        .pending_stance()
        .cloned()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn reconcile_pending_stance_for_control_loss(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<SpellCastOutcome> {
        let original = {
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            pending(&states, &runtime, actor, session)
        }?;
        // Preparation admitted only this exact targetless stance form; the
        // retained occurrence, index and command are the original owner values.
        let intent = SpellCastIntent {
            spell: original.spell_index,
            target: SpellTarget::None,
            aim_at_target: false,
        };
        self.cast_durable_stance(actor, session, original.command_id, &intent)
            .await
    }

    async fn stance_fence(
        &self,
        session_id: GameSessionId,
    ) -> Option<CurrentCharacterGameplayFence> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (session, _) = store.current_session_at(session_id).await.ok()?;
        if session.session_state() != GameSessionState::Active {
            return None;
        }
        let character_id =
            CharacterId::from_bytes(*session.commit().character_id().as_bytes()).ok()?;
        let record = self
            .root
            .read_current_character(self.character, character_id)
            .await
            .ok()?;
        Some(CurrentCharacterGameplayFence {
            character_id,
            game_session_id: session_id,
            connection_generation: session.current_connection_generation(),
            character_lease_generation: session.current_character_lease().generation(),
            runtime_scope: session.current_runtime_scope(),
            scope_ownership_generation: session.current_scope_generation(),
            expected_character_revision: record.revision,
        })
    }
    /// `None` delegates a non-stance command to the usual caster. A retained
    /// stance always reconciles its original occurrence before any new command.
    pub(in crate::gameplay_transport) async fn cast_durable_stance(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
    ) -> Option<SpellCastOutcome> {
        let retained = {
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            pending(&states, &runtime, actor, session)
        };
        let prepared = if let Some(retained) = retained {
            retained
        } else {
            let (spell, active) = self.spells.source_indexed(intent.spell)?;
            if !stance_spell(spell) {
                return None;
            }
            if !active
                || intent.target != SpellTarget::None
                || intent.aim_at_target
                || command_id == 0
            {
                return Some(SpellCastOutcome::rejected());
            }
            let Some(fence) = self.stance_fence(session).await else {
                return Some(SpellCastOutcome::rejected());
            };
            // Bracket all durable policy/facts/projection reads with the root revision.
            let Ok(Some(progression)) = self
                .root
                .read_character_progression(self.character, fence.character_id)
                .await
            else {
                return Some(SpellCastOutcome::rejected());
            };
            let Ok(loaded) = self
                .root
                .read_character_stance(self.character, fence.character_id)
                .await
            else {
                return Some(SpellCastOutcome::rejected());
            };
            let super::super::spell_character_facts::CastFactsLoad::Ready {
                facts,
                character_revision,
            } = super::super::spell_character_facts::load_character_cast_facts(
                self.root,
                self.character,
                fence.character_id,
            )
            .await
            else {
                return Some(SpellCastOutcome::rejected());
            };
            if character_revision != fence.expected_character_revision
                || progression.character_revision > character_revision
            {
                return Some(SpellCastOutcome::rejected());
            }
            let now = self.owner_now();
            let mut runtime = self.runtime.lock().await;
            let mut states = self.spell_states.lock().await;
            if !current_owner(&runtime, &fence, actor, session) {
                return Some(SpellCastOutcome::rejected());
            }
            let Ok(position) = runtime.borrow_movement_position().read(actor) else {
                return Some(SpellCastOutcome::rejected());
            };
            let Some(state) = states.get_mut(&runtime, actor, session) else {
                return Some(SpellCastOutcome::rejected());
            };
            if state.character_facts() != facts || state.durable_stance_key() != loaded.key() {
                return Some(SpellCastOutcome::rejected());
            }
            let position = position.position();
            // The closed stance profile has no target, direction, aggressive,
            // tile or Wheel requirement. These unused ports grant no authority.
            let operational = OperationalCastFacts {
                caster_position: crate::spell::chain::TilePosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                target_position: None,
                target: None,
                line_of_sight_clear: None,
                direction_available: false,
                wheel_unlocked: None,
                in_protection_zone: false,
                target_tile_solid: None,
                target_tile_creature: None,
            };
            let paid = match crate::spell::cast::prepare_native_owner_cast(
                state,
                spell,
                &operational,
                crate::spell::native::Facts::Stance {
                    active: state.standard_stance(),
                    vocation: facts.vocation,
                },
                now,
                &mut |_, _| 0,
            ) {
                Ok(paid) => paid,
                Err(disposition) => {
                    return Some(SpellCastOutcome {
                        disposition,
                        vitals: None,
                    });
                }
            };
            let prepared = match PreparedStance::new(
                state,
                paid,
                spell,
                actor,
                session,
                command_id,
                intent.spell,
                fence,
                progression.context.content,
                progression.policy_revision,
                now,
            ) {
                Ok(prepared) => prepared,
                Err(_) => return Some(SpellCastOutcome::rejected()),
            };
            if state.reserve_stance(prepared.clone()).is_err() {
                return Some(SpellCastOutcome::rejected());
            }
            prepared
        };
        let (spell, active) = self.spells.source_indexed(prepared.spell_index)?;
        if !active || !stance_spell(spell) {
            return Some(SpellCastOutcome::rejected());
        }
        // No Channel owner locks cross a durable await. Reissue only the same
        // request/fence; the writer's receipt key makes this idempotent.
        let receipt = match self
            .root
            .reconcile_character_stance(self.character, prepared.request.occurrence)
            .await
        {
            Ok(Some(receipt)) => receipt,
            Ok(None) => match self
                .root
                .commit_character_stance(
                    self.character,
                    self.holder,
                    prepared.fence,
                    prepared.request.clone(),
                )
                .await
            {
                Ok(
                    StanceChangeOutcome::Committed(receipt)
                    | StanceChangeOutcome::AlreadyCommitted(receipt),
                ) => receipt,
                Err(CharacterProgressionError::Unavailable(_)) => {
                    let Ok(Some(receipt)) = self
                        .root
                        .reconcile_character_stance(self.character, prepared.request.occurrence)
                        .await
                    else {
                        return Some(SpellCastOutcome::rejected());
                    };
                    receipt
                }
                Err(_) => {
                    let runtime = self.runtime.lock().await;
                    let mut states = self.spell_states.lock().await;
                    if let Some(state) = states.get_mut(&runtime, actor, session) {
                        state.cancel_stance(&prepared);
                    }
                    return Some(SpellCastOutcome::rejected());
                }
            },
            Err(_) => return Some(SpellCastOutcome::rejected()),
        };
        Some(
            self.finish_stance(&prepared, &receipt, spell, actor, session, command_id)
                .await,
        )
    }
    async fn finish_stance(
        &self,
        prepared: &PreparedStance,
        receipt: &CommittedCharacterStance,
        spell: &SpellDefinition,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
    ) -> SpellCastOutcome {
        if !prepared.matches_receipt(receipt) {
            return SpellCastOutcome::rejected();
        }
        let Some(current) = self.stance_fence(session).await else {
            return SpellCastOutcome::rejected();
        };
        // A receipt from an old connection, lease or owner is historical only.
        let mut original = prepared.fence;
        original.expected_character_revision = receipt.committed_character_revision();
        if current != original {
            return SpellCastOutcome::rejected();
        }
        let Ok(loaded) = self
            .root
            .read_character_stance(self.character, current.character_id)
            .await
        else {
            return SpellCastOutcome::rejected();
        };
        if loaded.key() != receipt.after()
            || loaded.committed_character_revision() != Some(receipt.committed_character_revision())
        {
            return SpellCastOutcome::rejected();
        }
        if self.stance_fence(session).await != Some(current) {
            return SpellCastOutcome::rejected();
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if !current_owner(&runtime, &current, actor, session) {
            return SpellCastOutcome::rejected();
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return SpellCastOutcome::rejected();
        };
        if state
            .commit_stance_receipt(prepared, receipt, spell, self.owner_now())
            .is_err()
        {
            return SpellCastOutcome::rejected();
        }
        SpellCastOutcome {
            disposition: if prepared.matches_command(actor, session, command_id) {
                SpellCastDisposition::Cast
            } else {
                SpellCastDisposition::Rejected
            },
            vitals: Some((state.revision(), state.vitals())),
        }
    }
}
