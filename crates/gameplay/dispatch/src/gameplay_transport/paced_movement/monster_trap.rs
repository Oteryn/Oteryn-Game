//! Crystal `data/scripts/movements/trap.lua:23-44,59-69` (201189f8) for monsters.

use super::special_movement::{TileEntries, Trap, tile_entries, transform, trap};
use crate::content::ReferenceResistanceKind;
use crate::foundation::ChannelRuntimeV1;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) fn apply_monster_trap_steps(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    map: &BundleMap,
    now: u64,
) {
    crate::gameplay_transport::quest_callbacks::monsters::settle_authored_monster_puzzles(
        runtime, map,
    );
    for step in map.facts.take_monster_steps() {
        if let Some(TileEntries {
            protection_zone,
            entries,
        }) = tile_entries(map, step.from)
            && !protection_zone
        {
            for (key, entry) in entries {
                if matches!(entry.appearance_id, 2146 | 2148 | 3945 | 44530) {
                    transform(map, key, entry.appearance_id - 1);
                }
            }
        }

        if !runtime.contains_live_creature(step.actor) {
            continue;
        }
        let Some(destination) = tile_entries(map, step.to) else {
            continue;
        };
        if destination.protection_zone
            || destination
                .entries
                .iter()
                .filter(|(_, entry)| entry.appearance_id == 3482)
                .count()
                > 1
        {
            continue;
        }
        for (key, entry) in &destination.entries {
            let Some(rule) = trap(entry.appearance_id).filter(|rule| rule.maximum_damage > 0)
            else {
                continue;
            };
            if transform(map, *key, rule.closed) {
                spike_trap_hit(runtime, states, step.actor, *key, rule, now);
            }
        }
    }
}

