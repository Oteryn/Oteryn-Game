//! Exercise the ordinary party death caller using two disjoint authored lairs.
use super::{ComposedFreshAdmission, TestResult, fixture::debug};
use crate::bestiary_postgres_harness::Harness;
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use std::sync::Arc;

const COUNT: &str = "oteryn:quest-progress/authored-kills/count";

#[test]
fn shared_party_credits_each_character_from_the_second_death_area_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "PostgreSQL admin URL is empty");
    super::party_xp::run(super::party_xp::Case::DhuntDeathAreas)
}

pub(super) fn install(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    let mut bindings: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quests/src/quest/authored_bindings/fixtures/kill.json"
    )))?;
    let quest = &mut bindings["quests"][0];
    // Start both party members in the active phase. No kill credit is seeded.
    quest["tracks"][0]["initial"] = 4.into();
    quest["stages"][1]["trigger"]["counter"]["death_areas"] = serde_json::json!([
        {"minimum": [1, 11, 6], "maximum": [1, 11, 6]},
        {"minimum": [1, 11, 7], "maximum": [1, 11, 7]}
    ]);
    let catalogue = QuestStateCatalogue::empty("dhunt-death-areas").ok_or("catalogue")?;
    owner.quest_catalogue = Some(Arc::new(
        Bindings::parse(&serde_json::to_vec(&bindings)?)?.attach(&catalogue)?,
    ));
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    const RAT: &str = "oteryn:creature.rat";
    let &(actor, session) = players.first().ok_or("party leader")?;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let queue = owner.states.attack.lock().await.kills().clone();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native gameplay")?;
    for (number, floor, credit) in [(800, 8, 0), (801, 7, 1), (802, 6, 2)] {
        let observed = owner.owner_now();
        let now = observed.get() / 1000;
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let health = runtime.companion_policy(RAT).map_err(debug)?.maximum_health;
            let target = runtime
                .admit_encounter_actor(RAT, health, MovementLocalPosition { x: 1, y: 11, floor })
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, RAT)
                .map_err(debug)?;
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(debug)?;
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(session, CommandId::new(number).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: RAT.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"dhunt-area-test-death",
                        damage: health,
                    },
                )
                .map_err(debug)?;
            for _ in 0..2 {
                record_creature_kill(
                    &mut runtime,
                    KillSink {
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
            }
            target
        };
        assert_eq!(
            owner.drain_kill_rewards(session).await,
            DrainOutcome::Drained
        );
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(target)
            .map_err(debug)?;
        let counts = counts(h).await?;
        if credit == 0 {
            assert!(counts.is_empty(), "out-of-area deaths earn no credit");
        } else {
            assert_eq!(
                counts,
                [credit, credit],
                "each eligible death credits both members once"
            );
        }
    }
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<Vec<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key = $1
         ORDER BY character_id LIMIT 3",
    )
    .bind(COUNT)
    .fetch_all(&h.pool)
    .await?)
}
