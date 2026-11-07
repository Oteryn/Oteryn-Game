//! Isolate source stock across already-activated reset epochs, retaining the Channel cache.
//! The activation pointer is a completed-reset fixture; this does not replace reset lifecycle tests.
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult,
    fixture::{Harness, debug, id},
};
use crate::foundation::{CharacterId, CharacterLease, GameSessionId};
use crate::gameplay_transport::{
    connection::UseCommand,
    container_view::ContainerObservation,
    item_move::{LoopMove, LoopMoveRefusal, LoopMoveReplay, LoopMoveResult, LoopTarget},
    item_view::{ItemKey, ViewItem},
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn draconia_stock_epochs_reused_view_replenishes() -> TestResult {
    super::draconia_container_stock::scenario("epoch-view")
}
#[test]
fn draconia_stock_epochs_replay_is_scoped_to_its_epoch() -> TestResult {
    super::draconia_container_stock::scenario("epoch-replay")
}
#[test]
fn draconia_stock_epochs_second_player_sees_shared_replenishment() -> TestResult {
    super::draconia_container_stock::scenario("epoch-observer")
}

async fn entries(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    root: ItemKey,
) -> TestResult<Vec<ViewItem>> {
    let view = crate::gameplay_transport::loop_move::observe_container(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.item_fence,
        root,
    )
    .await;
    let Some(ContainerObservation::Container { entries, .. }) = view else {
        return Err(format!("source stock view missing: {view:?}").into());
    };
    Ok(entries)
}
async fn guest(
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<AdmittedSession> {
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                super::quest_world_callbacks::local(Position {
                    x: 32795,
                    y: 31572,
                    floor: -7,
                }),
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(CharacterId::decode(&id(101)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, actor, session).await;
    let mut guest = *admitted;
    guest.game_session_id = session;
    guest.runtime_actor = Some(actor);
    owner.admit_progression_session(&mut guest).await;
    guest.item_fence = Some(
        owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "guest fence")?
            .ok_or("guest fence")?
            .item_fence(),
    );
    owner.admit_quest_session(&guest).await;
    Ok(guest)
}
async fn activate_fixture(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    let (server, client, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().client_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let map_revision =
        crate::map::overlay::map_revision(owner.bundle_map().ok_or("map")?.overlay.base());
    // Model the durable result of completed reset activation. Preserve the same
    // runtime/cache to prove the source reader keys its projection by the epoch itself.
    for epoch in [0_i64, 1] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$3,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'))")
            .bind(owner.world_id.as_bytes().as_slice()).bind(epoch.to_string()).bind(server.as_slice()).bind(&map_revision).bind(client.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    owner.reset_epoch = owner
        .root
        .active_world_reset_epoch(owner.world_id)
        .await?
        .ok_or("active reset epoch")?;
    assert_eq!(owner.reset_epoch, 1);
    eprintln!("POSTGRES_STOCK_RESET_EPOCH active=1 reused_channel_cache=true");
    Ok(())
}
pub(super) async fn exercise(
    deps: &Dependencies,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let base = owner.bundle_map().ok_or("map")?.overlay.base();
    let root = ItemKey::MapBase {
        bundle_digest: base.digest(),
        placement_key: crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: 32794,
                y: 31572,
                floor: -7,
            },
            1,
        )
        .ok_or("placement")?,
        reset_epoch: owner.reset_epoch,
    };
    let original = entries(owner, admitted, root).await?;
    assert_eq!(original.len(), 1);
    let observer = if name.ends_with("observer") {
        Some(guest(deps, owner, admitted).await?)
    } else {
        None
    };
    if let Some(guest) = &observer {
        assert_eq!(entries(owner, guest, root).await?, original);
    }
    super::draconia_container_stock::exercise(owner, admitted, h, deps.actor, "shared").await?;
    if let Some(guest) = &observer {
        assert!(entries(owner, guest, root).await?.is_empty());
    }
    activate_fixture(owner, h).await?;
    let ItemKey::MapBase {
        bundle_digest,
        placement_key,
        ..
    } = root
    else {
        return Err("root identity".into());
    };
    let next = ItemKey::MapBase {
        bundle_digest,
        placement_key,
        reset_epoch: owner.reset_epoch,
    };
    let refreshed = entries(owner, admitted, next).await?;
    assert_eq!(
        refreshed.len(),
        1,
        "completed reset must replenish shared stock despite a live old cache"
    );
    if let Some(guest) = &observer {
        assert_eq!(entries(owner, guest, next).await?, refreshed);
    }
    assert!(matches!(
        crate::gameplay_transport::loop_move::observe_container(
            owner,
            deps.actor,
            admitted.item_fence,
            root
        )
        .await,
        Some(ContainerObservation::Gone)
    ));
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 4200,
        item_fence: admitted.item_fence,
    };
    let stale = LoopMove {
        item: original[0].key,
        count: None,
        target: LoopTarget::MainBackpack,
    };
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(owner, deps.actor, command, stale)
            .await,
        LoopMoveResult::Refused(LoopMoveRefusal::Stale)
    );
    let request = LoopMove {
        item: refreshed[0].key,
        ..stale
    };
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(owner, deps.actor, command, request)
            .await,
        LoopMoveResult::Committed
    );
    if name.ends_with("replay") {
        assert_eq!(
            crate::gameplay_transport::loop_move::committed_loop_move(owner, command).await,
            Some(LoopMoveReplay::Moved)
        );
    }
    if let Some(guest) = &observer {
        assert!(entries(owner, guest, next).await?.is_empty());
    }
    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_keys WHERE key_number=3001")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(keys, 2, "one source key per shared reset epoch");
    let epochs: i64 =
        sqlx::query_scalar("SELECT count(DISTINCT reset_epoch) FROM game_map_item_mint_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(epochs, 2);
    eprintln!(
        "POSTGRES_STOCK_REPLENISHED keys=2 epochs=2 second_player={}",
        observer.is_some()
    );
    Ok(())
}
