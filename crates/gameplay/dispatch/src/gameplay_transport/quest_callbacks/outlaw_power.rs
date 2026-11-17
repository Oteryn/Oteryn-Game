//! Crystal Outlaw Camp lever 30032 moves the exact deposited ring through Ground custody.
use super::*;
use crate::durability::item_loop_move::{
    LoopDestination, LoopGround, LoopMoveReplay, LoopMoveRequest,
};
use crate::gameplay_transport::{
    item_ref_admission::{NativeItemWeights, pinned_index},
    loop_move,
};
use crate::map::{overlay::TilePos, view::EntryFacts};

pub(super) const SWITCH: &str = "oteryn:quest-callback/authored-outlaw-camp-power-switch";
const RING: &str = "oteryn:item.tibia.i3050";
const PEDESTAL: TilePos = TilePos {
    x: 32594,
    y: 32214,
    floor: -9,
};
const RECEIVING_PEDESTAL: TilePos = TilePos {
    x: 32613,
    y: 32220,
    floor: -10,
};
const TELEPORT_EFFECT: u16 = 11;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "fenced lever, actor and exact Ground item"
    )]
    pub(super) async fn execute_outlaw_power_switch(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        after: EntryFacts,
        close: bool,
        fence: Option<CurrentCharacterGameplayFence>,
        cause: QuestCause,
    ) -> UseDisposition {
        let toggle = |projection: Option<&crate::map::facts::QuestCallbackProjection>| {
            if map
                .facts
                .apply_quest_callback(key, revision, after, &[], &[], close, projection)
            {
                UseDisposition::Committed
            } else {
                UseDisposition::StaleState
            }
        };
        if close || !world_requirements_match(map, callback) {
            return toggle(None);
        }
        let (Some(fence), QuestCause::Use(command), Some(native), Some(index)) = (
            fence,
            cause,
            self.active_generation.and_then(|g| g.native_gameplay()),
            pinned_index(runtime, self.active_generation),
        ) else {
            return UseDisposition::Rejected;
        };
        let Ok(Some(projection)) = self.prepare_quest_scene(map, callback, false) else {
            return UseDisposition::Rejected;
        };
        let Some(ground) = self
            .states
            .loop_ground
            .current_for(runtime, map.overlay.base())
        else {
            return UseDisposition::Rejected;
        };
        let replay = self
            .root
            .read_item_loop_move_receipt(self.character, fence.character_id, command)
            .await;
        let already_moved = match replay {
            Ok(LoopMoveReplay::Moved) => true,
            Ok(LoopMoveReplay::Absent) => false,
            _ => return UseDisposition::Rejected,
        };
        if !already_moved {
            let facts = ground.facts(&map.facts);
            let Ok(Some(pedestal)) =
                crate::map::view::compose_tile(ground.overlay(), &facts, PEDESTAL)
            else {
                return UseDisposition::Rejected;
            };
            let ring = pedestal.entries.iter().find_map(|entry| {
                let crate::map::view::EntrySource::Ground(instance) = entry.source else {
                    return None;
                };
                let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                    entry.facts.definition
                else {
                    return None;
                };
                let (key, _) = index.definition_at(reference)?;
                (key == RING).then_some(instance)
            });
            let Some(item) = ring else {
                return toggle(None);
            };
            let Some(pickup_cell) = ground.pickup_cell(&item) else {
                return UseDisposition::StaleState;
            };
            let pin = runtime.content_pin();
            let (x, y, floor) = (
                RECEIVING_PEDESTAL.x,
                RECEIVING_PEDESTAL.y,
                RECEIVING_PEDESTAL.floor,
            );
            // Crystal actions_power_switch.lua:3-10 binds both pedestals. The actor's
            // reach is checked against the actual switch by the normal USE owner.
            let result = self
                .root
                .commit_item_loop_move(
                    permit,
                    self.character,
                    self.holder,
                    &fence.item_fence(),
                    command,
                    &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                    LoopMoveRequest {
                        item,
                        count: None,
                        pickup_cell: Some(pickup_cell),
                        destination: LoopDestination::Ground(LoopGround {
                            spatial_position: loop_move::ground_cell(
                                i32::from(x),
                                i32::from(y),
                                i16::from(floor),
                            ),
                            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                            content_revision: format!(
                                "sha256:{}",
                                hex(&pin.server_artifact_digest())
                            ),
                            placement_context: pin.frame_binding_digest().to_vec(),
                        }),
                    },
                )
                .await;
            self.states.loop_ground.invalidate();
            if !matches!(result, Ok(Ok(_))) {
                return UseDisposition::Rejected;
            }
        }
        let outcome = toggle(Some(&projection));
        if outcome == UseDisposition::Committed {
            map.facts.emit_movement_effect(PEDESTAL, TELEPORT_EFFECT);
        }
        outcome
    }
}
