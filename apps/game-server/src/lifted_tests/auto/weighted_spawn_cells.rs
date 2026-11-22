//! Weighted home spawns through World compilation, node activation and the owner turn.
use std::error::Error;

use crate::foundation::{ChannelId, ChannelRuntimeV1, NodeId, WorldId};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
use oteryn_world_bundle::spawn;
use oteryn_world_bundle_compiler::bundle::{BuildClass, Extent, Family, Terrain, TerrainKind};
use oteryn_world_bundle_compiler::compile::{Input, KeyResolver, Resolution, realize_spawns};
use oteryn_world_bundle_compiler::project::{CreatureFacts, Families};
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};

const RAT: &str = "oteryn:creature.rat";
const CAVE_RAT: &str = "oteryn:creature.cave_rat";

struct WalkableGround;

impl KeyResolver for WalkableGround {
    fn resolve(&self, key: &str) -> Resolution {
        match key {
            "terrain:grass" => Resolution::Resolved(Family::Terrain, 1),
            _ => Resolution::Unknown,
        }
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, oteryn_world_bundle::Error> {
        Ok((key == "terrain:grass").then_some(Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
            blocks_projectile: None,
        }))
    }

    fn floor_change(&self, _key: &str) -> bool {
        false
    }
}

fn authored_spawn_table() -> Result<spawn::Table, Box<dyn Error>> {
    let mut families = Families::default();
    families.add_spawns(
        br#"{
      "family":"Spawn.Source","coordinate_frame":"global-target-2026-09-27",
      "records":[{"declaration":{
        "identity":{"key":"oteryn:spawn.weighted_test"},
        "centre":{"x":1,"y":11,"floor":7},
        "points":[
          {"cell":{"x":1,"y":11,"floor":7},"creature":"oteryn:creature.cave_rat",
           "direction":"south","respawn_ms":20000,"weight":1},
          {"cell":{"x":2,"y":11,"floor":7},"creature":"oteryn:creature.rat",
           "direction":"south","respawn_ms":45000},
          {"cell":{"x":1,"y":11,"floor":7},"creature":"oteryn:creature.rat",
           "direction":"south","respawn_ms":30000,"weight":3}
        ]
      }}]
    }"#,
    )?;
    for key in [RAT, CAVE_RAT] {
        families.creatures.insert(
            key.into(),
            CreatureFacts {
                period: "All".into(),
                boss: false,
            },
        );
    }
    let tiles = [1, 2].map(|x| Tile {
        x,
        y: 11,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items: vec![Item {
            palette: 0,
            depth: 0,
            attrs: Attrs::default(),
        }],
    });
    let frame = zstd::bulk::compress(&sector::encode(&tiles)?, 3)?;
    let mut region = b"OTRB\x01\x07".to_vec();
    for value in [0_u16, 0, 1] {
        region.extend_from_slice(&value.to_le_bytes());
    }
    region.push(0);
    region.extend_from_slice(&21_u32.to_le_bytes());
    region.extend_from_slice(&u32::try_from(frame.len())?.to_le_bytes());
    region.extend_from_slice(&frame);
    let extent = Extent {
        min_x: 0,
        min_y: 0,
        max_x: 31,
        max_y: 31,
        floors: vec![-7],
    };
    let (table, report) = realize_spawns(
        &Input {
            regions: &[region],
            palette: &["terrain:grass".into()],
            identity: Default::default(),
            world: extent.clone(),
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &families,
        },
        &WalkableGround,
    )?;
    assert_eq!(report.realized_points, 3);
    let decoded = spawn::decode(&spawn::encode(&table), &extent)?;
    assert_eq!(decoded, table);
    Ok(decoded)
}

fn start_runtime(table: spawn::Table) -> Result<ChannelRuntimeV1, Box<dyn Error>> {
    let id = |n| [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n];
    let world = WorldId::decode(&id(1))?;
    let channel = ChannelId::decode(&id(2))?;
    let node = NodeId::decode(&id(3))?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("active native companion profiles")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (2, 11, -7),
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
        channel_pin,
    )?;
    native.install_companion_policies(&mut runtime)?;
    let bundle = crate::map::boot::tests::door_world()?.with_spawns_for_test(table);
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    assert_eq!(
        executor
            .block_on(crate::node::serve::realize_bundle_spawns(
                &mut runtime,
                &bundle,
                &oteryn_foundation::CancellationToken::new(),
            ))
            .map_err(|error| error.to_string())?,
        Some((2, 1))
    );
    Ok(runtime)
}

