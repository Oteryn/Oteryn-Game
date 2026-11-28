//! A committed landing must retain its POI credit across a busy holder and relog.
use super::cyclopedia_fixtures::support;
use super::*;
use crate::domain::cyclopedia::Exploration;
use crate::foundation::{MovementLocalPosition, MovementPositionSnapshot};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, LogoutReply};
use crate::gameplay_transport::guild_intent::GuildReply;
use oteryn_protocol_oteryn::{cyclopedia as wire, logout::LogoutResult};

#[test]
fn cyclopedia_exact_poi_landing_credits_once_after_busy_holder_and_relog() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cyclopedia POI scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_poi_landing", true, false)
            .await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, activation) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let world = support::world(&deps, generation)?;
            let cells = world
                .movement_cells(activation.qualified_room().movement_cells())
                .map_err(debug)?;
            support::seed(&h, world.base().digest()).await?;
            let start = world.start();
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                activation.frame_binding().digest(),
                world.map_revision_digest(),
                (start.x.into(), start.y.into(), i16::from(-start.floor)),
            );
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let session = deps.admitted.game_session_id;
            let current = store.current_session(session).await?;
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
            let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(deps.actor, session, current.current_character_lease())
                .map_err(debug)?;
            deps.runtime = Mutex::new(runtime);

            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
            owner.active_generation = Some(generation);
            deps.initialize(&owner).await;
            track(&owner, &deps)?;
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.controller.as_mut().ok_or("controller")?.transport =
                current.current_transport().ok_or("transport")?;
            let offers = crate::gameplay_transport::cyclopedia::offered(
                owner.offered_capabilities(),
                owner.cyclopedia_available(),
            );
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(&offers, &[wire::CAPABILITY_CYCLOPEDIA_MAP_V1])
                    .ok_or("Cyclopedia capability")?;
            let initial = query(&owner, admitted, admitted.continuity.next_command_id).await?;
            assert_eq!(initial.found, 0);
            let (_, subarea) = world
                .base()
                .subareas
                .resolve(start.x, start.y, start.floor)
                .ok_or("entry subarea")?;
            let character = admitted.item_fence.ok_or("fenced character")?.character_id;
            let exploration: String = sqlx::query_scalar(
                "SELECT (progress->'subareas'->$2::text->'exploration')::text
                 FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(character.as_bytes().as_slice())
            .bind(subarea.to_string())
            .fetch_one(&h.pool)
            .await?;
            let exploration: Exploration = serde_json::from_str(&exploration)?;
            let point = exploration.points.first().ok_or("server-generated POI")?;
            let target = MovementLocalPosition {
                x: point.x,
                y: point.y,
                floor: -point.floor,
            };
            let poi_tile = crate::map::overlay::TilePos {
                x: u16::try_from(point.x)?,
                y: u16::try_from(point.y)?,
                floor: i8::try_from(point.floor)?,
            };
            assert!(world.enterable(poi_tile));
            let adjacent = [poi_tile.x.checked_sub(1), poi_tile.x.checked_add(1)]
                .into_iter()
                .flatten()
                .find(|&x| world.enterable(crate::map::overlay::TilePos { x, ..poi_tile }))
                .map(|x| MovementLocalPosition {
                    x: x.into(),
                    ..target
                })
                .ok_or("walkable neighbour of the POI")?;
            // Position the fixture, then hand the real committed cardinal landing
            // to the same discovery hook that ordinary_paced_step invokes.
            let entered = {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut from = movement.read(deps.actor).map_err(debug)?;
                assert_eq!(from.position().floor, adjacent.floor);
                while from.position() != adjacent {
                    let at = from.position();
                    let next = if at.x != adjacent.x {
                        MovementLocalPosition {
                            x: at.x + (adjacent.x - at.x).signum(),
                            ..at
                        }
                    } else {
                        MovementLocalPosition {
                            y: at.y + (adjacent.y - at.y).signum(),
                            ..at
                        }
                    };
                    from = movement.commit_cardinal(from, next).map_err(debug)?;
                }
                movement.commit_cardinal(from, target).map_err(debug)?
            };

            eprintln!("POSTGRES_STARTED W12 exact POI landing");
            credit_after_holder_return(&owner, deps.actor, session, entered).await?;
            let first = query(&owner, admitted, admitted.continuity.next_command_id + 1).await?;
            assert_eq!(
                first.found, 1,
                "the committed exact landing must credit the POI"
            );
            assert!(first.revision > initial.revision);
            assert!(
                !first
                    .visible
                    .contains(&(poi_tile.x, poi_tile.y, -poi_tile.floor))
            );

            // Repeat the same landing receipt and then physically leave and re-enter.
            owner.cyclopedia_step(deps.actor, session, entered).await;
            let revisited = {
                let mut runtime = owner.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let left = movement.commit_cardinal(entered, adjacent).map_err(debug)?;
                movement.commit_cardinal(left, target).map_err(debug)?
            };
            owner.cyclopedia_step(deps.actor, session, revisited).await;
            let repeated = query(&owner, admitted, admitted.continuity.next_command_id + 2).await?;
            assert_eq!(repeated, first);

            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
            let successor = super::full_world_support::relogin(&h).await?;
            assert_ne!(successor, session);
            let current = store.current_session(successor).await?;
            let future_fence =
                crate::gameplay_transport::item_fence_of(current).ok_or("successor fence")?;
            assert!(
                future_fence.character_lease_generation
                    > admitted
                        .item_fence
                        .ok_or("old fence")?
                        .character_lease_generation
            );
            let gameplay = owner
                .current_quest_fence(successor)
                .await
                .map_err(|()| "successor gameplay read failed")?
                .ok_or("successor gameplay fence")?;
            let decided = owner
                .read_admission_position(gameplay)
                .await
                .map_err(|()| "successor position read failed")?;
            let actor = {
                let mut runtime = owner.runtime.lock().await;
                let reserved = runtime.reserve_fresh_session(successor).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime
                    .bind_attacker_lease(actor, successor, current.current_character_lease())
                    .map_err(debug)?;
                owner
                    .initialize_admitted_position(&mut runtime, actor, successor, &decided)
                    .await
                    .map_err(debug)?;
                assert_eq!(
                    runtime
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    target
                );
                actor
            };
            let mut fresh = AdmittedSession {
                game_session_id: successor,
                runtime_actor: Some(actor),
                item_fence: Some(future_fence),
                ..admitted
            };
            fresh
                .controller
                .as_mut()
                .ok_or("successor controller")?
                .transport = current.current_transport().ok_or("successor transport")?;
            let reloaded = query(&owner, fresh, fresh.continuity.next_command_id).await?;
            assert_eq!(reloaded.found, first.found);
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}

async fn credit_after_holder_return(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    entered: MovementPositionSnapshot,
) -> TestResult {
    use std::{
        future::{Future, poll_fn},
        task::Poll,
    };
    let pass = owner.root.try_issue_semantic_pass()?;
    let mut credit = std::pin::pin!(owner.cyclopedia_step(actor, session, entered));
    let first = poll_fn(|cx| Poll::Ready(credit.as_mut().poll(cx))).await;
    let runtime_free = owner.runtime.try_lock().is_ok();
    pass.run(|_, _| Box::pin(async { Ok(()) })).await?;
    assert!(
        first.is_pending(),
        "a busy holder must not discard the exact landing"
    );
    assert!(
        runtime_free,
        "waiting for the holder must release runtime ownership"
    );
    credit.await;
    Ok(())
}

async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> TestResult<wire::MapResult> {
    match owner
        .cyclopedia_wire(
            admitted,
            command_id,
            wire::COMMAND_TYPE_CYCLOPEDIA_QUERY,
            &[],
        )
        .await
    {
        GuildReply::Payload(bytes) => wire::decode_result(&bytes).map_err(debug),
        _ => Err("Cyclopedia query did not return a payload".into()),
    }
}
