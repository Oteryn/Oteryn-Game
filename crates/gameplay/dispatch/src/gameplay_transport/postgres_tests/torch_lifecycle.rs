use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

const START: MovementLocalPosition = MovementLocalPosition {
    x: 6,
    y: 0,
    floor: 7,
};
const TORCH: u8 = 71;
const BAG: u8 = 72;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Wire,
    Pause,
    Moves,
}

#[test]
fn torch_lights_through_ordinary_use_wire() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    on_big_stack(Case::Wire)
}

#[test]
fn torch_use_pauses_remaining_burn_and_replays_without_toggling_again() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    on_big_stack(Case::Pause)
}

#[test]
fn torch_burns_across_ground_and_nested_container_moves_and_expires_in_place() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    on_big_stack(Case::Moves)
}

fn on_big_stack(case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "torch scenario panicked")??;
    Ok(())
}

async fn read(pool: &sqlx::PgPool) -> TestResult<(String, i16)> {
    Ok(sqlx::query_as("SELECT definition_production_key,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(TORCH).as_slice()).fetch_one(pool).await?)
}

async fn timer(pool: &sqlx::PgPool) -> TestResult<(Option<i64>, Option<i64>)> {
    Ok(sqlx::query_as("SELECT deadline_ms,remaining_ms FROM game_torch_clocks WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(TORCH).as_slice()).fetch_one(pool).await?)
}

async fn shift_deadline(pool: &sqlx::PgPool, milliseconds: i64) -> TestResult {
    sqlx::query("UPDATE game_torch_clocks SET deadline_ms=deadline_ms-$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(TORCH).as_slice()).bind(milliseconds).execute(pool).await?;
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "each scenario uses the same composed Channel owner"
)]
fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let name = match case {
            Case::Wire => "torch_wire",
            Case::Pause => "torch_pause",
            Case::Moves => "torch_custody",
        };
        eprintln!("POSTGRES_CASE_STARTED {name}");
        let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = fixture_content(world)?;
        let generation = controller.active().ok_or("active content")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let map = fixture_map(world, native)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = super::rust_lottery_fixtures::bundle_action_room(
            pin.qualified_room(),
            &map,
            world,
            &cells,
        )?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (START.x, START.y, START.floor),
        );
        let frame_digest = channel_pin.frame_binding_digest();
        let deps = Dependencies::with_content(
            channel_pin,
            room.clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(deps.actor, START)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        deps.initialize(&owner).await;
        let pool = &harness.pool;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(frame_digest.as_slice()).execute(pool).await?;
        super::item_look_fixtures::seed_main_backpack(
            &harness,
            &[
                ("oteryn:item.tibia.i2920", 1),
                ("oteryn:item.tibia.i2854", 1),
            ],
        )
        .await?;
        let session = deps.admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence read")?
            .ok_or("fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let torch = ItemKey::Instance(id(TORCH));
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let use_torch = async |command_id| {
            owner
                .use_event_item(
                    deps.actor,
                    command(command_id),
                    intent.clone(),
                    Some(torch),
                    true,
                )
                .await
                .map(|result| result.map(|outcome| outcome.disposition))
                .map_err(|_| "torch use failed")
        };
        let move_torch = async |command_id, target| {
            owner
                .move_loop_item(
                    deps.actor,
                    command(command_id),
                    LoopMove {
                        item: torch,
                        count: None,
                        target,
                        slot: None,
                    },
                )
                .await
        };

        if case == Case::Wire {
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(fence);
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[4, 6, 11, 12, 15, 18, 350],
                )
                .ok_or("wire capabilities")?;
            let reference = native
                .item_index()
                .ok_or("item index")?
                .definition_ref("Item", "oteryn:item.tibia.i2920", "definition-r1")
                .ok_or("torch ref")?;
            let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                .map_err(debug)?;
            let (_, disposition, _, _) = super::rust_lottery_fixtures::exchange_with_visuals(
                &owner,
                admitted,
                1,
                &payload,
                Some(reference),
            )
            .await?;
            assert_eq!(disposition, UseDisposition::Committed);
        } else {
            assert_eq!(use_torch(1).await?, Some(UseDisposition::Committed));
        }
        assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2921".into(), 1));
        let (deadline, remaining) = timer(pool).await?;
        assert!(deadline.is_some() && remaining.is_none());

        if case == Case::Moves {
            let ground = LoopTarget::Ground(WorldTilePosition {
                x: 6,
                y: 0,
                floor: -7,
            });
            assert_eq!(move_torch(2, ground).await, LoopMoveResult::Committed);
            assert_eq!(timer(pool).await?, (deadline, None));
            assert_eq!(use_torch(3).await?, Some(UseDisposition::Committed));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2920".into(), 1));
            assert_eq!(use_torch(4).await?, Some(UseDisposition::Committed));
            let relit = timer(pool).await?;
            assert_eq!(
                move_torch(5, LoopTarget::Container(ItemKey::Instance(id(BAG)))).await,
                LoopMoveResult::Committed
            );
            assert_eq!(timer(pool).await?, relit);
            // Put the first stage 20 seconds in the past. The next stage uses
            // the original boundary, even when the deadline worker runs late.
            shift_deadline(pool, 620_000).await?;
            owner.drain_source_item_deadlines().await;
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2923".into(), 1));
            let medium = relit.0.ok_or("relit deadline")? - 320_000;
            assert_eq!(timer(pool).await?, (Some(medium), None));
            assert_eq!(use_torch(6).await?, Some(UseDisposition::Committed));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2922".into(), 1));
            assert_eq!(use_torch(7).await?, Some(UseDisposition::Committed));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2923".into(), 1));
            let relit_medium = timer(pool).await?;
            // Make the medium and short stages due together. The spent stub
            // remains in the same slot with its own donor removal duration.
            shift_deadline(pool, 600_000).await?;
            owner
                .states
                .spell_states
                .lock()
                .await
                .next_item_deadline_pass_us = 0;
            owner.drain_source_item_deadlines().await;
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2926".into(), 1));
            assert_eq!(timer(pool).await?, relit_medium);
            assert_eq!(use_torch(8).await?, Some(UseDisposition::NotPossible));
            shift_deadline(pool, 300_000).await?;
            owner
                .states
                .spell_states
                .lock()
                .await
                .next_item_deadline_pass_us = 0;
            owner.drain_source_item_deadlines().await;
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2926".into(), 2));
            let nested: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid)")
                .bind(id(TORCH).as_slice()).fetch_one(pool).await?;
            assert!(!nested);
        } else if case == Case::Pause {
            shift_deadline(pool, 120_000).await?;
            assert_eq!(use_torch(2).await?, Some(UseDisposition::Committed));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2920".into(), 1));
            let paused = timer(pool).await?;
            assert_eq!(paused.0, None);
            let (at, expires): (i64, i64) = sqlx::query_as(
                "SELECT occurred_at_ms,deadline_before_ms FROM game_torch_transitions WHERE command_id=2",
            ).fetch_one(pool).await?;
            assert_eq!(paused.1, Some(expires - at));
            assert!((1..=480_000).contains(&paused.1.ok_or("paused budget")?));
            owner.drain_source_item_deadlines().await;
            assert_eq!(timer(pool).await?, paused);
            assert_eq!(use_torch(1).await?, Some(UseDisposition::Committed));
            assert_eq!(timer(pool).await?, paused);
            assert_eq!(use_torch(3).await?, Some(UseDisposition::Committed));
            let (resumed, remaining) = timer(pool).await?;
            let at: i64 = sqlx::query_scalar(
                "SELECT occurred_at_ms FROM game_torch_transitions WHERE command_id=3",
            )
            .fetch_one(pool)
            .await?;
            assert_eq!(resumed, paused.1.map(|remaining| at + remaining));
            assert_eq!(remaining, None);
        } else if case == Case::Wire {
            use crate::gameplay_transport::connection::ControlLossResult;

            check_legacy_session_fence(pool, session).await?;
            assert_eq!(
                owner.lose_control(deps.admitted, std::time::Duration::ZERO, None).await,
                ControlLossResult::Recorded
            );
            let (state, has_transport, outcome): (i16, bool, i16) = sqlx::query_as(
                "SELECT s.session_state, s.current_transport_ref IS NOT NULL,
                    game_torch_use(encode($1,'hex')::uuid, s.game_session_id, 2, decode('01','hex'),
                        s.scope_ownership_generation, s.current_generation, NULL, NULL, NULL, NULL)
                 FROM game_durability_reconnect_sessions s
                 WHERE s.game_session_id=encode($2,'hex')::uuid",
            )
            .bind(id(TORCH).as_slice())
            .bind(session.as_bytes().as_slice())
            .fetch_one(pool)
            .await?;
            assert_eq!((state, has_transport, outcome), (1, false, 2));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2921".into(), 1));
            assert_eq!(timer(pool).await?, (deadline, None));
        }
        eprintln!("POSTGRES_CASE_COMPLETED {name}");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// The source profiles are supplied in the test activation only while the
// separate content lane owns the corresponding production profile inputs.
fn fixture_content(
    world: WorldId,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use sha2::{Digest, Sha256};
    let mut input =
        super::death_ml_loss_composed_fixtures::production_input_with_source("content-1", None)?;
    let profiles = input.item_profiles.as_mut().ok_or("profile pin")?;
    let mut document: serde_json::Value = serde_json::from_slice(&profiles.bytes)?;
    let bindings: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/bindings/items.json",
    )))?;
    let records = document["records"].as_array_mut().ok_or("profiles")?;
    let unlit = records
        .iter()
        .find(|r| r["production_binding"]["external_id"] == "2920")
        .ok_or("base torch profile")?
        .clone();
    // Crystal 201189f8 data/items/items.xml:6317-6349.
    for (id, name, weight) in [
        (2921, "lit torch", 500),
        (2922, "torch", 500),
        (2923, "lit torch", 450),
        (2924, "torch", 500),
        (2925, "lit torch", 400),
        (2926, "burnt down torch", 350),
    ] {
        let id_text = id.to_string();
        if records
            .iter()
            .any(|r| r["production_binding"]["external_id"] == id_text)
        {
            continue;
        }
        let binding = bindings["bindings"]
            .as_array()
            .ok_or("bindings")?
            .iter()
            .find(|r| r["external_id"] == id_text)
            .ok_or("exact torch source binding")?;
        let mut profile = unlit.clone();
        profile["authoring"]["item"]["key"] = format!("candidate:item/{id}").into();
        profile["production_binding"] = binding.clone();
        profile["production_definition"]["production_key"] = binding["target"]["key"].clone();
        profile["semantics"]["physical"]["value"]["weight"]["value"] = weight.into();
        profile["semantics"]["presentation"]["value"]["name"]["value"] = name.into();
        records.push(profile);
    }
    profiles.bytes = serde_json::to_vec(&document)?;
    profiles.sha256 = Sha256::digest(&profiles.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::content::native_gameplay::test_support::try_activate_input(world, &input).map_err(debug)
}

fn fixture_map(
    world: WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let index = native.item_index().ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let bag = "oteryn:item.tibia.i2854";
    let oven = "oteryn:item.tibia.i2535";
    let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
        (bag, reference(bag).ok_or("bag")?.get() - 1),
        (oven, reference(oven).ok_or("oven")?.get() - 1),
    );
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| {
            let policy = native.item_policy(key, "definition-r1")?.record();
            let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical
            else {
                return None;
            };
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.attributes.blocks_movement,
                blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                door: false,
            })
        },
    )
    .map_err(debug)
}


