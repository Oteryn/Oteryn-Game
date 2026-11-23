use std::collections::{BTreeMap, BTreeSet};

use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, GameSessionId, MovementLocalPosition, NodeId};
use crate::map::{boot, overlay::TilePos};
use oteryn_gameplay_world_change::variants::{self, Position, Selected};
use oteryn_world_bundle::{bundle, sector};

#[test]
fn warpath_occupied_landing_leaves_no_admission_or_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run("warpath")
}

#[test]
fn spider_nest_occupied_landing_leaves_no_admission_or_visit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run("spider_nest")
}

fn run(key: &'static str) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "blocked_daily_passage", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let selection = (1..1_000).find_map(|epoch| {
            let selected = Selected::roll(deps.admitted.world_id.as_bytes(), epoch).ok()?;
            let (definition, portal) = selected.active().filter(|(d, _)| d.key == key)
                .find_map(|(d, v)| v.portals.iter().find(|portal| !portal.exit).map(|p| (d, p)))?;
            Some((epoch, selected, definition, portal))
        }).ok_or("active entrance")?;
        let (epoch, selected, definition, portal) = selection;
        let metadata = [7_u8; 32];
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'1970-01-01T06:00:00Z','1970-01')")
            .bind(deps.admitted.world_id.as_bytes().as_slice()).bind(epoch.to_string()).bind(metadata.as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)")
            .bind(deps.admitted.world_id.as_bytes().as_slice()).bind(epoch.to_string()).execute(&mut *tx).await?;
        crate::durability::world_event::changes::finalize_reset_for_test(&mut tx, deps.admitted.world_id, epoch).await?;
        tx.commit().await?;

        let (controller, activation) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id, "content-1",
        )?;
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let index = native.item_index().ok_or("item index")?;
        let origin = (portal.position.0 - 1, portal.position.1, portal.position.2);
        let (bytes, mut pins) = passage_bundle(definition, origin)?;
        pins.digest = bundle::read(&bytes)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let checked = boot::check(bytes, boot::BootPins {
            start: TilePos { x: origin.0, y: origin.1, floor: -origin.2 },
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
        }).map_err(debug)?.compose_epoch_admitted_layers(epoch, &[], &[]).map_err(debug)?;
        let map = checked.with_daily_map(crate::map::daily::DailyMap {
            seed: metadata, fungus: false, grimvale_full_moon: false, items: Vec::new(),
            trader: None, variants: selected,
            unavailable_variants: variants::definitions()?.iter()
                .filter(|d| d.key != key).map(|d| d.key.as_str()).collect(),
        }).boot(deps.admitted.world_id, deps.admitted.channel_id, |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: Some(false), blocks_projectile: false, pickupable: false, door: false,
            })
        }).map_err(debug)?;
        assert!(map.facts().daily_variant_passage(portal.position));
        for (x, y, floor) in [origin, portal.position, portal.destination] {
            assert!(map.enterable(TilePos { x, y, floor: -floor }), "fixture cell {x},{y},{floor} must be walkable");
        }
        let cells = map.movement_cells(activation.qualified_room().movement_cells()).map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id, 1, active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(), map.frame_binding_digest(deps.admitted.world_id),
            map.map_revision_digest(), (i32::from(origin.0), i32::from(origin.1), i16::from(origin.2)),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id, deps.admitted.channel_id, NodeId::decode(&id(1)).map_err(debug)?,
            1, 1, 1, "runtime-scope-assignment:1", 2, pin,
        ).map_err(debug)?;
        let session = deps.admitted.game_session_id;
        let player = runtime.reserve_fresh_session(session).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(player).map_err(debug)?;
        let origin_snapshot = runtime.initialize_pinned_test_position(deps.actor, local(origin)).map_err(debug)?;
        let blocker = runtime.reserve_fresh_session(GameSessionId::decode(&id(87)).map_err(debug)?).map_err(debug)?;
        let blocker = runtime.commit_fresh_session(blocker).map_err(debug)?;
        runtime.initialize_pinned_test_position(blocker, local(portal.destination)).map_err(debug)?;
        let no_doors = crate::map::floor::BundleDoors(map.facts().doors());
        let enter = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, &cells, deps.actor, session, origin_snapshot, local(portal.position), false, &no_doors,
        ).map_err(debug)?;
        let entered = runtime.commit_source_step(enter).map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        owner.reset_epoch = epoch;
        deps.initialize(&owner).await;
        let fence = owner.current_quest_fence(session).await.map_err(|()| "session fence")?.ok_or("session fence")?;
        owner.runtime.lock().await.bind_attacker_lease(
            deps.actor, session, crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            ).map_err(debug)?,
        ).map_err(debug)?;
        let permit = owner.spell_lane_permit().await.ok_or("movement lane")?;
        assert_eq!(owner.daily_variant_step(&permit, deps.actor, session, origin_snapshot, entered).await.position(), local(origin));
        let recorded: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_daily_variant_entries),
             (SELECT count(*) FROM game_daily_variant_warpath),
             (SELECT coalesce(sum(visits),0)::bigint FROM game_daily_variant_spider_progress)",
        ).fetch_one(&h.pool).await?;
        assert_eq!(recorded, (0, 0, 0));
        drop(permit);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn local((x,y,z): Position) -> MovementLocalPosition {
    MovementLocalPosition { x: i32::from(x), y: i32::from(y), floor: i16::from(z) }
}

fn passage_bundle(definition: &variants::Definition, origin: Position) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    let (bytes, pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    let mut ground = data.manifest.palette.iter()
        .find(|entry| entry.key == "terrain:grass").ok_or("test ground")?.clone();
    data.manifest.palette = vec![ground.clone()];
    // The source Spider Nest layer replaces the entire entrance tile with this ground.
    ground.key = "oteryn:terrain.tibia.i432".into();
    ground.id = 1;
    data.manifest.palette.push(ground);
    let positions: BTreeSet<_> = definition.inactive_tiles.iter().map(|t| t.position)
        .chain(definition.variants.iter().flat_map(|v| {
            v.tiles.iter().map(|t| t.position).chain(v.spawns.iter().map(|s| s.position))
                .chain(v.npcs.iter().map(|n| n.position)).chain(v.barriers.iter().copied())
                .chain(v.portals.iter().flat_map(|p| [p.position, p.destination]))
        })).chain([origin]).collect();
    let mut groups = BTreeMap::<_, Vec<_>>::new();
    for &(x,y,z) in &positions {
        groups.entry((-z,y/32,x/32)).or_default().push(sector::Tile {
            x,y,flags:0,house:0,zones:vec![],
            items:vec![sector::Item { palette:0,depth:0,attrs:Default::default() }],
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: positions.iter().map(|p| p.0).min().ok_or("x minimum")?,
        min_y: positions.iter().map(|p| p.1).min().ok_or("y minimum")?,
        max_x: positions.iter().map(|p| p.0).max().ok_or("x maximum")? + 1,
        max_y: positions.iter().map(|p| p.1).max().ok_or("y maximum")? + 1,
        floors: positions.iter().map(|p| -p.2).collect::<BTreeSet<_>>().into_iter().collect(),
    };
    let mut sectors: Vec<_> = groups.into_iter().map(|((floor,sy,sx),mut tiles)| {
        tiles.sort_by_key(|t| (t.y,t.x));
        bundle::Sector { floor,sy,sx,tiles }
    }).collect();
    sectors.sort_by_key(bundle::order);
    Ok((oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,&sectors,&Default::default(),&Default::default(),
    )?, pins))
}
