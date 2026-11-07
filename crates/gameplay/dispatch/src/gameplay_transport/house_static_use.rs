//! Fixed House torches/ovens retain BASE lineage and the existing native map CAS.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    world_map::{MapTargetRefusal, MapViewSource, resolve_map_target},
};
use crate::{
    durability::consume_transform::HouseActionContext,
    foundation::{ChannelRuntimeV1, ExactActorRef},
    map::{boot::BundleMap, overlay::TilePos},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
struct Turn<'a> {
    map: &'a BundleMap,
    native: &'a crate::content::native_gameplay::NativeGameplayState,
    runtime: &'a tokio::sync::Mutex<ChannelRuntimeV1>,
    states: &'a tokio::sync::Mutex<super::actor_spell::ChannelSpellStates>,
    context: HouseActionContext,
    command: UseCommand,
    exact_actor: ExactActorRef,
    generation: [u8; 32],
    server_generation: [u8; 32],
    actor: TilePos,
    target: WorldObjectTarget,
    published: Option<UseDisposition>,
}
impl Turn<'_> {
    fn publish(&mut self) {
        let Ok(runtime) = self.runtime.try_lock() else {
            self.published = Some(UseDisposition::Exhausted);
            return;
        };
        let Some(fence) = self.command.item_fence else {
            self.published = Some(UseDisposition::Rejected);
            return;
        };
        let Ok(current) = HouseActionContext::from_native_owner(
            &runtime,
            self.exact_actor,
            self.command.game_session_id,
            fence,
        ) else {
            self.published = Some(UseDisposition::StaleState);
            return;
        };
        if current.house() != self.context.house()
            || current.position() != self.context.position()
            || runtime.content_pin().client_artifact_digest() != self.generation
            || runtime.content_pin().server_artifact_digest() != self.server_generation
            || !runtime
                .house_player_control_facts(self.exact_actor, self.command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            self.published = Some(UseDisposition::StaleState);
            return;
        }
        let Ok(states) = self.states.try_lock() else {
            self.published = Some(UseDisposition::Exhausted);
            return;
        };
        if states.is_dead(self.exact_actor) {
            self.published = Some(UseDisposition::Rejected);
            return;
        }
        self.published = Some(
            super::item_transform::use_house_static(
                self.map,
                self.native,
                self.generation,
                self.actor,
                &self.target,
            )
            .unwrap_or(UseDisposition::NothingToUse),
        );
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_house_static_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
    ) -> Result<Option<UseOutcome>, ()> {
        let UseTarget::WorldObject(target) = &intent.target else {
            return Ok(None);
        };
        if intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
        {
            return Ok(None);
        }
        let result = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(fence) = command.item_fence else {
            return Ok(result(UseDisposition::Rejected));
        };
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(result(UseDisposition::NotPossible));
        };
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let _lane = self.spell_lane_permit().await.ok_or(())?;
        let (context, generation, server_generation) = {
            let runtime = self.runtime.lock().await;
            let context = HouseActionContext::from_native_owner(
                &runtime,
                actor,
                command.game_session_id,
                fence,
            )
            .map_err(|_| ())?;
            if super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none() {
                return Ok(result(UseDisposition::StaleState));
            }
            if !runtime
                .house_player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Ok(result(UseDisposition::Rejected));
            }
            (
                context,
                runtime.content_pin().client_artifact_digest(),
                runtime.content_pin().server_artifact_digest(),
            )
        };
        if self.states.spell_states.lock().await.is_dead(actor) {
            return Ok(result(UseDisposition::Rejected));
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Ok(result(UseDisposition::Exhausted));
        }
        let position = {
            let facts = map.facts.pinned();
            let source = MapViewSource {
                overlay: &map.overlay,
                facts: &facts,
                content_generation: generation,
                reset_epoch: self.reset_epoch,
            };
            match resolve_map_target(&source, target, |_| None::<()>) {
                Ok(target) => target.pos,
                Err(MapTargetRefusal::StateRevisionMismatch) => {
                    return Ok(result(UseDisposition::StaleState));
                }
                Err(MapTargetRefusal::Use(disposition)) => return Ok(result(disposition)),
            }
        };
        let current = context.position();
        let actor_position = TilePos {
            x: current[0].try_into().map_err(|_| ())?,
            y: current[1].try_into().map_err(|_| ())?,
            floor: i8::try_from(current[2])
                .map_err(|_| ())?
                .checked_neg()
                .ok_or(())?,
        };
        let target_position = [
            i32::from(position.x),
            i32::from(position.y),
            i32::from(position.floor).checked_neg().ok_or(())?,
        ];
        if target_position[2] != current[2]
            || target_position[0].abs_diff(current[0]) > 1
            || target_position[1].abs_diff(current[1]) > 1
        {
            return Ok(result(UseDisposition::TooFar));
        }
        let mut turn = Turn {
            map,
            native,
            runtime: &self.runtime,
            states: &self.states.spell_states,
            context: context.clone(),
            command,
            exact_actor: actor,
            generation,
            server_generation,
            actor: actor_position,
            target: target.clone(),
            published: None,
        };
        // No runtime/spell-state mutex crosses this await. The synchronous callback
        // takes only nonblocking guards, reproves the frozen actor, and publishes CAS
        // while the DB still holds the exact current House/target-tile read locks.
        let admitted = self
            .root
            .with_house_static_source(
                self.character,
                self.holder,
                context,
                target_position,
                &mut turn,
                Turn::publish,
            )
            .await;
        if let Some(published) = turn.published {
            return Ok(result(published));
        }
        match admitted {
            Ok(false) => Ok(result(UseDisposition::Rejected)),
            Ok(true) => Err(()),
            Err(_) => Err(()),
        }
    }
}