async fn check_legacy_session_fence(
    pool: &sqlx::PgPool,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    // Exercise the pre-fresh session encoding against real torch custody and
    // character guards. Roll back the fixture rewrite and the torch transition.
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions
         SET fresh_replay_key=NULL, initial_fresh_replay_key=NULL, session_state=1,
             control_loss_epoch=1, original_grace_deadline=0, predecessor_generation=current_generation
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    let use_torch = "SELECT game_torch_use(encode($1,'hex')::uuid, s.game_session_id,
            $3::int, decode('01','hex'), s.scope_ownership_generation, s.current_generation,
            NULL, NULL, NULL, NULL)
        FROM game_durability_reconnect_sessions s
        WHERE s.game_session_id=encode($2,'hex')::uuid";
    let outcome: i16 = sqlx::query_scalar(use_torch)
        .bind(id(TORCH).as_slice())
        .bind(session.as_bytes().as_slice())
        .bind(2_i32)
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(outcome, 1, "active legacy session can extinguish its lit torch");
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions SET session_state=2, current_transport_ref=NULL
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    let outcome: i16 = sqlx::query_scalar(use_torch)
        .bind(id(TORCH).as_slice())
        .bind(session.as_bytes().as_slice())
        .bind(3_i32)
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(outcome, 2, "reconnectable legacy session cannot relight the torch");
    tx.rollback().await?;
    Ok(())
}
