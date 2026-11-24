//! Discovery candidate sampling through native Find and committed STEP.
use super::cyclopedia_fixtures::support;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, maintain_ready, prepare_current_session, reset};
use crate::domain::cyclopedia::{BasicArea, Exploration, Point};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, NodeId};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::boot::{BootPins, BundleWorld, boot};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, encode_step_intent, encode_step_result,
};
use oteryn_world_bundle::{
    bundle,
    sector::{Attrs, Item, Tile},
    subarea::{Area, Mask, Run, Table},
    writer::{self, Sector},
};

const ENTRY: TilePos = TilePos {
    x: 8,
    y: 6,
    floor: -7,
};

#[test]
fn discovery_samples_beyond_visible_prefix() -> TestResult {
    postgres_case(false, "discovery_samples_beyond_visible_prefix")
}

#[test]
fn discovery_retains_existing_pois_across_step_and_relogin() -> TestResult {
    postgres_case(
        true,
        "discovery_retains_existing_pois_across_step_and_relogin",
    )
}

fn postgres_case(retain: bool, name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || scenario(retain, name))
}

fn scenario(retain: bool, name: &str) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_poi_sampling", true, false)
            .await?;
        eprintln!("POSTGRES_STARTED W12 {name}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activation_pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let world = sampling_world(&deps, generation)?;
        let cells = world
            .movement_cells(activation_pin.qualified_room().movement_cells())
            .map_err(debug)?;
        support::seed(&h, world.base().digest()).await?;

        let server = generation.identity().server_artifact_digest();
        let client = generation.identity().client_artifact_digest();
        let frame = activation_pin.frame_binding().digest();
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            server,
            client,
            frame,
            world.map_revision_digest(),
            (
                i32::from(ENTRY.x),
                i32::from(ENTRY.y),
                i16::from(-ENTRY.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
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
        if retain {
            let mut movement = runtime.borrow_movement_position();
            let mut current = movement.read(deps.actor).map_err(debug)?;
            for _ in 0..8 {
                let at = current.position();
                current = movement
                    .commit_cardinal(
                        current,
                        crate::foundation::MovementLocalPosition { x: at.x - 1, ..at },
                    )
                    .map_err(debug)?;
            }
            for _ in 0..6 {
                let at = current.position();
                current = movement
                    .commit_cardinal(
                        current,
                        crate::foundation::MovementLocalPosition { y: at.y + 1, ..at },
                    )
                    .map_err(debug)?;
            }
        }
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.combat_room = Some(
            activation_pin
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: i32::from(ENTRY.x),
                        y: i32::from(ENTRY.y),
                        floor: i16::from(-ENTRY.floor),
                    },
                )
                .ok_or("discovery combat room")?,
        );
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = (&cells).into();
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;

        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offers = crate::gameplay_transport::cyclopedia::offered(
            owner.offered_capabilities(),
            owner.cyclopedia_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[4, 6, 57]).ok_or("discovery capabilities")?;

        let hidden: Vec<_> = (0..7)
            .map(|x| Point {
                x,
                y: 13,
                floor: -7,
            })
            .collect();
        let find = [3, 1, 0, 0, 0, 0, 0, 0, 0];
        if retain {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_world_reset_epochs SET server_digest=$1,client_digest=$2,frame_digest=$3")
                .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_cyclopedia(character_id,bundle_digest,progress,occurrence,catalogue_digest) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5)")
                .bind(id(41).as_slice()).bind(world.base().digest().as_slice())
                .bind(serde_json::json!({"legacy": {"discovered": false, "exploration": null},
                    "subareas": {"2": {"discovered": false, "exploration": {"points": hidden, "found": 64}}}}))
                .bind(id(99).as_slice()).bind(BasicArea::content_digest().as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root
                .initialize_admission_equipment(
                    &authority,
                    &h.node,
                    &admitted.item_fence.ok_or("equipment fence")?,
                    server,
                    fresh.operation(),
                )
                .await
                .map_err(debug)?;

            let actual = support::exchange(
                &owner,
                &mut admitted,
                1,
                1,
                &encode_step_intent(StepDirection::South),
            )
            .await?;
            assert_eq!(
                actual,
                (
                    CommandStatus::Accepted,
                    encode_step_result(StepDisposition::Moved)
                )
            );
            let expected = Exploration {
                points: hidden,
                found: 65,
            };
            assert_eq!(exploration(&h).await?, expected);

            // The STEP socket is closed by exchange. Rebuild the connection's owner and
            // run native Find from a new admitted socket against the same character row.
            let commands = std::mem::take(
                &mut std::sync::Arc::get_mut(&mut owner.states)
                    .expect("isolated dispatch state")
                    .command_ingress,
            );
            drop(owner);
            let mut owner = deps.owner(&h, &authority);
            std::sync::Arc::get_mut(&mut owner.states)
                .expect("isolated dispatch state")
                .command_ingress = commands;
            owner.movement_cells = (&cells).into();
            owner.active_generation = Some(generation);
            deps.initialize(&owner).await;
            maintain_ready(&h.root).await?;
            let (status, reply) = support::exchange(&owner, &mut admitted, 2, 73, &find).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!((reply[0], reply[10]), (0, 2));
            assert_eq!(exploration(&h).await?, expected);
        } else {
            let (status, reply) = support::exchange(&owner, &mut admitted, 1, 73, &find).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                reply[0], 0,
                "native Find must generate seven hidden viewpoints"
            );
            let mut saved = exploration(&h).await?;
            saved.points.sort_unstable();
            assert_eq!(
                saved,
                Exploration {
                    points: hidden,
                    found: 0
                }
            );
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn exploration(h: &Harness) -> TestResult<Exploration> {
    let json: serde_json::Value = sqlx::query_scalar(
        "SELECT progress->'subareas'->'2'->'exploration' FROM game_character_cyclopedia",
    )
    .fetch_one(&h.pool)
    .await?;
    Ok(serde_json::from_value(json)?)
}

fn sampling_world(
    deps: &Dependencies,
    generation: &crate::content::ActiveGeneration,
) -> TestResult<BundleWorld> {
    let index = generation
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("item index")?;
    let backpack = "oteryn:item.tibia.i2854";
    let gold = "oteryn:item.tibia.i3031";
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (
            backpack,
            reference(backpack).ok_or("backpack definition")?.get() - 1,
        ),
        (gold, reference(gold).ok_or("coin definition")?.get() - 1),
    );
    let mut parsed = bundle::read(&bytes)?;
    // All 221 tiles on rows 0–12 are in view from ENTRY. Row 13 has exactly
    // seven more walkable tiles in the same admitted subarea.
    let runs: Vec<_> = (0..14)
        .map(|y| Run {
            floor: -7,
            y,
            begin: 0,
            end: if y == 13 { 7 } else { 17 },
            subarea: 2,
        })
        .collect();
    let tiles = runs
        .iter()
        .flat_map(|run| {
            (run.begin..run.end).map(move |x| Tile {
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
        .collect();
    parsed.sectors = vec![Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles,
    }];
    let hash = "a".repeat(64);
    parsed.manifest.subareas = Table {
        client_version: "15.30".into(),
        candidate_sha256: hash.clone(),
        areas: [1, 2]
            .map(|id| Area {
                id,
                key: format!("oteryn:content.area.fixture{id}"),
                name: format!("Area {id}"),
                parent: (id == 2).then_some(1),
                source_record_sha256: hash.clone(),
                town_id: None,
                hometown: false,
                temple: None,
            })
            .to_vec(),
        masks: vec![Mask {
            subarea: 2,
            asset: format!("content/assets/files/subarea-0002-{hash}.bmp.lzma"),
            compressed_sha256: hash.clone(),
            bmp_sha256: hash.clone(),
            source_record_sha256: hash,
            source_tiles: 228,
            admitted_tiles: 228,
        }],
        runs,
        ..Table::default()
    };
    let bytes = writer::write(&parsed.manifest, &parsed.sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: ENTRY,
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: true,
                door: false,
            })
        },
    )
    .map_err(debug)
}
