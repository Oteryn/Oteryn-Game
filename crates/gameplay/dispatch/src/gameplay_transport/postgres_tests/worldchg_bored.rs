//! Server-save finalization captures Wyda's daily gate before publishing the epoch.
use super::*;
use crate::foundation::{ChannelId, WorldId};
use oteryn_gameplay_world_change::daily::roll_calendar;

#[test]
fn worldchg_bored_is_persisted_by_server_save_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("worldchg-bored", move || {
        run(async move |_| exercise(&admin).await)
    })
}

async fn exercise(admin: &str) -> TestResult {
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let expected = roll_calendar(world.as_bytes(), 1, 1, 1).bored;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let h = Harness::create_with_admission_seed(admin, "bored_reset", true, false).await?;
    let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        world,
        Some(include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    );
    let room = pin.qualified_room();
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let mut deps = Dependencies::new()?;
    deps.room = room.clone();
    planned_world_reset::reset_fixture::set_runtime(&mut deps, world, channel, &pin, 1)?;
    planned_world_reset::reset_fixture::publish_ready(&h, 1).await?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = controller.active();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(room), Some(room));
    assert!(owner.ensure_source_map_initialized().await);
    let server = room.compiled().server_digest();
    let client = room.compiled().client_digest();
    let frame = room.frame_binding().digest();
    let map = room.movement_cells().scope().map_revision.as_str();
    let world_bytes = world.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_bytes).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: world,
                    channel_id: channel,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: server,
                    client_artifact_digest: client,
                    frame_binding_digest: frame,
                },
            )
            .await?
    );
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world_bytes).bind(server.as_slice()).bind(map).bind(client.as_slice()).bind(frame.as_slice())
        .fetch_one(&h.pool).await?);
    assert!(!h.root.world_change_active(world, "bored").await?);

    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,'[]'::jsonb,1)")
        .bind(world_bytes).bind(server.as_slice()).bind(map).bind(client.as_slice()).bind(frame.as_slice())
        .fetch_one(&h.pool).await?);
    let reset = h.root.retiring_world_reset(world).await?.ok_or("reset")?;
    assert!(!h.root.world_change_active(world, "bored").await?);
    assert_eq!(
        h.root.finish_world_reset(&authority, &h.node, &reset, room, room).await.map_err(debug)?,
        Some(1)
    );
    let stored: Option<bool> = sqlx::query_scalar("SELECT (decision->>'bored')::boolean FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world_bytes).fetch_one(&h.pool).await?;
    assert_eq!(stored, Some(expected));
    assert_eq!(h.root.world_change_active(world, "bored").await?, expected);
    assert!(h.root.world_changes(world, 0).await?.is_none());
    Ok(())
}