fn spike_trap_hit(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: crate::foundation::ExactActorRef,
    key: u64,
    rule: Trap,
    now: u64,
) {
    let Ok(snapshot) = runtime.companion_snapshot(actor) else {
        return;
    };
    if !snapshot.state.conditions.accepts_time(now) {
        return;
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn:creature-spike-trap:v1")
        .chain_update(actor.placement_identity())
        .chain_update(snapshot.position_revision.to_be_bytes())
        .chain_update(key.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let Ok(draw) = deterministic_decision_u64(&root, occurrence, "movement.trap.damage", 0) else {
        return;
    };
    let damage = crate::spell::uniform_draw(
        draw,
        i64::from(rule.minimum_damage),
        i64::from(rule.maximum_damage),
    );
    let Ok(damage) = u32::try_from(damage) else {
        return;
    };
    let element = match rule.element {
        ReferenceResistanceKind::Earth => DotElement::Poison,
        _ => DotElement::Bleeding,
    };
    let Some(definition) = ConditionDefinition::new(
        "environment.spike_trap",
        1,
        ConditionValues::DamageOverTime {
            element,
            total_min: damage,
            total_max: damage,
            per_tick: damage,
            interval_ms: 2_000,
            delayed: false,
        },
    ) else {
        return;
    };
    let entry = CreatureFieldEntry {
        definition,
        source: None,
        lineage: None,
        occurrence,
    };
    if runtime
        .commit_creature_field_entry(actor, now, Some(&entry))
        .is_err()
    {
        return;
    }
    let _ = states.tick_source_creature_conditions(
        runtime,
        actor,
        crate::foundation::TickFacts {
            in_protection_zone: false,
            standing_on_field: None,
        },
        SemanticTimeMicros::from_micros(now),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::MovementLocalPosition;
    use crate::map::boot::{self, BundleCollisionIndexExt};
    use crate::map::facts::MonsterStep;
    use crate::map::overlay::TilePos;
    use crate::map::view::MapFacts;
    use oteryn_gameplay_content::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
    };
    use std::num::NonZeroU32;

    type BoxError = Box<dyn std::error::Error + Send + Sync>;

    fn trap_case(trap: u16, protection_zone: bool) -> Result<(u16, u16, i64, i64), BoxError> {
        let (mut runtime, actor, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x72);
        let key = format!("oteryn:item.tibia.i{trap}");
        let other = format!(
            "oteryn:item.tibia.i{}",
            super::trap(trap)
                .map(|rule| rule.closed)
                .filter(|closed| *closed != trap)
                .unwrap_or(100)
        );
        let (bytes, mut pins) = boot::tests::bundle_with_item_ids((&key, 0), (&other, 1));
        let mut bundle = oteryn_world_bundle::writer::read(&bytes)?;
        if protection_zone {
            for tile in &mut bundle.sectors[0].tiles {
                if tile.x == 6 {
                    tile.flags = 1;
                }
            }
        }
        let bytes = oteryn_world_bundle::writer::write(
            &bundle.manifest,
            &bundle.sectors,
            &Default::default(),
        )?;
        pins.digest = oteryn_world_bundle::writer::read(&bytes)?.digest;
        let pos = TilePos {
            x: 6,
            y: 0,
            floor: -7,
        };
        let base = crate::map::load(&bytes, &pins).map_err(|error| format!("{error:?}"))?;
        let booted = boot::boot(
            &bytes,
            &boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: pos,
            },
            actor.world_id(),
            actor.channel_id().ok_or("channel")?,
            |candidate| {
                Some(crate::map::facts::ItemDefinition {
                    reference: NonZeroU32::new(if candidate == key {
                        1
                    } else if candidate == other {
                        2
                    } else {
                        return None;
                    })?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        let room = crate::content::qualify_native_entry_room(actor.world_id())
            .map_err(|error| format!("{error:?}"))?;
        let cells = booted
            .movement_cells(room.movement_cells())
            .map_err(|error| format!("{error:?}"))?;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(collision) =
            cells.index()
        else {
            return Err("bundle collision".into());
        };
        let map = collision.map();
        let at = MovementLocalPosition {
            x: 6,
            y: 0,
            floor: 7,
        };
        let away = MovementLocalPosition { x: 5, ..at };
        let server_digest = runtime.content_pin().server_artifact_digest();
        runtime.install_companion_policies(CompiledCreaturePolicies::from_active_artifact(
            server_digest,
            vec![CompiledCreaturePolicy {
                definition_key: "oteryn:creature.rat".into(),
                definition_revision: "fixture:1".into(),
                display_name: "Rat".into(),
                maximum_health: 1_000,
                forge_difficulty: None,
                base_speed: 220,
                outfit_look_type: 21,
                object_look_type: None,
                summonable: false,
                convinceable: false,
                mana_cost: None,
                is_familiar: false,
                condition_immunities: vec![],
                defense: Some(0),
                armor: Some(0),
                mitigation: Some(CreatureExactRatio {
                    numerator: 0,
                    denominator: 1,
                }),
                resistances: vec![],
                damage_immunities: vec![],
                healing_from_damage: vec![],
                flags: CreatureFlags {
                    attackable: true,
                    illusionable: false,
                    health_hidden: false,
                },
                preferred_distance: Some(1),
                reward_boss: Some(false),
                blocked_by_nearby_players: false,
            }],
        )?)?;
        let rat = runtime
            .admit_pinned_monster_lab_creature(at, "oteryn:creature.rat", 1_000)
            .map_err(|error| format!("{error:?}"))?;
        runtime.install_creature_policy(rat, "oteryn:creature.rat")?;
        let states = ChannelSpellStates::default();
        let health_before =
            i64::try_from(runtime.live_creature_health(rat).ok_or("rat before hit")?)?;
        assert!(map.facts.record_monster_step(MonsterStep {
            actor: rat,
            from: away,
            to: at,
        }));
        apply_monster_trap_steps(&mut runtime, &states, map, 1_000_000);
        let after_step_in = map
            .facts
            .base_entry(pos, 1, 0)
            .ok_or("trap after step-in")?
            .appearance_id;
        let health_after =
            i64::try_from(runtime.live_creature_health(rat).ok_or("rat after hit")?)?;
        assert!(map.facts.record_monster_step(MonsterStep {
            actor: rat,
            from: at,
            to: away,
        }));
        apply_monster_trap_steps(&mut runtime, &states, map, 1_000_001);
        let after_step_out = map
            .facts
            .base_entry(pos, 1, 0)
            .ok_or("trap after step-out")?
            .appearance_id;
        Ok((after_step_in, after_step_out, health_before, health_after))
    }

    #[test]
    fn m10_monster_on_spike_trap_closes_it_takes_a_hit_and_reopens_it_on_step_out()
    -> Result<(), BoxError> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), BoxError> {
                let (closed, reopened, before, after) = trap_case(2145, false)?;
                assert_eq!((closed, reopened), (2146, 2145));
                assert!(after < before);
                let (closed, reopened, before, after) = trap_case(3944, false)?;
                assert_eq!((closed, reopened), (3945, 3944));
                assert!(after < before);
                Ok(())
            })?
            .join()
            .map_err(|_| "monster trap test thread")?
    }

    #[test]
    fn m10_monster_spike_trap_does_nothing_in_a_protection_zone() -> Result<(), BoxError> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), BoxError> {
                let (closed, reopened, before, after) = trap_case(2145, true)?;
                assert_eq!((closed, reopened), (2145, 2145));
                assert_eq!(after, before);
                Ok(())
            })?
            .join()
            .map_err(|_| "monster trap test thread")?
    }

    #[test]
    fn m10_monster_never_triggers_the_magic_wolf_trap_damage() -> Result<(), BoxError> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), BoxError> {
                let (closed, reopened, before, after) = trap_case(12368, false)?;
                assert_eq!((closed, reopened), (12368, 12368));
                assert_eq!(after, before);
                Ok(())
            })?
            .join()
            .map_err(|_| "monster trap test thread")?
    }
}
