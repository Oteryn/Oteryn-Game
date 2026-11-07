use super::*;
use crate::foundation::{ChannelId, NodeId, WorldId};
use crate::map::boot::BundleCollisionIndexExt;
use crate::map::{boot, overlay::TilePos, view};
use std::num::NonZeroU32;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn wolf_step(item: u16, key: &str, reward: u16, captures: bool) -> TestResult {
    let world = WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
    let channel = ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?;
    let node = NodeId::decode(&id(3)).map_err(|e| format!("{e:?}"))?;
    let keys = (
        format!("oteryn:item.tibia.i{item}"),
        format!(
            "oteryn:item.tibia.i{}",
            if item == 12368 { 100 } else { reward }
        ),
    );
    let (bytes, pins) = boot::tests::bundle_with_item_ids((&keys.0, 0), (&keys.1, 1));
    let base = crate::map::load(&bytes, &pins).map_err(|e| format!("{e:?}"))?;
    let booted = boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
        },
        world,
        channel,
        |k| {
            Some(crate::map::facts::ItemDefinition {
                reference: NonZeroU32::new(if k == keys.0 {
                    1
                } else if k == keys.1 {
                    2
                } else if k == "oteryn:item.tibia.i12369" {
                    3
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
    .map_err(|e| format!("{e:?}"))?;
    let entry = crate::content::qualify_native_entry_room(world).map_err(|e| format!("{e:?}"))?;
    let cells = booted
        .movement_cells(entry.movement_cells())
        .map_err(|e| format!("{e:?}"))?;
    let pin =
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
            world,
            1,
            cells.scope().generation_digest,
            [2; 32],
            [3; 32],
            booted.map_revision_digest(),
            (1, 0, 7),
        );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(|e| format!("{e:?}"))?;
    if item == 12721 {
        use oteryn_gameplay_content::content::creature_policy::{
            CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
        };
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    runtime.content_pin().server_artifact_digest(),
                    vec![CompiledCreaturePolicy {
                        definition_key: "oteryn:creature.crystal_wolf".into(),
                        definition_revision: "definition-r1".into(),
                        display_name: "Crystal Wolf".into(),
                        maximum_health: 750,
                        forge_difficulty: None,
                        base_speed: 220,
                        outfit_look_type: 1,
                        object_look_type: None,
                        summonable: false,
                        convinceable: false,
                        mana_cost: None,
                        is_familiar: false,
                        condition_immunities: vec![],
                        defense: Some(1),
                        armor: Some(1),
                        mitigation: None,
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
                )
                .map_err(|e| format!("{e:?}"))?,
            )
            .map_err(|e| format!("{e:?}"))?;
    }
    let at = MovementLocalPosition {
        x: 6,
        y: 0,
        floor: 7,
    };
    let wolf = runtime
        .admit_pinned_monster_lab_creature(MovementLocalPosition { x: 5, ..at }, key, 20)
        .map_err(|e| format!("{e:?}"))?;
    let binding = runtime.binding();
    let mut map = MonsterMap {
        valid: true,
        scope: binding.runtime_scope(),
        generation: binding.scope_generation(),
        context: runtime.pinned_movement_context(),
        activation: 1,
        server: runtime.content_pin().server_artifact_digest(),
        client: [2; 32],
        frame: [3; 32],
        map: booted.map_revision_digest(),
        tiles: BTreeMap::new(),
    };
    map.tiles.insert(
        (6, 0, 7),
        Some(Tile {
            admits: true,
            projectile: false,
            speed: 150,
            field: None,
        }),
    );
    let before = runtime
        .read_actor_position(wolf)
        .map_err(|e| format!("{e:?}"))?;
    map.step(&mut runtime, &cells, wolf, before, CardinalStep::East)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        !runtime.contains_live_creature(wolf),
        captures,
        "Crystal trap.lua:39-42 removes Starving Wolf on committed monster step"
    );
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(collision) =
        cells.index()
    else {
        return Err("bundle".into());
    };
    let bundle = collision.map();
    assert_eq!(
        bundle.facts.take_monster_steps(),
        vec![crate::map::facts::MonsterStep {
            actor: wolf,
            from: before.position(),
            to: at,
        }],
        "every committed monster step is recorded once for the trap owner"
    );
    assert!(bundle.facts.take_monster_steps().is_empty());
    let composed = view::compose_tile(
        &bundle.overlay,
        &bundle.facts,
        TilePos {
            x: 6,
            y: 0,
            floor: -7,
        },
    )
    .map_err(|e| format!("{e:?}"))?
    .ok_or("tile")?;
    assert!(
        composed
            .entries
            .iter()
            .any(|entry| entry.facts.appearance_id == item),
        "the trap stays"
    );
    assert_eq!(
        composed
            .entries
            .iter()
            .filter(|entry| entry.facts.appearance_id == reward)
            .count(),
        usize::from(captures && item == 12368)
    );
    assert_eq!(
        bundle
            .facts
            .movement_effects_since(0)
            .iter()
            .map(|(_, _, effect)| *effect)
            .collect::<Vec<_>>(),
        if captures {
            vec![if item == 12368 { 32 } else { 44 }]
        } else {
            vec![]
        }
    );
    if item == 12721 && captures {
        let replacements = runtime
            .positioned_melee_census()
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(replacements.len(), 1);
        let replacement = replacements[0].0;
        assert_ne!(replacement, wolf, "Crystal creates a new incarnation");
        let replacement = runtime
            .companion_snapshot(replacement)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            replacement.state.policy.definition_key,
            "oteryn:creature.crystal_wolf"
        );
        assert_eq!(replacement.health, 750);
        assert_eq!(replacement.position, at);
    }
    assert!(
        map.step(&mut runtime, &cells, wolf, before, CardinalStep::East)
            .is_err(),
        "stale step cannot capture twice"
    );
    Ok(())
}

#[test]
fn m10_wolf_trap_runs_on_real_monster_step() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            wolf_step(12368, "oteryn:creature.rat", 12369, false)?;
            wolf_step(12368, "oteryn:creature.starving_wolf", 12369, true)
        })?
        .join()
        .map_err(|_| "wolf test thread")?
}

#[test]
fn m10_thornfire_wolf_is_replaced_on_real_monster_step() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            wolf_step(12721, "oteryn:creature.rat", 100, false)?;
            wolf_step(12721, "oteryn:creature.thornfire_wolf", 100, true)
        })?
        .join()
        .map_err(|_| "wolf test thread")?
}
