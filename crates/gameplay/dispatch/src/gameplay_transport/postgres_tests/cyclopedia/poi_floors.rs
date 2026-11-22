use super::*;
use crate::domain::cyclopedia::{Exploration, Point};
use crate::gameplay_transport::connection::regression_routed_step;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::{
    bundle,
    sector::{Attrs, Item, Tile},
    subarea::{Area, Mask, Run, Table},
    writer::{self, Sector},
};

#[test]
fn w12_rookgaard_step_samples_viewpoints_only_on_source_poi_floors() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_on_stack("rookgaard_poi_floors", false)
}

#[test]
fn w12_floor_override_still_keeps_hidden_viewpoint_constraint() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_on_stack("rookgaard_poi_hidden", true)
}

fn run_on_stack(name: &'static str, check_hidden_window: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| scenario(admin, name, check_hidden_window).await)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Rookgaard viewpoint scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String, name: &str, check_hidden_window: bool) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activation_pin) =
        super::super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active generation")?;
    let floor = if check_hidden_window { 7 } else { 9 };
    let world = world(&deps, generation, -floor)?;
    let cells = world
        .movement_cells(activation_pin.qualified_room().movement_cells())
        .map_err(debug)?;
    support::seed(&h, world.base().digest()).await?;
    let frame = activation_pin.frame_binding().digest();
    let server = generation.identity().server_artifact_digest();
    let client = generation.identity().client_artifact_digest();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_world_reset_epochs SET server_digest=$1,client_digest=$2,frame_digest=$3",
    )
    .bind(server.as_slice())
    .bind(client.as_slice())
    .bind(frame.as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        server,
        client,
        frame,
        world.map_revision_digest(),
        (1, 0, i16::from(floor)),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);

    let mut owner = deps.owner(&h, &authority);
    owner.movement_cells = (&cells).into();
    owner.active_generation = Some(generation);
    deps.initialize(&owner).await;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?;
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);

    eprintln!("POSTGRES_STARTED W12 {name}");
    super::super::maintain_ready(owner.root).await?;
    let outcome = regression_routed_step(&owner, admitted, deps.actor, StepDirection::East).await?;
    assert_eq!(outcome.disposition, StepDisposition::Moved);

    if check_hidden_window {
        // At x=2 the x=10 viewpoint is still inside the view. Only the six on floor 8 are hidden.
        assert!(exploration(&h).await?.is_none());
        super::super::maintain_ready(owner.root).await?;
        let outcome =
            regression_routed_step(&owner, admitted, deps.actor, StepDirection::West).await?;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
    }
    let exploration = exploration(&h)
        .await?
        .ok_or("STEP must generate Rookgaard viewpoints")?;
    assert_eq!(exploration.points.len(), 7);
    assert!(
        exploration
            .points
            .iter()
            .all(|p| matches!(p.floor, -8 | -7))
    );
    assert!(exploration.points.iter().any(|p| p.floor == -7));
    assert!(exploration.points.iter().any(|p| p.floor == -8));
    if check_hidden_window {
        let boundary = Point {
            x: 10,
            y: 0,
            floor: -7,
        };
        assert!(exploration.points.contains(&boundary));
        assert_eq!(
            exploration.points.iter().filter(|p| p.floor == -8).count(),
            6
        );
    }
    assert_eq!(exploration.found, 0);
    Ok(())
}

async fn exploration(h: &Harness) -> TestResult<Option<Exploration>> {
    let stored: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT progress->'subareas'->'317'->'exploration' FROM game_character_cyclopedia",
    )
    .fetch_optional(&h.pool)
    .await?
    .flatten();
    Ok(stored
        .filter(|value| !value.is_null())
        .map(serde_json::from_value)
        .transpose()?)
}

fn world(
    deps: &Dependencies,
    generation: &crate::content::ActiveGeneration,
    start_floor: i8,
) -> TestResult<crate::map::boot::BundleWorld> {
    const ROOKGAARD: u32 = 317;
    const THAIS: u32 = 24;
    const BMP_SHA: &str = "c6a580008af72eb6f42ab0cb0835e00c3abfc3b47ad6a85852356086db6dc8c2";
    let index = generation
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("item index")?;
    let item = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let pack_key = "oteryn:item.tibia.i2854";
    let coin_key = "oteryn:item.tibia.i3031";
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (
            pack_key,
            item(pack_key).ok_or("backpack definition")?.get() - 1,
        ),
        (coin_key, item(coin_key).ok_or("coin definition")?.get() - 1),
    );
    let mut parsed = bundle::read(&bytes)?;

    // Floor 9 fills the old 128-tile pool before either eligible floor is visited.
    // Floor 8 offers six viewpoints. Floor 7 adds the walking lane and one boundary point.
    let runs: Vec<_> = (0..4)
        .map(|y| (-9, y, 0, 32))
        .chain([(-8, 0, 1, 7), (-7, 0, 1, 3), (-7, 0, 10, 11)])
        .map(|(floor, y, begin, end)| Run {
            floor,
            y,
            begin,
            end,
            subarea: ROOKGAARD,
        })
        .collect();
    let floors = vec![-9, -8, -7];
    parsed.sectors = floors
        .iter()
        .map(|&floor| Sector {
            floor,
            sx: 0,
            sy: 0,
            tiles: runs
                .iter()
                .filter(|run| run.floor == floor)
                .flat_map(|run| {
                    (run.begin..run.end).map(|x| Tile {
                        x,
                        y: run.y,
                        flags: 0,
                        house: 0,
                        zones: vec![],
                        items: vec![Item {
                            palette: 0,
                            depth: 0,
                            attrs: Attrs::default(),
                        }],
                    })
                })
                .collect(),
        })
        .collect();
    parsed.manifest.world.floors = floors;
    parsed.manifest.subareas = Table {
        client_version: "15.30".into(),
        candidate_sha256: "b3488ba592d9e65e3c15ea7ff7752eb4b6596f0fa8d31afc45befc17ee4843f9".into(),
        areas: vec![
            Area {
                id: THAIS,
                key: "oteryn:content.area.region.thais".into(),
                name: "Thais".into(),
                parent: None,
                source_record_sha256:
                    "8a160e089e3666b8d26f5ce93cd3b099d2f5cda00ee08fbfea86f2cdd9f9f294".into(),
                town_id: None,
                hometown: false,
                temple: None,
            },
            Area {
                id: ROOKGAARD,
                key: "oteryn:content.area.city.rookgaard".into(),
                name: "Rookgaard".into(),
                parent: Some(THAIS),
                source_record_sha256:
                    "36c89b7a1e988f6cc16c82278317ee1d141d9cce6af0ca9bac007f8e00517f5a".into(),
                town_id: None,
                hometown: false,
                temple: None,
            },
        ],
        masks: vec![Mask {
            subarea: ROOKGAARD,
            asset: format!("content/assets/files/subarea-0317-{BMP_SHA}.bmp.lzma"),
            compressed_sha256: "2db037631a06816f83a315a60413bd5993f5819a60795e6bbdaae511ea101371"
                .into(),
            bmp_sha256: BMP_SHA.into(),
            source_record_sha256:
                "d2cd00897a8ae624eb2075d19256749137e16194e8da79d49d2c59f3e2a8803b".into(),
            source_tiles: 29_134,
            admitted_tiles: runs.iter().map(|run| u32::from(run.end - run.begin)).sum(),
        }],
        runs,
        ..Table::default()
    };
    let bytes = writer::write(&parsed.manifest, &parsed.sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 1,
                y: 0,
                floor: start_floor,
            },
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: item(key)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: true,
                door: false,
            })
        },
    )
    .map_err(debug)
}
