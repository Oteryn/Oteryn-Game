//! Crystal 201189f8: Ravager floor rune and Greater Canopic Jar healing.
use super::{Plan, Role};
use oteryn_gameplay_content::content::{
    NativeEntryMovementCells, native_cell_lookup::NativeMovementCollisionIndex,
};
use oteryn_gameplay_entities::{CarrierError, ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_world::map::{
    boot::BundleCollisionIndexExt, facts::MovementSpeech, overlay::TilePos, view,
};
use oteryn_protocol_oteryn::entity_names::EntityDisplayName;
use sha2::{Digest, Sha256};

const BOSS: &str = "oteryn:creature.the_ravager";
const JAR: &[u8] = b"oteryn:creature.greater_canopic_jar";
const ROLES: [Role; 1] = [Role {
    creature: BOSS,
    after: 0,
}];

#[derive(Debug, Default)]
pub struct RavagerHealing {
    next_tile_us: u64,
    next_proximity_us: u64,
}

pub fn plan() -> Plan {
    Plan {
        roles: &ROLES,
        terminal: 1,
    }
}

impl RavagerHealing {
    pub fn think(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        boss: ExactActorRef,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        let Some(health) = runtime.live_creature_health(boss) else {
            return Ok(());
        };
        let maximum = u64::try_from(runtime.companion_policy(BOSS)?.maximum_health)
            .map_err(|_| CarrierError::InvalidCreatureHealth)?;
        if health >= maximum || now_us < self.next_tile_us.min(self.next_proximity_us) {
            return Ok(());
        }
        let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
            return Err(CarrierError::PositionContextMismatch);
        };
        let at = runtime.read_actor_position(boss)?.position();
        let tile = TilePos {
            x: u16::try_from(at.x).map_err(|_| CarrierError::PositionContextMismatch)?,
            y: u16::try_from(at.y).map_err(|_| CarrierError::PositionContextMismatch)?,
            floor: i8::try_from(at.floor)
                .ok()
                .and_then(i8::checked_neg)
                .ok_or(CarrierError::PositionContextMismatch)?,
        };
        let map = index.map();
        let floor_heal = now_us >= self.next_tile_us
            && u128::from(health) * 10_000 < u128::from(maximum) * 9_999
            && view::compose_tile(&map.overlay, &map.facts, tile).is_ok_and(|stack| {
                stack.is_some_and(|stack| {
                    stack
                        .entries
                        .iter()
                        .any(|entry| entry.facts.appearance_id == 519)
                })
            });
        let jar_heal = now_us >= self.next_proximity_us
            && runtime
                .positioned_melee_census()?
                .iter()
                .any(|(actor, position, _)| {
                    let other = position.position();
                    other.floor == at.floor
                        && other.x.abs_diff(at.x) <= 25
                        && other.y.abs_diff(at.y) <= 25
                        && runtime.matches_live_creature_identity(*actor, JAR)
                });
        let amount = u64::from(jar_heal) * 10_000
            + if floor_heal {
                {
                    let digest = Sha256::new()
                        .chain_update(b"oteryn:ravager-floor-heal:v1")
                        .chain_update(runtime.content_pin().server_artifact_digest())
                        .chain_update(now_us.to_be_bytes())
                        .chain_update(at.x.to_be_bytes())
                        .chain_update(at.y.to_be_bytes())
                        .finalize();
                    let mut bytes = [0; 8];
                    bytes.copy_from_slice(&digest[..8]);
                    crate::spell::uniform_draw(u64::from_be_bytes(bytes), 7_500, 9_000) as u64
                }
            } else {
                0
            };
        if amount > 0 {
            runtime.commit_source_creature_heal_batch(
                runtime.content_pin().server_artifact_digest(),
                &[(boss, BOSS.into(), maximum, amount)],
            )?;
            if floor_heal {
                map.facts.emit_movement_speech(MovementSpeech {
                    speaker: boss,
                    position: tile,
                    talk_type: 36,
                    text: "The floor rune restores the Ravager's health.".into(),
                    captured_name: EntityDisplayName::new("The Ravager").ok(),
                });
            }
            if jar_heal {
                map.facts.emit_movement_effect(tile, 13);
            }
        }
        if now_us >= self.next_tile_us {
            self.next_tile_us = now_us.saturating_add(1_000_000);
        }
        if now_us >= self.next_proximity_us {
            self.next_proximity_us = now_us.saturating_add(2_000_000);
        }
        Ok(())
    }
}
