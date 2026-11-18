//! Crystal 201189f8 mind-travel.lua:21–39 and :117–143.
use super::*;
use crate::durability::{
    consume_transform::{
        ConsumeTransformRequest,
        mind::{MindAction, potion_landing},
    },
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, MovementPositionSnapshot};
use crate::map::view::{MapFacts, placement_key};

pub(super) use crate::durability::consume_transform::mind::POTION;

const LANDINGS: [MovementLocalPosition; 2] = [
    MovementLocalPosition {
        x: 33913,
        y: 31653,
        floor: 9,
    },
    MovementLocalPosition {
        x: 33921,
        y: 31650,
        floor: 9,
    },
];

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_mind_potion(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        use_command: UseCommand,
        fence: CurrentCharacterItemFence,
        native: &NativeGameplayState,
        source: [u8; 16],
    ) -> Result<UseDisposition, UseDisposition> {
        let refused = UseDisposition::NotPossible;
        let session = use_command.game_session_id;
        let command = CommandRef::new(
            session,
            CommandId::new(use_command.command_id).map_err(|_| refused)?,
        );
        let index = potion_landing(command, source).ok_or(refused)?;
        let destination = LANDINGS[index];
        let map = self.bundle_map().ok_or(refused)?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let proof = {
            let runtime = self.runtime.lock().await;
            let entered = runtime.read_actor_position(actor).map_err(|_| refused)?;
            let at = entered.position();
            if at.floor != 7 || at.x.abs_diff(33865) > 8 || at.y.abs_diff(31533) > 7 {
                return Err(refused);
            }
            let states = self.states.spell_states.lock().await;
            let blocked =
                super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
            crate::map::floor::prepare_quest_callback_teleport_facing(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                entered,
                destination,
                blocked,
                &closed,
                None,
            )
            .map_err(|_| refused)?
        };
        let request = ConsumeTransformRequest {
            command,
            source,
            target: None,
            intent: [b"mind-potion:".as_slice(), source.as_slice()].concat(),
        };
        let content = super::super::item_ref_admission::NativeItemWeights(native)
            .with_wheel(self.premium_coordinator);
        let consume = || {
            self.root.consume_mind_item(
                permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                MindAction::Potion,
            )
        };
        let (result, reconciling) = match consume().await {
            Ok(result) => (result, false),
            // Retry while holding the same actor lane and exact landing proof.
            // An acknowledged receipt can settle an uncertain first commit.
            Err(_) => (consume().await.map_err(|_| UseDisposition::Rejected)?, true),
        };
        let result = result.ok_or(refused)?;
        if result.disposition != UseDisposition::Committed {
            return Err(result.disposition);
        }
        if !result.fresh && !reconciling {
            return Ok(UseDisposition::Committed);
        }
        let mut runtime = self.runtime.lock().await;
        runtime
            .commit_source_step(proof)
            .map_err(|_| UseDisposition::Rejected)?;
        if let Some(at) = tile(destination) {
            let _ = map.facts.emit_movement_effect(at, 38);
        }
        self.states.chat.push_quest_feedback(
            session,
            "You feel dizzy. When your sight clears, you are somewhere else.",
        );
        Ok(UseDisposition::Committed)
    }

    pub(in crate::gameplay_transport) async fn step_mind_field(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) {
        let Some(map) = self.bundle_map() else { return };
        let Some(at) = tile(entered.position()) else {
            return;
        };
        if map.facts.house_tile(at) {
            return;
        }
        let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            return;
        };
        let count = base.depths().iter().filter(|&&depth| depth == 0).count();
        let on_field = (0..count)
            .filter_map(|ordinal| placement_key(at, u8::try_from(ordinal).ok()?))
            .any(|key| !map.facts.base_hidden(key) && map.facts.action_at(key) == Some(40004));
        if !on_field || self.runtime.lock().await.read_actor_position(actor) != Ok(entered) {
            return;
        }
        let Some(command) = super::super::quest_callbacks::step_command(
            actor,
            session,
            entered.revision(),
            self.reset_epoch,
        ) else {
            return;
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        let Some(native) = self
            .active_generation
            .and_then(ActiveGeneration::native_gameplay)
        else {
            return;
        };
        let intent = [
            b"mind-field:".as_slice(),
            actor.placement_identity().as_slice(),
            &entered.revision().to_be_bytes(),
            &self.reset_epoch.to_be_bytes(),
        ]
        .concat();
        let _ = self
            .root
            .consume_mind_item(
                permit,
                self.character,
                self.holder,
                fence.item_fence(),
                ConsumeTransformRequest {
                    command,
                    source: [0; 16],
                    target: None,
                    intent,
                },
                native,
                &super::super::item_ref_admission::NativeItemWeights(native)
                    .with_wheel(self.premium_coordinator),
                MindAction::Field,
            )
            .await;
    }
}

fn tile(position: MovementLocalPosition) -> Option<crate::map::overlay::TilePos> {
    Some(crate::map::overlay::TilePos {
        x: u16::try_from(position.x).ok()?,
        y: u16::try_from(position.y).ok()?,
        floor: i8::try_from(position.floor.checked_neg()?).ok()?,
    })
}
