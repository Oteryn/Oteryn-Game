//! Crystal `data/scripts/movements/trap.lua:23-44,59-69` (201189f8) for monsters.

use super::special_movement::{TileEntries, Trap, tile_entries, transform, trap};
use crate::content::ReferenceResistanceKind;
use crate::foundation::ChannelRuntimeV1;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use crate::map::view::MapFacts;
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
        // The Grimvale callback is independent of generic spike-trap PZ/jaw gates.
        for (key, entry) in &destination.entries {
            if entry.appearance_id == 22059 {
                silvered_trap_hit(runtime, map, step.actor, *key, step.to);
            }
        }
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

/// Crystal 9f5a72c data-global/scripts/quests/grimvale/movements_silvered_trap.lua:3-22.
/// All five qualified bodies share the source name; only maxHP 50000 takes untyped damage.
fn silvered_trap_hit(
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    actor: crate::foundation::ExactActorRef,
    key: u64,
    at: crate::foundation::MovementLocalPosition,
) {
    let Ok(body) = runtime.companion_snapshot(actor) else {
        return;
    };
    if map.overlay.runtime_scope() != runtime.binding().runtime_scope()
        || !matches!(
            body.state.policy.definition_key.as_str(),
            "oteryn:creature.feroxa"
                | "oteryn:creature.feroxa2"
                | "oteryn:creature.feroxa3"
                | "oteryn:creature.feroxa4"
                | "oteryn:creature.feroxa5"
        )
    {
        return;
    }
    let damaging = body.maximum_health == 50_000;
    let committed = if damaging {
        let digest = Sha256::new()
            .chain_update(b"oteryn:feroxa-silvered-trap:v1")
            .chain_update(actor.placement_identity())
            .chain_update(body.position_revision.to_be_bytes())
            .chain_update(key.to_be_bytes())
            .chain_update(map.facts.object_revision(key).to_be_bytes())
            .finalize();
        let occurrence: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let mut binding = occurrence.as_bytes().to_vec();
        binding.push(0);
        binding.extend_from_slice(&1_000_i64.to_be_bytes());
        runtime.borrow_exact_actor_commit().commit_raw_creature_damage_batch(
            &[actor], oteryn_gameplay_entities::OwnerDamageCommand {
                target: body.state.policy.definition_key.as_bytes(),
                occurrence: occurrence.as_bytes(),
                binding: &binding,
                damage: 1_000,
            }, |_| {
                if transform(map, key, 22074) { Ok(()) }
                else { Err(oteryn_gameplay_domain::domain::carrier_values::CarrierError::PlanConflict) }
            },
        ).is_ok()
    } else {
        transform(map, key, 22074)
    };
    if committed
        && let Some(pos) = (|| {
            Some(crate::map::overlay::TilePos {
                x: at.x.try_into().ok()?,
                y: at.y.try_into().ok()?,
                floor: i8::try_from(at.floor).ok()?.checked_neg()?,
            })
        })()
    {
        // Qualified client effect bindings: DRAWBLOOD=1, BLOCKHIT=4.
        if damaging {
            map.facts.emit_movement_effect(pos, 1);
        }
        map.facts.emit_movement_effect(pos, 4);
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
        trap_case_for(trap, protection_zone, "oteryn:creature.rat", 1_000)
    }

    fn trap_case_for(
        trap: u16,
        protection_zone: bool,
        creature: &str,
        health: i64,
    ) -> Result<(u16, u16, i64, i64), BoxError> {
        trap_case_source(trap, protection_zone, creature, health, false, false)
    }

    fn trap_case_source(
        trap: u16,
        protection_zone: bool,
        creature: &str,
        health: i64,
        volatile: bool,
        phase: bool,
    ) -> Result<(u16, u16, i64, i64), BoxError> {
        let (mut runtime, actor, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x72);
        let key = format!("oteryn:item.tibia.i{trap}");
        let other = format!(
            "oteryn:item.tibia.i{}",
            super::trap(trap)
                .map(|rule| rule.closed)
                .filter(|closed| *closed != trap)
                .unwrap_or(if trap == 22059 { 22074 } else { 100 })
        );
        let (bytes, mut pins) = boot::tests::bundle_with_item_ids((&key, 0), (&other, 1));
        let mut bundle = oteryn_world_bundle::writer::read(&bytes)?;
        if volatile || phase {
            for tile in &mut bundle.sectors[0].tiles {
                if tile.x == 6 {
                    tile.items.truncate(1);
                }
            }
        }
        if protection_zone && !phase {
            for tile in &mut bundle.sectors[0].tiles {
                if tile.x == 6 {
                    tile.flags = 1;
                }
            }
        }
        if phase {
            bundle.manifest.world.min_x += 33409;
            bundle.manifest.world.max_x += 33409;
            bundle.manifest.world.min_y += 31522;
            bundle.manifest.world.max_y += 31522;
            bundle.manifest.world.floors = vec![-11];
            for sector in &mut bundle.sectors {
                sector.floor = -11;
                for tile in &mut sector.tiles {
                    tile.x += 33409;
                    tile.y += 31522;
                }
                sector.sx = sector.tiles.first().ok_or("phase sector")?.x / 32;
                sector.sy = sector.tiles.first().ok_or("phase sector")?.y / 32;
            }
        }
        let bytes = oteryn_world_bundle::writer::write(
            &bundle.manifest,
            &bundle.sectors,
            &Default::default(),
        )?;
        pins.digest = oteryn_world_bundle::writer::read(&bytes)?.digest;
        let pos = TilePos {
            x: if phase { 33415 } else { 6 },
            y: if phase { 31522 } else { 0 },
            floor: if phase { -11 } else { -7 },
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
        if phase {
            use crate::content::{NativeSpellTileFlags, QualifiedSpellTile};
            let ground = map.facts.base_entry(pos, 0, 0).ok_or("phase ground")?;
            let trap_entry = map.facts.movement_item_facts(trap).ok_or("phase trap")?;
            assert!(map.publish_feroxa_phase(
                *actor.world_id().as_bytes(),
                1,
                [4; 32],
                vec![crate::map::facts::FeroxaPhaseTile {
                    position: pos,
                    entries: vec![ground, trap_entry],
                    no_logout: false,
                    teleport: None,
                    spell: QualifiedSpellTile::from_bundle(
                        Some((u32::from(ground.appearance_id), 150)),
                        vec![u32::from(trap)],
                        NativeSpellTileFlags {
                            protection_zone,
                            block_solid: false,
                            block_projectile: false,
                            immovable_block_solid: false,
                            immovable_block_item: false,
                            immovable_nonfield_block_item: false,
                            floor_change: false
                        },
                        Some((false, false)),
                    ),
                },]
            ));
        }
        if volatile {
            assert!(map.facts.create_movement_item(pos, trap, 0, || true));
        }
        let original_created = map.facts.movement_items(pos);
        let pinned_before = map.facts.pinned();
        let appearance = || -> Result<u16, BoxError> {
            if volatile {
                Ok(map
                    .facts
                    .movement_items(pos)
                    .first()
                    .ok_or("created trap")?
                    .facts
                    .appearance_id)
            } else {
                Ok(map
                    .facts
                    .base_entry(pos, 1, 0)
                    .ok_or("base trap")?
                    .appearance_id)
            }
        };
        let at = MovementLocalPosition {
            x: i32::from(pos.x),
            y: i32::from(pos.y),
            floor: -i16::from(pos.floor),
        };
        let away = MovementLocalPosition { x: at.x - 1, ..at };
        let server_digest = runtime.content_pin().server_artifact_digest();
        runtime.install_companion_policies(CompiledCreaturePolicies::from_active_artifact(
            server_digest,
            vec![CompiledCreaturePolicy {
                definition_key: creature.into(),
                definition_revision: "fixture:1".into(),
                display_name: "Rat".into(),
                maximum_health: health,
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
                armor: Some(if creature == "oteryn:creature.feroxa2" {
                    10_000
                } else {
                    0
                }),
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
            .admit_pinned_monster_lab_creature(at, creature, health)
            .map_err(|error| format!("{error:?}"))?;
        runtime.install_creature_policy(rat, creature)?;
        let states = ChannelSpellStates::default();
        let health_before =
            i64::try_from(runtime.live_creature_health(rat).ok_or("rat before hit")?)?;
        assert!(map.facts.record_monster_step(MonsterStep {
            actor: rat,
            from: away,
            to: at,
        }));
        if trap == 22059 {
            // Another committed move may precede draining the native step journal.
            // The earlier trap contact still belongs to this exact actor generation.
            runtime
                .commit_quest_platform_displacement(at, away, |_, _, _| true, || true)
                .map_err(|error| format!("{error:?}"))?;
        }
        apply_monster_trap_steps(&mut runtime, &states, map, 1_000_000);
        let after_step_in = appearance()?;
        let health_after =
            i64::try_from(runtime.live_creature_health(rat).ok_or("rat after hit")?)?;
        if trap == 22059 {
            assert!(map.facts.record_monster_step(MonsterStep {
                actor: rat,
                from: away,
                to: at
            }));
            apply_monster_trap_steps(&mut runtime, &states, map, 1_000_001);
            assert_eq!(
                i64::try_from(runtime.live_creature_health(rat).ok_or("replayed trap")?)?,
                health_after,
                "a consumed silvered trap cannot hit again"
            );
        }
        assert!(map.facts.record_monster_step(MonsterStep {
            actor: rat,
            from: at,
            to: away,
        }));
        apply_monster_trap_steps(&mut runtime, &states, map, 1_000_001);
        let after_step_out = appearance()?;
        if volatile {
            let current = map.facts.movement_items(pos);
            assert_eq!(current.len(), original_created.len());
            assert_eq!(
                current[0].source, original_created[0].source,
                "a transform preserves the exact created item identity"
            );
            assert_eq!(
                pinned_before.movement_items(pos),
                original_created,
                "previously pinned views retain their original source facts"
            );
            let crate::map::view::EntrySource::Added(id) = current[0].source else {
                return Err("created source identity".into());
            };
            assert!(
                !map.facts.compare_transform_movement_item(id.get(), 0, trap),
                "a stale revision cannot rearm a consumed trap"
            );
            assert!(
                !map.facts.compare_transform_movement_item(
                    crate::map::view::placement_key(pos, 0).ok_or("base key")?,
                    0,
                    trap
                ),
                "a base placement is not a callback-created item"
            );
        }
        Ok((after_step_in, after_step_out, health_before, health_after))
    }

    #[test]
    fn qn5_feroxa_phase_only_trap_contact_reads_the_current_stack_and_zone() -> Result<(), BoxError>
    {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), BoxError> {
                assert_eq!(
                    trap_case_source(22059, true, "oteryn:creature.feroxa2", 50_000, false, true)?,
                    (22074, 22074, 50_000, 49_000)
                );
                assert_eq!(
                    trap_case_source(2145, true, "oteryn:creature.rat", 1_000, false, true)?,
                    (2145, 2145, 1_000, 1_000)
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "phase trap test thread panicked")?
    }

    #[test]
    fn qn5_callback_created_traps_trigger_and_retain_their_native_identity() -> Result<(), BoxError>
    {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), BoxError> {
                let (closed, exited, before, after) =
                    trap_case_source(22059, false, "oteryn:creature.feroxa2", 50_000, true, false)?;
                assert_eq!(
                    (closed, exited, before, after),
                    (22074, 22074, 50_000, 49_000)
                );
                let (closed, exited, before, after) =
                    trap_case_source(2145, false, "oteryn:creature.rat", 1_000, true, false)?;
                assert_eq!((closed, exited), (2146, 2145));
                assert!(after < before);
                Ok(())
            })?
            .join()
            .map_err(|_| "created trap thread")?
    }

    #[test]
    fn qn5_silvered_trap_binds_feroxa_forms_and_bypasses_armor_only_for_second_form()
    -> Result<(), BoxError> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), BoxError> {
                for protected in [false, true] {
                    let (closed, after_exit, before, after) =
                        trap_case_for(22059, protected, "oteryn:creature.feroxa2", 50_000)?;
                    assert_eq!((closed, after_exit), (22074, 22074));
                    assert_eq!(
                        (before, after),
                        (50_000, 49_000),
                        "source untyped hit ignores armor"
                    );
                }
                for (key, hp) in [
                    ("feroxa", 100_000),
                    ("feroxa3", 130_000),
                    ("feroxa4", 130_000),
                    ("feroxa5", 150_000),
                ] {
                    let (closed, after_exit, before, after) =
                        trap_case_for(22059, false, &format!("oteryn:creature.{key}"), hp)?;
                    assert_eq!((closed, after_exit), (22074, 22074));
                    assert_eq!((before, after), (hp, hp));
                }
                let (closed, after_exit, before, after) = trap_case(22059, false)?;
                assert_eq!((closed, after_exit), (22059, 22059));
                assert_eq!(before, after);
                Ok(())
            })?
            .join()
            .map_err(|_| "silvered trap test thread")?
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
