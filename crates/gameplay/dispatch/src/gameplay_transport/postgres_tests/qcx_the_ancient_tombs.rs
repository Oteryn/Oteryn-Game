//! The seven-flame gate accepts returning characters without reducing their progress.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{loader, world_callbacks::Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_ancient_tombs_quest";
const PREFIX: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/";

#[test]
fn morguthis_seven_flames_preserve_higher_imported_progress() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "qcx_ancient_tombs", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("content generation")?;
        let world =
            super::l6_native_flames::world(generation.native_gameplay().ok_or("native content")?)?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let origin = local(Position {
            x: 33163,
            y: 32696,
            floor: -14,
        });
        let goal = local(Position {
            x: 33163,
            y: 32694,
            floor: -14,
        });
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: origin.x,
                    y: origin.y,
                    floor: origin.floor,
                },
            )
            .ok_or("tomb bundle room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
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
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);

        let treasure = format!("{PREFIX}morguthis_treasure");
        // A restored character has all seven flames and a value above the door's stage.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT encode($1,'hex')::uuid,$2||'morguthis_blue_flame_storage'||n::text,$3,1,2 FROM generate_series(1,7) n")
            .bind(id(41).as_slice()).bind(PREFIX).bind(QUEST).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_character_quest_tracks VALUES(encode($1,'hex')::uuid,$2,$3,3,2)",
        )
        .bind(id(41).as_slice())
        .bind(&treasure)
        .bind(QUEST)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
        owner.quest_catalogue = Some(Arc::new(
            loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        let result = wire(
            &owner,
            &mut admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::North),
        )
        .await?;
        assert_eq!(
            decode_step_result(&result).map_err(debug)?,
            StepDisposition::Moved
        );
        let actual = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            actual, goal,
            "the gate admits a character with all seven flags"
        );
        let stored: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(41).as_slice()).bind(&treasure).fetch_one(&h.pool).await?;
        assert_eq!(stored, 3, "gate must preserve higher treasure progress");
        Ok(())
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED morguthis_seven_flames duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
