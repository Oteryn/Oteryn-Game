//! Demon Oak reward access and mutually exclusive claims through the composed USE dispatcher.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, UseCommand};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::{
    boot::{BootPins, BundleWorld},
    overlay::TilePos,
};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.demon_oak";
const DONE: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";
const GRAVE: TilePos = TilePos {
    x: 32710,
    y: 32398,
    floor: -7,
};
const LANDING: TilePos = TilePos {
    x: 32714,
    y: 32398,
    floor: -8,
};
const CHESTS: [(TilePos, u16); 4] = [
    (
        TilePos {
            x: 32710,
            y: 32393,
            floor: -8,
        },
        3389,
    ),
    (
        TilePos {
            x: 32712,
            y: 32393,
            floor: -8,
        },
        8077,
    ),
    (
        TilePos {
            x: 32714,
            y: 32393,
            floor: -8,
        },
        14768,
    ),
    (
        TilePos {
            x: 32716,
            y: 32393,
            floor: -8,
        },
        14769,
    ),
];

#[test]
fn reported_victory_opens_the_unmarked_gravestone() -> TestResult {
    scenario("qcx_oak_grave", None)
}
#[test]
fn demon_legs_uses_the_shared_reward_choice() -> TestResult {
    scenario("qcx_oak_legs", Some(0))
}
#[test]
fn rainbow_shield_uses_the_shared_reward_choice() -> TestResult {
    scenario("qcx_oak_shield", Some(1))
}
#[test]
fn thorn_spitter_uses_the_shared_reward_choice() -> TestResult {
    scenario("qcx_oak_spitter", Some(2))
}
#[test]
fn arcana_uses_the_shared_reward_choice() -> TestResult {
    scenario("qcx_oak_arcana", Some(3))
}

fn scenario(name: &'static str, choice: Option<usize>) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
        let fresh = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let (controller, activation) =
            super::death_ml_loss_composed::production_generation_with_pin(
                admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let start = match choice {
            Some(index) => TilePos {
                y: 32394,
                ..CHESTS[index].0
            },
            None => TilePos {
                x: GRAVE.x - 1,
                ..GRAVE
            },
        };
        let world = reward_world(start, admitted.world_id, admitted.channel_id)?;
        let cells = world
            .movement_cells(activation.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = activation.frame_binding().digest();
        let pin = ChannelContentPin::from_activation(
            admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (start.x.into(), start.y.into(), -i16::from(start.floor)),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            admitted.world_id,
            admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(admitted.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, local(start))
            .map_err(debug)?;
        admitted.runtime_actor = Some(actor);
        deps.runtime = tokio::sync::Mutex::new(runtime);

        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(Arc::new(
            load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        let native = generation.native_gameplay().ok_or("native items")?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let backpack = crate::durability::starter_grant::StarterTemplate {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &NativeItemWeights(native),
                &backpack,
            )
            .await
            .map_err(debug)?;
        owner.admit_quest_session(&admitted).await;
        let target = choice.map_or(GRAVE, |index| CHESTS[index].0);
        assert_ne!(
            use_at(&owner, admitted, target, 1).await?,
            UseDisposition::Committed
        );

        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let seeded = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,2,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(DONE).bind(QUEST).execute(&mut *tx).await?;
        assert_eq!(seeded.rows_affected(), 1);
        tx.commit().await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        assert_eq!(
            use_at(&owner, admitted, target, 2).await?,
            UseDisposition::Committed
        );

        if let Some(index) = choice {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            assert_ne!(
                use_at(&owner, admitted, target, 3).await?,
                UseDisposition::Committed
            );
            for (ci, (_, item)) in CHESTS.iter().enumerate() {
                let count: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
                    .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
                assert_eq!(count, i64::from(ci == index), "item {item}");
            }
            let value: i64 = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
            )
            .bind(DONE)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(value, 3, "all four options share Done 2 → 3");
        } else {
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(LANDING)
            );
        }
        Ok(())
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} {:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn local(pos: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: pos.x.into(),
        y: pos.y.into(),
        floor: -i16::from(pos.floor),
    }
}

async fn use_at(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pos: TilePos,
    command_id: u64,
) -> TestResult<UseDisposition> {
    let map = owner.bundle_map().ok_or("bundle map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    let ordinal = if pos == GRAVE { 1 } else { 2 };
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)
        .ok_or("world object placement")?;
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("current item fence")?;
    let outcome = owner
        .use_world_object(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: 0,
            },
        )
        .await;
    Ok(outcome.disposition)
}

fn reward_world(
    start: TilePos,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<BundleWorld> {
    use oteryn_world_bundle::{bundle, sector, writer};
    use std::collections::BTreeMap;
    let (fixture, mut pins) = crate::map::boot::tests::bundle();
    let mut manifest = bundle::read(&fixture)?.manifest;
    manifest.world = bundle::Extent {
        min_x: 32709,
        min_y: 32393,
        max_x: 32717,
        max_y: 32399,
        floors: vec![-8, -7],
    };
    manifest.palette.truncate(1);
    for (id, appearance) in [(1, 1979), (2, 2187), (3, 2472)] {
        manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            id,
            family: bundle::Family::Terrain,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let mut cells = BTreeMap::new();
    cells.insert((LANDING.floor, LANDING.y, LANDING.x), vec![0]);
    cells.insert((GRAVE.floor, GRAVE.y, GRAVE.x - 1), vec![0]);
    cells.insert((GRAVE.floor, GRAVE.y, GRAVE.x), vec![0, 1]);
    for (pos, _) in CHESTS {
        cells.insert((pos.floor, pos.y, pos.x), vec![0, 2, 3]);
        cells.insert((pos.floor, pos.y + 1, pos.x), vec![0]);
    }
    let mut sectors = BTreeMap::<_, Vec<_>>::new();
    for ((floor, y, x), stack) in cells {
        sectors
            .entry((floor, x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: stack
                    .into_iter()
                    .map(|palette| sector::Item {
                        palette,
                        depth: 0,
                        attrs: sector::Attrs {
                            text: (palette == 1).then(|| "Yesim Adeit".into()),
                            ..Default::default()
                        },
                    })
                    .collect(),
            });
    }
    let mut sectors = sectors
        .into_iter()
        .map(|((floor, sx, sy), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    let bytes = writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        world,
        channel,
        |_| None,
    )
    .map_err(Into::into)
}
