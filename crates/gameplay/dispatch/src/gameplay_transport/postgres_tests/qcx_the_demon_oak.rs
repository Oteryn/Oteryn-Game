//! Exercise the Yesim Adeit gravestone as projected in the production map.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::quest_world_callbacks::{local, wire, world};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NativeEntryStart, channel_content_pin::ChannelContentPin};
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state,
    world_callbacks::{self, Position},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use std::sync::Arc;

const GRAVE: Position = Position {
    x: 32710,
    y: 32398,
    floor: -7,
};
const REWARD_ROOM: Position = Position {
    x: 32714,
    y: 32398,
    floor: -8,
};
const DONE: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";

#[test]
fn reported_victory_opens_the_unmarked_gravestone() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "qcx_demon_oak_grave", false, false).await?;
        prepare_current_session(&h).await?;
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
        let mut map_grave = world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/demon-oak-reward-grave")
            .ok_or("grave callback")?;
        // Project the real appearance and absence of a UID regardless of the callback under test.
        map_grave.selector.unique = None;
        map_grave.selector.appearance = Some(1979);
        let world = world(&map_grave, GRAVE)?;
        let cells = world
            .movement_cells(activation.qualified_room().movement_cells())
            .map_err(debug)?;
        let start = Position {
            x: GRAVE.x - 1,
            ..GRAVE
        };
        let frame = activation.frame_binding().digest();
        let room = activation
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                NativeEntryStart {
                    x: start.x.into(),
                    y: start.y.into(),
                    floor: -i16::from(start.floor),
                },
            )
            .ok_or("reward room bundle")?;
        let content = ChannelContentPin::from_activation(
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
            content,
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
        owner.rooms = SeamRooms::new(None, Some(&room));
        owner.movement_cells = (&cells).into();
        let catalogue = Arc::new(
            load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        owner.quest_catalogue = Some(Arc::clone(&catalogue));
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner.admit_quest_session(&admitted).await;
        let [oldrak] =
            super::classic_quests::install(&owner, &admitted, ["oteryn:npc.oldrak"]).await?;
        let mut placement = world.base().digest().to_vec();
        let key = oteryn_world_bundle::bundle::placement_key(GRAVE.floor, GRAVE.x, GRAVE.y, 1)
            .ok_or("gravestone placement")?;
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
        .map_err(debug)?;

        let result = wire(&owner, &mut admitted, 1, 2, payload.clone()).await?;
        assert_ne!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(start)
        );

        // Seed the existing defeat transition and report it through the real NPC dispatcher.
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use crate::foundation::{CommandId, CommandRef};
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key:
                        "oteryn:quest-transition/quest/u8_2/the_demon_oak/done/action_1".into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(100).map_err(debug)?,
                    )),
                },
                Arc::clone(&catalogue),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "{outcome:?}"
        );
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        super::classic_quests::talk(&mut owner, &admitted, oldrak, 101, "hi").await?;
        super::classic_quests::talk(&mut owner, &admitted, oldrak, 102, "demon oak").await?;
        assert_eq!(
            super::classic_quests::progress(&owner, &admitted, &authority, DONE).await?,
            2
        );
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        admitted.item_fence = Some(
            owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(debug)?
                .ok_or("fence after reporting")?
                .item_fence(),
        );
        let result = wire(&owner, &mut admitted, 2, 2, payload).await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(REWARD_ROOM)
        );
        Ok(())
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED reported_victory_opens_the_unmarked_gravestone {:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
