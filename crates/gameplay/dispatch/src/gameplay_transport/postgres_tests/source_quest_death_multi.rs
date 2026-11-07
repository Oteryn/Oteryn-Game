//! Two real admitted characters exercise the source damaging-player selection policy.
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult,
    fixture::{Harness, debug, id},
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
const CREATURE: &str = "oteryn:creature.renegade_orc";
const TRACK: &str = "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/quest_door";

#[test]
fn source_quest_death_multi_both_damagers() -> TestResult {
    scenario("both")
}
#[test]
fn source_quest_death_multi_lesser_active_damager() -> TestResult {
    scenario("lesser")
}
#[test]
fn source_quest_death_multi_non_damaging_observer() -> TestResult {
    scenario("observer")
}
fn scenario(case: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(case, move || {
        super::quest_world_callbacks::scenario(
            &format!("source-quest-death-multi-{case}"),
            Position {
                x: 33000,
                y: 32000,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "SOURCE_QUEST_DEATH_MULTI POSTGRES_CASE_{} case={case} duration={:?}",
        if result.is_ok() {
            "COMPLETED"
        } else {
            "FAILED"
        },
        started.elapsed()
    );
    result
}
pub(super) async fn exercise(
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    assert!(version.starts_with("PostgreSQL"));
    eprintln!("SOURCE_QUEST_DEATH_MULTI actual_postgresql=true {version}");
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let guest_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let guest_actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(guest_session)
            .map_err(debug)?;
        let guest = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        runtime
            .initialize_pinned_test_position(
                guest,
                MovementLocalPosition {
                    y: origin.y + 1,
                    ..origin
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                guest,
                guest_session,
                CharacterLease::new(CharacterId::decode(&id(101)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        guest
    };
    deps.initialize_player(owner, guest_actor, guest_session)
        .await;
    let mut guest = *admitted;
    guest.game_session_id = guest_session;
    guest.runtime_actor = Some(guest_actor);
    owner.admit_progression_session(&mut guest).await;
    guest.item_fence = Some(
        owner
            .current_quest_fence(guest_session)
            .await
            .map_err(|()| "guest fence")?
            .ok_or("guest fence")?
            .item_fence(),
    );
    owner.admit_quest_session(&guest).await;
    if !name.ends_with("lesser") {
        super::source_quest_death::start_quest(owner, session, 8000).await?;
    }
    super::source_quest_death::start_quest(owner, guest_session, 8100).await?;
    owner.refresh_quest_session(session, false).await;
    owner.refresh_quest_session(guest_session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let origin = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let policy = runtime.companion_policy(CREATURE).map_err(debug)?;
        let target = runtime
            .admit_encounter_actor(
                CREATURE,
                policy.maximum_health,
                MovementLocalPosition {
                    x: origin.x + 1,
                    ..origin
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, CREATURE)
            .map_err(debug)?;
        if !name.ends_with("observer") {
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    guest_actor,
                    CommandRef::new(guest_session, CommandId::new(8101).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: CREATURE.as_bytes(),
                        occurrence: guest_session.as_bytes(),
                        binding: b"source-quest-lesser-damage",
                        damage: 10,
                    },
                )
                .map_err(debug)?;
        }
        let remaining = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(8001).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: CREATURE.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"source-quest-lethal-damage",
                    damage: remaining,
                },
            )
            .map_err(debug)?;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session,
                actor,
            },
            now,
            Some(observed),
        );
        target
    };
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        crate::gameplay_transport::kill_reward::DrainOutcome::Drained
    );
    for (character, expected) in [
        (
            id(41),
            if name.ends_with("lesser") {
                None
            } else {
                Some(1)
            },
        ),
        (
            id(101),
            Some(if name.ends_with("observer") { 0 } else { 1 }),
        ),
    ] {
        let value:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(character.as_slice()).bind(TRACK).fetch_optional(&h.pool).await?;
        assert_eq!(
            value, expected,
            "native damaging-player selection for {character:?}"
        );
    }
    assert!(!owner.runtime.lock().await.corpse_projection_pinned(target));
    Ok(())
}