#[test]
fn shared_spawn_cell_draws_by_weight_on_each_respawn_and_keeps_single_creature_cells()
-> Result<(), Box<dyn Error>> {
    let _clock = test_clock::set(Schedule::default(), 0);
    let table = authored_spawn_table()?;
    let mut runtime = start_runtime(table)?;
    let points = runtime.spawn_point_creatures();
    assert_eq!(points.len(), 2);
    let (shared, home, _) = points[0];
    let (single, _, _) = points[1];
    assert_eq!((shared.ordinal, single.ordinal, home.x), (0, 1, 1));

    let mut now = 0_u64;
    // This scope and the authored 1:3 weights fix the draw at every admission ordinal.
    let expected = [
        RAT, CAVE_RAT, RAT, RAT, CAVE_RAT, RAT, RAT, CAVE_RAT, RAT, RAT, CAVE_RAT, RAT, CAVE_RAT,
        RAT, RAT,
    ];
    for (ordinal, species) in expected.into_iter().enumerate() {
        let live = runtime.spawn_point_creatures();
        let actor = live[0].2.ok_or("weighted home occupant")?;
        let fixed_actor = live[1].2.ok_or("single home occupant")?;
        assert!(runtime.matches_live_creature_identity(actor, species.as_bytes()));
        assert!(runtime.matches_live_creature_identity(fixed_actor, RAT.as_bytes()));
        let health = runtime.companion_policy(species)?.maximum_health;
        assert_eq!(runtime.companion_snapshot(actor)?.health, health);
        if ordinal + 1 == expected.len() {
            break;
        }

        let occurrence = (ordinal as u64).to_be_bytes();
        runtime.commit_monster_lab_damage(actor, &occurrence, health)?;
        if ordinal == 0 {
            runtime.commit_monster_lab_damage(fixed_actor, b"single-home", 20)?;
        }
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, now);
        let due = runtime
            .pending_respawn(shared)
            .ok_or("weighted deadline")?
            .due_micros;
        assert_eq!(due, now + 30_000_000);
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, due);
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        crate::gameplay_transport::drive_owner_spawns(&mut runtime, due + 4_200_000);
        now = due + 4_250_000;
        if ordinal == 0 {
            let single_due = runtime
                .pending_respawn(single)
                .ok_or("single deadline")?
                .due_micros;
            assert_eq!(single_due, 45_000_000);
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, single_due);
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, single_due + 4_200_000);
            let replacement = runtime.spawn_point_creatures()[1]
                .2
                .ok_or("single replacement")?;
            assert_ne!(replacement, fixed_actor);
            now = single_due + 4_250_000;
        }
    }
    Ok(())
}

#[test]
fn occupied_weighted_spawn_retries_the_same_owner_draw() -> Result<(), Box<dyn Error>> {
    let _clock = test_clock::set(Schedule::default(), 0);
    let mut runtime = start_runtime(authored_spawn_table()?)?;
    let (point, home, actor) = runtime.spawn_point_creatures()[0];
    let rat = actor.ok_or("initial weighted occupant")?;
    assert!(runtime.matches_live_creature_identity(rat, RAT.as_bytes()));
    runtime.commit_monster_lab_damage(rat, b"weighted-occupancy", 20)?;
    let blocker = runtime.admit_pinned_monster_lab_creature(home, RAT, 20)?;

    crate::gameplay_transport::drive_owner_spawns(&mut runtime, 0);
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, 30_000_000);
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, 34_200_000);
    let retry = runtime
        .pending_respawn(point)
        .ok_or("occupied home retry")?;
    assert_eq!((retry.attempt, retry.due_micros), (1, 39_200_000));
    assert!(runtime.spawn_point_creatures()[0].2.is_none());

    runtime.remove_test_actor(blocker)?;
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, retry.due_micros);
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, retry.due_micros + 4_200_000);
    let replacement = runtime.spawn_point_creatures()[0]
        .2
        .ok_or("retried birth")?;
    assert_ne!(replacement, rat);
    // Ordinal 1 is Cave Rat, whereas consuming another draw would select Rat.
    assert!(runtime.matches_live_creature_identity(replacement, CAVE_RAT.as_bytes()));
    Ok(())
}
