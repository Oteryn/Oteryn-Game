//! Closed Crystal201189 USE1948 source, composed with the existing movement owner.
//! ladder_up.lua SHA730bc7664484e953b6efc3c8faeba9936d355c5cdde91d1f3e0686f714f4df13;
//! position.lua SHAc18d16465394a4064e82d35463105511c1f273e7c6de588f29b9da8b1eadaf9b.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission,
};
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::map::{boot::BundleMap, view::MapFacts};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

// Position:moveUpstairs22–49: SOUTH first, then N,E,W (the source repeats W),
// SW,SE,NW,NE. Repeated W has no effect because selection is a pure tile read.
const OFFSETS: &[(i32, i32)] = &[
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (-1, 1),
    (1, 1),
    (-1, -1),
    (1, -1),
];
fn upstairs(
    spot: LogicalCell,
    walkable: impl Fn(LogicalCell) -> bool,
) -> Option<MovementLocalPosition> {
    let z = spot.z.checked_sub(1).filter(|z| (0..=15).contains(z))?;
    for &(dx, dy) in OFFSETS {
        let (Some(x), Some(y)) = (spot.x.checked_add(dx), spot.y.checked_add(dy)) else {
            continue;
        };
        if !(0..=u16::MAX as i32).contains(&x) || !(0..=u16::MAX as i32).contains(&y) {
            continue;
        }
        if walkable(LogicalCell { x, y, z }) {
            return Some(MovementLocalPosition {
                x,
                y,
                floor: i16::try_from(z).ok()?,
            });
        }
    }
    None
}
fn source_ladder(
    map: &BundleMap,
    target: &WorldObjectTarget,
) -> Option<(crate::map::overlay::TilePos, u8, u64)> {
    let bytes: &[u8; 40] = target.placement.as_slice().try_into().ok()?;
    if bytes[..32] != map.overlay.base().digest() {
        return None;
    }
    let key = u64::from_be_bytes(bytes[32..].try_into().ok()?);
    let (pos, ordinal) = crate::map::view::placement_of(key)?;
    let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))?
        .0;
    let facts = map.facts.base_entry(pos, ordinal, *id)?;
    // A source-specific AID/UID action retains its existing callback priority.
    (facts.appearance_id == 1948 && !facts.bound && !map.facts.house_tile(pos))
        .then_some((pos, ordinal, key))
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_bundle_ladder(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        source_ladder(map, target)?;
        let permit = self.spell_lane_permit().await?;
        if !self.refresh_pvp_locks(actor, command.game_session_id).await {
            return Some(UseOutcome::rejected());
        }
        let optional_pvp = self
            .special_movement_policy(&permit, actor, command.game_session_id)
            .await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let session = command.game_session_id;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|p| p.control_loss.is_none())
        {
            return Some(UseOutcome::rejected());
        }
        let index = item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let (pos, ordinal, _) = source_ladder(map, target)?;
        let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let id = *tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))?
            .0;
        let facts = map.facts.base_entry(pos, ordinal, id)?;
        let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) = facts.definition
        else {
            return None;
        };
        let (key, _) = index.definition_at(reference)?;
        if key != "oteryn:item.tibia.i1948" {
            return None;
        }
        let pinned = map.facts.pinned();
        let source = super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &pinned,
            content_generation: runtime.content_pin().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        if let Err(refusal) = super::world_map::resolve_map_target(&source, target, |_| None::<()>)
        {
            let disposition = match refusal {
                super::world_map::MapTargetRefusal::Use(d) => d,
                _ => UseDisposition::StaleState,
            };
            return Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            });
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return Some(UseOutcome::rejected());
        };
        let before = expected.position();
        let spot = LogicalCell {
            x: i32::from(pos.x),
            y: i32::from(pos.y),
            z: -i32::from(pos.floor),
        };
        if i32::from(before.floor) != spot.z
            || before.x.abs_diff(spot.x) > 1
            || before.y.abs_diff(spot.y) > 1
        {
            return Some(UseOutcome {
                disposition: UseDisposition::TooFar,
                ..UseOutcome::rejected()
            });
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Some(UseOutcome {
                disposition: UseDisposition::Exhausted,
                ..UseOutcome::rejected()
            });
        }
        let cells = self.movement_cells;
        let Some(destination) = upstairs(spot, |at| {
            cells.index().lookup(cells.scope(), at) == Ok(CollisionClass::Walkable)
                && cells
                    .spell_tiles()
                    .lookup(cells.scope(), at)
                    .is_ok_and(|tile| {
                        tile.ground_present()
                            && !tile.flags().block_solid
                            && !tile.flags().block_projectile
                    })
        }) else {
            return Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            });
        };
        let blocked = super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        if blocked
            && cells
                .spell_tiles()
                .lookup(
                    cells.scope(),
                    LogicalCell {
                        x: destination.x,
                        y: destination.y,
                        z: i32::from(destination.floor),
                    },
                )
                .is_ok_and(|t| t.flags().protection_zone)
        {
            return Some(UseOutcome {
                disposition: UseDisposition::PzBlocked,
                ..UseOutcome::rejected()
            });
        }
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            cells,
            actor,
            session,
            expected,
            destination,
            blocked,
            &closed,
        ) else {
            return Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            });
        };
        let Ok(after) = runtime.commit_source_step(proof) else {
            return Some(UseOutcome::rejected());
        };
        let now = self.owner_now().get();
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now,
            Some(before),
            after.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now,
            super::paced_movement::underwater_drown::underwater(map, before),
            super::paced_movement::underwater_drown::underwater(map, after.position()),
        );
        self.states.chat.close_out_of_range(&runtime, session);
        Some(UseOutcome {
            disposition: UseDisposition::Committed,
            ..UseOutcome::rejected()
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ladder_source_prefers_south_then_exact_fallback_and_refuses_invalid_floors() {
        let spot = LogicalCell { x: 50, y: 60, z: 8 };
        assert_eq!(
            upstairs(spot, |_| true),
            Some(MovementLocalPosition {
                x: 50,
                y: 61,
                floor: 7
            })
        );
        assert_eq!(
            upstairs(spot, |p| p.x == 50 && p.y == 59),
            Some(MovementLocalPosition {
                x: 50,
                y: 59,
                floor: 7
            })
        );
        assert_eq!(
            upstairs(spot, |p| p.x == 49 && p.y == 60),
            Some(MovementLocalPosition {
                x: 49,
                y: 60,
                floor: 7
            })
        );
        assert_eq!(upstairs(spot, |_| false), None);
        assert_eq!(upstairs(LogicalCell { x: 50, y: 60, z: 0 }, |_| true), None);
    }
}
