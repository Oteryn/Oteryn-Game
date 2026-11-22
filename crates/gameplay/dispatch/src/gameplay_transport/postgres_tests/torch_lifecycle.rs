use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{MovementLocalPosition, WorldId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

const START: MovementLocalPosition = MovementLocalPosition { x: 7, y: 0, floor: 7 };
const TORCH: u8 = 71;
const BAG: u8 = 72;

#[test]
fn torch_use_pauses_remaining_burn_and_replays_without_toggling_again() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    on_big_stack(false)
}

#[test]
fn torch_burns_across_ground_and_nested_container_moves_and_expires_in_place() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    on_big_stack(true)
}

fn on_big_stack(moves: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(moves).map_err(|error| error.to_string()))?
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

fn scenario(moves: bool) -> TestResult {
    run(async |admin| {
        let name = if moves { "torch_custody" } else { "torch_pause" };
        let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active content")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(), room, crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime.lock().await.initialize_pinned_test_position(deps.actor, START).map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
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
        super::item_look_fixtures::seed_main_backpack(&harness, &[("oteryn:item.tibia.i2920", 1), ("oteryn:item.tibia.i2854", 1)]).await?;
        let session = deps.admitted.game_session_id;
        let fence = owner.current_quest_fence(session).await.map_err(|_| "fence read")?.ok_or("fence")?.item_fence();
        let command = |command_id| UseCommand { game_session_id: session, command_id, item_fence: Some(fence) };
        let torch = ItemKey::Instance(id(TORCH));
        let use_torch = async |command_id| {
            let intent = UseIntent {
                target: UseTarget::Item(std::num::NonZeroU64::MIN),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            };
            owner.use_event_item(deps.actor, command(command_id), intent, Some(torch), true)
                .await.map(|result| result.map(|outcome| outcome.disposition))
                .map_err(|_| "torch use failed")
        };
        let move_torch = async |command_id, target| {
            owner.move_loop_item(deps.actor, command(command_id), LoopMove { item: torch, count: None, target, slot: None }).await
        };

        assert_eq!(use_torch(1).await?, Some(UseDisposition::Committed));
        assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2921".into(), 1));
        let (deadline, remaining) = timer(pool).await?;
        assert!(deadline.is_some() && remaining.is_none());

        if moves {
            let ground = LoopTarget::Ground(WorldTilePosition { x: 7, y: 0, floor: -7 });
            assert_eq!(move_torch(2, ground).await, LoopMoveResult::Committed);
            assert_eq!(timer(pool).await?, (deadline, None));
            assert_eq!(use_torch(3).await?, Some(UseDisposition::Committed));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2920".into(), 1));
            assert_eq!(use_torch(4).await?, Some(UseDisposition::Committed));
            let relit = timer(pool).await?;
            assert_eq!(move_torch(5, LoopTarget::Container(ItemKey::Instance(id(BAG)))).await, LoopMoveResult::Committed);
            assert_eq!(timer(pool).await?, relit);
            // Model a delayed deadline pass without sleeping or altering the production clock.
            shift_deadline(pool, 1_500_000).await?;
            owner.drain_source_item_deadlines().await;
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2926".into(), 2));
            let nested: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid)")
                .bind(id(TORCH).as_slice()).fetch_one(pool).await?;
            assert!(!nested);
        } else {
            shift_deadline(pool, 120_000).await?;
            assert_eq!(use_torch(2).await?, Some(UseDisposition::Committed));
            assert_eq!(read(pool).await?, ("oteryn:item.tibia.i2920".into(), 1));
            let paused = timer(pool).await?;
            assert_eq!(paused.0, None);
            assert!((470_000..=480_000).contains(&paused.1.ok_or("paused budget")?));
            owner.drain_source_item_deadlines().await;
            assert_eq!(timer(pool).await?, paused);
            assert_eq!(use_torch(1).await?, Some(UseDisposition::Committed));
            assert_eq!(timer(pool).await?, paused);
            assert_eq!(use_torch(3).await?, Some(UseDisposition::Committed));
            let (resumed, remaining) = timer(pool).await?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(pool).await?;
            let resumed = resumed.ok_or("resumed deadline")? - now;
            let paused = paused.1.ok_or("paused budget")?;
            assert!((paused - 10_000..=paused).contains(&resumed));
            assert_eq!(remaining, None);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
