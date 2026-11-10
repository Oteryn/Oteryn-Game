//! Crystal201189f soul_war_mechanics.lua942-963,1014-1026.
//! Original33949/33984 Player callbacks consume the captured physical Item and
//! publish the existing torment counter/icon successor only after its known ACK.
use super::*;
use crate::durability::creature_source_items::{SourceGroundStep, SourceGroundStepKind};
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_encounters::bosses::soul_war_torment::PlayerCounter;

#[derive(Debug, Clone)]
pub(super) struct PendingMegalomania {
    input: SourceGroundStep,
    before: Arc<PlayerSpellState>,
    after: Arc<PlayerSpellState>,
    reset_message: bool,
}

fn prepare(
    runtime: &ChannelRuntimeV1,
    player: &PlayerSpellState,
    input: SourceGroundStep,
) -> Result<PendingMegalomania, ()> {
    if !input.current_for(runtime) {
        return Err(());
    }
    let mut counter = PlayerCounter::from_current(player.soul_war_torment_count()).ok_or(())?;
    let reset_message = match input.kind() {
        SourceGroundStepKind::DeadAspect => {
            let positive = counter.value() > 0;
            if positive {
                counter.reset();
            }
            positive
        }
        SourceGroundStepKind::Remains => {
            counter.remove(5);
            false
        }
        SourceGroundStepKind::Weeping => return Err(()),
    };
    // Dead Aspect at counter0 never invokes reset in Lua. Preserve any existing
    // hidden/visible icon DATA in that literal branch, rather than manufacturing
    // another counter transition from a matching Item definition.
    let after = if input.kind() == SourceGroundStepKind::DeadAspect && !reset_message {
        player.clone()
    } else {
        player
            .stage_soul_war_torment_counter(counter.value())
            .ok_or(())?
    };
    Ok(PendingMegalomania {
        input,
        before: Arc::new(player.clone()),
        after: Arc::new(after),
        reset_message,
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::expect_used,
        reason = "the original Player reservation and complete predecessor are validated before synchronous publication"
    )]
    pub(super) async fn resolve_one_megalomania_step(
        &self,
        permit: &SpellLanePermit,
        mut pending: PendingStep,
    ) -> Result<(), ()> {
        if pending.megalomania.is_none() {
            let input = pending
                .movement
                .source_ground_steps()
                .iter()
                .find(|input| !pending.completed_items.contains(&input.item_instance_id()))
                .cloned()
                .ok_or(())?;
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
            runtime
                .validate_source_actor_reservation(&pending.reservation, &pending.movement)
                .map_err(|_| ())?;
            let player = states
                .get(
                    &runtime,
                    pending.movement.actor(),
                    pending.movement.session(),
                )
                .ok_or(())?;
            let effect = prepare(&runtime, player, input)?;
            let entry = states
                .spite_steps
                .pending
                .iter_mut()
                .find(|entry| entry.movement.event_identity() == pending.movement.event_identity())
                .ok_or(())?;
            if entry.megalomania.is_some() || entry.weeping.is_some() {
                return Err(());
            }
            pending.megalomania = Some(effect);
            *entry = pending.clone();
        }
        let effect = pending.megalomania.as_ref().ok_or(())?;
        let outcome = self
            .root
            .commit_source_ground_step(permit, self.character, self.holder, &effect.input)
            .await
            .map_err(|_| ())?;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
        runtime
            .validate_source_actor_reservation(&pending.reservation, &pending.movement)
            .map_err(|_| ())?;
        let index = states
            .spite_steps
            .pending
            .iter()
            .position(|entry| entry.movement.event_identity() == pending.movement.event_identity())
            .ok_or(())?;
        if states.spite_steps.pending[index]
            .megalomania
            .as_ref()
            .is_none_or(|retained| retained.input.binding() != effect.input.binding())
            || states.get(
                &runtime,
                pending.movement.actor(),
                pending.movement.session(),
            ) != Some(effect.before.as_ref())
        {
            return Err(());
        }
        if outcome.removed {
            if effect.after.revision() != effect.before.revision() {
                states
                    .commit(
                        &runtime,
                        pending.movement.actor(),
                        pending.movement.session(),
                        effect.after.as_ref().clone(),
                    )
                    .then_some(())
                    .expect("prepared canonical torment counter/icon successor");
            }
            if let Some(room) = self.effective_rooms().gate_field_step_ingress() {
                super::super::soul_war_scene::emit_effect(
                    room,
                    pending.movement.movement().position(),
                    50,
                );
            }
            if effect.reset_message {
                self.states.server_messages.send_current(
                    pending.movement.session(),
                    ServerMessage::Advance(
                        "The ooze calms your dread but leaves you vulnerable to phantasmal attacks!"
                            .into(),
                    ),
                );
            }
            self.states.loop_ground.invalidate();
        }
        pending
            .completed_items
            .push(effect.input.item_instance_id());
        pending.megalomania = None;
        states.spite_steps.pending[index] = pending;
        Ok(())
    }
}
