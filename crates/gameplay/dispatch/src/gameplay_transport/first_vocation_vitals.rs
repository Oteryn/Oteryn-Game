//! First-vocation dispatch wiring and the existing ordered vitals projection.
use super::{ComposedFreshAdmission, connection, npc_command};
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_last_position::{LastPositionWrite, LastVitals};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::Vocation;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn vocation_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: NpcTalkIntent,
    ) -> npc_command::NpcTalkOutcome {
        let ordinary = self.newhaven_talk(actor, command, intent.clone()).await;
        if ordinary.disposition != NpcIntentDisposition::Ok {
            return ordinary;
        }
        self.finish_npc_vocation_choice(actor, command.game_session_id, &intent, ordinary)
            .await
    }

    pub(super) async fn save_first_vocation_vitals(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterGameplayFence,
        before: &DurableBuildState,
    ) -> bool {
        let placed = {
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            if states.is_dead(actor) || states.has_pending_spell_commit(actor, session) {
                return false;
            }
            let Some(state) = states.get(&runtime, actor, session) else {
                return false;
            };
            if state.character_facts().vocation != Vocation::None
                || state.character_facts().magic_level != u32::from(before.magic().0)
                || state.has_training_to_flush()
            {
                return false;
            }
            let Ok(position) = runtime.read_actor_position(actor) else {
                return false;
            };
            let vitals = state.vitals();
            let Some(vitals) = LastVitals::new(vitals.health, vitals.mana) else {
                return false;
            };
            let mut placed = self.placed_record(&runtime, position.position());
            // Explicit even at maxima: NULL would mean the *new* class's maxima on restart.
            placed.record.vitals = Some(vitals);
            placed
        };
        matches!(
            self.root
                .write_character_last_position(
                    self.character,
                    self.holder,
                    fence,
                    placed.write_sequence,
                    placed.record,
                )
                .await,
            Ok(LastPositionWrite::Written)
        )
    }
}
