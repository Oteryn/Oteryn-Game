//! M49 native projection is part of the session-independent World boss owner.
use super::*;
use crate::content::{CollisionClass, LogicalCell, channel_content_pin::ChannelContentPin};
use crate::durability::item_decay_retire::CorpseDecayFence;
use crate::durability::item_loop_move::{
    ElementalSummonClock, ElementalSummonProjection, ElementalSummonScope,
};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::map::floor::{BundleDoors, CellBlocker};
use oteryn_gameplay_entities::{CarrierError, ScopeRuntimeAssignmentBinding};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn local(clock: &ElementalSummonClock) -> MovementLocalPosition {
    let at = clock.intent.position;
    MovementLocalPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

/// No await separates current continuity/pin checks, collision and native admission.
pub(in crate::gameplay_transport) fn project(
    runtime: &mut ChannelRuntimeV1,
    binding: ScopeRuntimeAssignmentBinding,
    pin: &ChannelContentPin,
    clocks: &[ElementalSummonClock],
    walkable: impl Fn(MovementLocalPosition) -> bool,
) -> Result<(), CarrierError> {
    runtime.owner_fence()?;
    let keys: Vec<_> = clocks
        .iter()
        .map(|clock| clock.spawn_key.as_str())
        .collect();
    let unique: BTreeSet<_> = keys.iter().copied().collect();
    if runtime.binding() != binding
        || runtime.content_pin() != pin
        || clocks.len() > 4
        || unique.len() != clocks.len()
    {
        return Err(CarrierError::InvalidSpawnDefinition);
    }
    for clock in clocks {
        if clock.owner_generation != binding.scope_generation()
            || !clock
                .spawn_key
                .strip_prefix("qn1.elemental:")
                .is_some_and(|id| {
                    id.len() == 32
                        && id.bytes().any(|b| b != b'0')
                        && id
                            .bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                })
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        runtime.companion_policy(&clock.intent.creature_key)?;
        if let Some(old) = runtime
            .boss_spawns()
            .iter()
            .find(|spawn| spawn.key == clock.spawn_key)
            && (old.creature != clock.intent.creature_key
                || old.position != local(clock)
                || old.cycle != clock.cycle)
        {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
    }
    runtime.retain_elemental_boss_spawns(pin, binding.scope_generation(), &keys)?;
    for clock in clocks {
        let live = runtime
            .boss_spawns()
            .iter()
            .find(|spawn| spawn.key == clock.spawn_key)
            .and_then(|spawn| spawn.actor)
            .is_some_and(|actor| runtime.contains_live_creature(actor));
        if live || walkable(local(clock)) {
            runtime.install_elemental_boss_spawn(
                pin,
                binding.scope_generation(),
                &clock.spawn_key,
                &clock.intent.creature_key,
                local(clock),
                clock.cycle,
            )?;
        }
    }
    Ok(())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn elemental_spawn_pass(&self) -> Result<bool, RespawnExit> {
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return Ok(true);
        };
        // A generation only activates this source lane when an elemental corpse is qualified.
        if ![
            "oteryn:item.tibia.i8139",
            "oteryn:item.tibia.i8140",
            "oteryn:item.tibia.i8141",
            "oteryn:item.tibia.i8106",
        ]
        .iter()
        .any(|key| native.item_policy(key, "definition-r1").is_some())
        {
            return Ok(true);
        }
        let Some(map) = self.bundle_map() else {
            return Ok(true);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(false);
        };
        let (binding, pin) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
            if runtime.binding().channel_id().is_none() {
                return Ok(true);
            }
            // Channel pins bind the revision string, as BundleWorld does at admission.
            // Arrival readsets separately retain the underlying bundle digest below.
            let map_digest: [u8; 32] =
                Sha256::digest(crate::map::overlay::map_revision(map.overlay.base()).as_bytes())
                    .into();
            if runtime.content_pin().server_artifact_digest() != native.source_digest()
                || runtime.content_pin().map_revision_digest() != map_digest
            {
                return Err(RespawnExit::StaleOwner);
            }
            (runtime.binding(), {
                let pin = runtime.content_pin();
                let start = pin.entry_start();
                crate::foundation::ChannelContentPin::from_activation(
                    pin.world_id(),
                    pin.activation_sequence(),
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                    (start.x, start.y, start.floor),
                )
            })
        };
        let owner = CorpseDecayFence {
            runtime_scope: binding.runtime_scope(),
            scope_ownership_generation: binding.scope_generation(),
        };
        let projection = ElementalSummonProjection {
            scope: ElementalSummonScope {
                map: map.overlay.base().digest(),
                reset_epoch: self.reset_epoch,
            },
            content: pin.server_artifact_digest(),
            frame: pin.frame_binding_digest(),
        };
        let clocks = match self
            .root
            .recover_elemental_summons(&permit, self.character, self.holder, owner, projection)
            .await
        {
            Ok(clocks) => clocks,
            Err(_) => return Ok(false), // Keep every existing actor on an uncertain or stale read.
        };
        let mut runtime = self.runtime.lock().await;
        if runtime.binding() != binding || runtime.content_pin() != &pin {
            return Err(RespawnExit::StaleOwner);
        }
        let closed = BundleDoors(map.facts.doors());
        let cells = self.movement_cells();
        let walkable = |at: MovementLocalPosition| {
            let cell = LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            };
            cells.index().lookup(cells.scope(), cell) == Ok(CollisionClass::Walkable)
                && !closed.blocks(cell)
        };
        if project(&mut runtime, binding, &pin, &clocks, walkable).is_err() {
            runtime.retire_owner_cycle();
            return Err(RespawnExit::StaleOwner);
        }
        // Confirmed clocks may remain queued for an occupied or blocked cell.
        Ok(true)
    }
}
