//! The pinned Spoiler (curid 91784, oldid 1097856) puts all five residents before castle entry.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire, world};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, NodeId};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{hero_of_bounac::QUEST, loader, world_callbacks::{self, Position}};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, decode_step_result, encode_step_intent,
};
use std::sync::Arc;

#[test]
fn castle_steps_require_all_five_residents() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                let started = std::time::Instant::now();
                for y in [32480, 32481] {
                    for trust in [4, 5] {
                        exercise(&admin, y, trust).await?;
                    }
                }
                eprintln!("QCX75 castle steps elapsed={:?}", started.elapsed());
                Ok(())
            }).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "QCX75 castle step panicked")?
        .map_err(Into::into)
}

async fn exercise(admin: &str, y: u16, trust: i64) -> TestResult {
    let h = Harness::create_appearance(admin.to_owned(), "qcx75_castle").await?;
    super::reset();
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
    let generation = controller.active().ok_or("active generation")?;
    let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    let catalogue = loaded.catalogue();
    let callback = world_callbacks::load_for(catalogue)?
        .into_iter()
        .find(|callback| callback.key == format!("oteryn:quest-callback/lion-castle-enter-{y}"))
        .ok_or("castle entry callback")?;
    let trigger = Position { x: 32400, y, floor: -4 };
    let start = Position { x: 32399, ..trigger };
    let world = world(&callback, trigger)?;
    let room = activated.qualified_room();
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let entry = crate::content::NativeEntryStart { x: start.x.into(), y: start.y.into(), floor: 4 };
    let room = room.bundle_view(cells.clone(), frame, world.map_revision_digest(), entry)
        .ok_or("castle test room")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id, 1, generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(), frame, world.map_revision_digest(),
        (entry.x, entry.y, entry.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id, deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?, 1, 1, 1, "runtime-scope-assignment:1", 2, pin,
    ).map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime.initialize_pinned_test_position(deps.actor, local(start)).map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
    owner.quest_catalogue = Some(Arc::new(catalogue.clone()));

    // Seed quest history while the upstream resident producers are unbound.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    for (track, value) in [("access_eastern_side", 1), ("bounac_trust", trust)] {
        let key = format!("oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/{track}");
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(key).bind(QUEST).bind(value)
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;

    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "quest fence")?.ok_or("quest fence")?;
    admitted.item_fence = Some(fence.item_fence());
    owner.runtime.lock().await.bind_attacker_lease(
        deps.actor, admitted.game_session_id,
        CharacterLease::new(CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation).map_err(debug)?,
    ).map_err(debug)?;
    owner.admit_quest_session(&admitted).await;
    let reply = wire(&owner, &mut admitted, 1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East)).await?;
    let disposition = decode_step_result(&reply).map_err(debug)?;
    let actual = owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position();
    if trust == 5 {
        assert_eq!(actual, local(Position { x: 32397, ..trigger }), "castle lane {y}: {disposition:?}");
    } else {
        assert_ne!(actual, local(Position { x: 32397, ..trigger }), "castle lane {y}: {disposition:?}");
    }
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
