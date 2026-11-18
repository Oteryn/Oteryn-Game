//! Crystal 201189f8: Shargon's adjacent field check and seven-second minion upgrade.
use super::{Plan, Role};
use oteryn_gameplay_content::content::{
    NativeEntryMovementCells, native_cell_lookup::NativeMovementCollisionIndex,
};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition,
};
use oteryn_gameplay_world::map::{
    boot::BundleCollisionIndexExt, facts::MovementSpeech, overlay::TilePos, view,
};
use oteryn_protocol_oteryn::entity_names::EntityDisplayName;
use sha2::{Digest, Sha256};

const BOSS: &str = "oteryn:creature.death_priest_shargon";
const LESSER: &str = "oteryn:creature.lesser_death_minion";
const GREATER: &str = "oteryn:creature.greater_death_minion";
const ROLES: [Role; 1] = [Role {
    creature: BOSS,
    after: 0,
}];

pub fn plan() -> Plan {
    Plan {
        roles: &ROLES,
        terminal: 1,
    }
}

#[derive(Default, Debug)]
pub struct ShargonGrowth {
    next_field_check_us: u64,
    upgrades: [Option<(ExactActorRef, u64)>; 2],
}

impl ShargonGrowth {
    pub fn think(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        if !runtime.contains_live_creature(boss) {
            return Ok(());
        }
        let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
            return Err(CarrierError::PositionContextMismatch);
        };
        let map = index.map();
        let at = runtime.read_actor_position(boss)?.position();
        if now_us >= self.next_field_check_us {
            let draw = Sha256::new()
                .chain_update(b"oteryn:shargon-field:v1")
                .chain_update(runtime.content_pin().server_artifact_digest())
                .chain_update(now_us.to_be_bytes())
                .chain_update(at.x.to_be_bytes())
                .chain_update(at.y.to_be_bytes())
                .finalize();
            let sampled = MovementLocalPosition {
                x: at.x.saturating_add(i32::from(draw[0] % 3) - 1),
                y: at.y.saturating_add(i32::from(draw[1] % 3) - 1),
                ..at
            };
            if tile(sampled)
                .and_then(|pos| {
                    view::compose_tile(&map.overlay, &map.facts, pos)
                        .ok()
                        .flatten()
                })
                .is_some_and(|stack| {
                    stack
                        .entries
                        .iter()
                        .any(|entry| matches!(entry.facts.appearance_id, 2129 | 2130))
                })
            {
                runtime.remove_encounter_actor(boss)?;
                yell(
                    map,
                    boss,
                    at,
                    "That trap will not hold me. Deal with my minions!",
                );
                // Crystal's Death Reaper lookup has no matching creature
                // definition at this revision. The boss still disappears.
                return Ok(());
            }
            self.next_field_check_us = now_us.saturating_add(1_000_000);
        }

        for upgrade in &mut self.upgrades {
            if let Some((minion, due)) = *upgrade {
                if !runtime.matches_live_creature_identity(minion, LESSER.as_bytes())
                    || runtime.native_summon_master(minion) != Some(boss)
                {
                    *upgrade = None;
                } else if now_us >= due {
                    let position = runtime.read_actor_position(minion)?.position();
                    let health = runtime.companion_policy(GREATER)?.maximum_health;
                    let before = runtime.encounter_parts().0.clone();
                    runtime.remove_encounter_actor(minion)?;
                    if let Err(error) =
                        runtime.commit_quest_creature_spawn(&[(GREATER, health, position)])
                    {
                        *runtime.encounter_parts_mut().0 = before;
                        return Err(error);
                    }
                    *upgrade = None;
                    yell(map, boss, at, "Rise, my minion, and grow stronger!");
                    if let Some(pos) = tile(position) {
                        map.facts.emit_movement_effect(pos, 17);
                    }
                }
            }
        }

        if self.upgrades.iter().any(Option::is_none)
            && runtime.native_summon_count(boss, Some(LESSER)) > 0
        {
            for (minion, _, _) in runtime.positioned_melee_census()? {
                if runtime.native_summon_master(minion) != Some(boss)
                    || !runtime.matches_live_creature_identity(minion, LESSER.as_bytes())
                    || self
                        .upgrades
                        .iter()
                        .flatten()
                        .any(|(existing, _)| *existing == minion)
                {
                    continue;
                }
                let Some(slot) = self.upgrades.iter_mut().find(|slot| slot.is_none()) else {
                    break;
                };
                *slot = Some((minion, now_us.saturating_add(7_000_000)));
            }
        }
        Ok(())
    }
}

fn tile(at: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: u16::try_from(at.x).ok()?,
        y: u16::try_from(at.y).ok()?,
        floor: i8::try_from(at.floor).ok()?.checked_neg()?,
    })
}

fn yell(
    map: &oteryn_gameplay_world::map::boot::BundleMap,
    speaker: ExactActorRef,
    at: MovementLocalPosition,
    text: &str,
) {
    if let Some(position) = tile(at) {
        map.facts.emit_movement_speech(MovementSpeech {
            speaker,
            position,
            talk_type: 37,
            text: text.into(),
            captured_name: EntityDisplayName::new("Death Priest Shargon").ok(),
        });
    }
}
