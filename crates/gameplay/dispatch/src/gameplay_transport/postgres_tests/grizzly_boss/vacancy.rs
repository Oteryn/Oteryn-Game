//! Q10=B regressions exercise native movement and the current Channel's existing
//! room visit owner. The second Character has a real fresh admission publication.
use super::*;

#[derive(Debug, Clone, Copy)]
pub(super) enum Case {
    OtherPlayer,
    OtherBoss,
    OldDeadline,
    RetirementReplay,
}

#[test]
fn grizzly_vacated_room_allows_another_player_before_deadline() -> TestResult {
    run_case(Case::OtherPlayer)
}
#[test]
fn grizzly_early_exit_allows_a_different_boss_before_deadline() -> TestResult {
    run_case(Case::OtherBoss)
}
#[test]
fn grizzly_old_deadline_never_removes_a_new_visit_boss() -> TestResult {
    run_case(Case::OldDeadline)
}
#[test]
fn grizzly_vacancy_retirement_replay_has_no_second_effect() -> TestResult {
    run_case(Case::RetirementReplay)
}

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(1, false, Some(case)).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Grizzly vacancy PostgreSQL thread panicked")??;
    Ok(())
}

pub(super) async fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    deps: &Dependencies,
    boss: &quest::GrizzlyBoss,
    origin: MovementLocalPosition,
    case: Case,
) -> TestResult {
    let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
    let old = h
        .root
        .pending_grizzly_boss_entry(owner.character, character)
        .await
        .map_err(debug)?
        .ok_or("first active visit")?;
    assert!(!old.expired);
    let old_boss = monsters(owner).await?[0];
    if matches!(case, Case::OtherBoss) {
        let other = quest::boss(3).ok_or("Deathbine")?;
        let other_origin = MovementLocalPosition {
            x: i32::from(other.portal.x) - 1,
            ..local(other.portal)
        };
        teleport(owner, deps.actor, admitted.game_session_id, other_origin).await?;
        world::boss_points(h).await?;
        action(owner, 200, Action::SelectBoss(other.id)).await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        assert_eq!(
            h.root
                .pending_grizzly_boss_entry(owner.character, character)
                .await
                .map_err(debug)?
                .ok_or("visit retained until native census")?
                .cause,
            old.cause
        );
        east(owner, admitted, 3).await?;
        assert_position(owner, deps.actor, local(other.player_position)).await?;
        assert_eq!(value(owner, other.storage).await?, 2);
        let new = h
            .root
            .pending_grizzly_boss_entry(owner.character, character)
            .await
            .map_err(debug)?
            .ok_or("second boss visit")?;
        assert_eq!(new.boss_id, other.id);
        assert_ne!(new.cause, old.cause);
    } else {
        // Leave by the real native teleport primitive. No quest state or visit is
        // edited: the next player's accepted Step must discover the vacancy.
        teleport(
            owner,
            deps.actor,
            admitted.game_session_id,
            MovementLocalPosition {
                x: origin.x - 1,
                ..origin
            },
        )
        .await?;
        let (actor, mut second) = second_player(owner, deps, h, origin).await?;
        let session = second.game_session_id;
        action_for(owner, session, 100, Action::Join).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_character_quest_tracks SET value=1 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(101).as_slice()).bind(quest::storage_track(quest::BOSS_POINTS_STORAGE)).execute(&mut *tx).await?;
        tx.commit().await?;
        action_for(owner, session, 101, Action::SelectBoss(boss.id)).await?;
        owner.refresh_quest_session(session, false).await;
        east(owner, &mut second, 1).await?;
        assert_position(owner, actor, local(boss.player_position)).await?;
        let new = h
            .root
            .pending_grizzly_boss_entry(
                owner.character,
                crate::domain::CharacterId::from_bytes(id(101)).map_err(debug)?,
            )
            .await
            .map_err(debug)?
            .ok_or("new player's room visit")?;
        assert_eq!(new.state, GrizzlyBossEntryState::Active);
        assert_ne!(new.character_id, old.character_id);
        assert!(!new.expired);
        assert!(
            new.entered_at_unix_ms < old.deadline_unix_ms,
            "a vacated room is reusable immediately, without waiting ten minutes"
        );
    }
    assert!(!owner.runtime.lock().await.contains_live_creature(old_boss));
    let fresh = monsters(owner).await?;
    assert_eq!(
        fresh.len(),
        1,
        "the old room monster is cleared before the new visit is installed"
    );
    let states: (i64, i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE state='CLOSED'),count(*) FILTER(WHERE state='ACTIVE') FROM game_character_grizzly_boss_entries")
        .fetch_one(&h.pool).await?;
    assert_eq!(states, (1, 1));
    assert_eq!(visits(h).await?, 2);
    match case {
        Case::OldDeadline => {
            // The old timer's immutable key is retained, while the room now belongs
            // to another entry. Re-proving that key must precede physical cleanup.
            let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
            assert!(
                owner
                    .retire_grizzly_visit(&permit, &old, true)
                    .await
                    .map_err(debug)?
            );
            drop(permit);
            assert_eq!(monsters(owner).await?, fresh);
            assert!(owner.grizzly_boss_pass().await.map_err(debug)?);
            assert_eq!(monsters(owner).await?, fresh);
        }
        Case::RetirementReplay => {
            let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
            let binding = owner.runtime.lock().await.binding();
            let channel_id = binding.channel_id().ok_or("channel")?;
            for _ in 0..2 {
                assert!(
                    !h.root
                        .retire_vacated_grizzly_boss_entry(
                            owner.character,
                            owner.holder,
                            &permit,
                            binding.world_id(),
                            channel_id,
                            binding.scope_generation(),
                            &old
                        )
                        .await
                        .map_err(debug)?
                );
            }
            assert_eq!(monsters(owner).await?, fresh);
            assert_eq!(visits(h).await?, 2);
        }
        _ => {}
    }
    let unchanged: (i64, i64) = sqlx::query_as("SELECT entered_at,deadline_at FROM game_character_grizzly_boss_entries WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric")
        .bind(old.character_id.as_bytes().as_slice()).bind(old.cause.game_session_id().as_bytes().as_slice())
        .bind(old.cause.command_id().get().to_string()).fetch_one(&h.pool).await?;
    assert_eq!(unchanged, (old.entered_at_unix_ms, old.deadline_unix_ms));
    Ok(())
}

async fn teleport(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    destination: MovementLocalPosition,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells,
        actor,
        session,
        before,
        destination,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

pub(super) async fn second_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    h: &Harness,
    origin: MovementLocalPosition,
) -> TestResult<(ExactActorRef, AdmittedSession)> {
    h.seed_pvp_target().await?;
    super::super::appearance_change_postgres::observer_visibility_tests::admit(h).await?;
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "second fence")?
        .ok_or("second current fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, origin)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: deps.admitted.world_id,
        channel_id: deps.admitted.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])
                .map_err(debug)?,
            account_id: id(100),
        }),
        continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize_player(owner, actor, session).await;
    owner.admit_quest_session(&admitted).await;
    assert!(owner.track_restored(session, actor, &AdmissionPosition::FirstLogin, fence));
    Ok((actor, admitted))
}

async fn action_for(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    n: u64,
    action: Action,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_grizzly_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestCause::Command(CommandRef::new(session, CommandId::new(n).map_err(debug)?)),
            action,
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
            &NoGrizzlyPremium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "{outcome:?}"
    );
    Ok(())
}
