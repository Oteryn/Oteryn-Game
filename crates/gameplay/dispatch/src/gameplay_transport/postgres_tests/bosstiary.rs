//! Boss kill occurrence replay and current-session fencing through the durable owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::bosstiary::{BosstiaryBoss, progress};
use crate::durability::bestiary_progress::BestiaryKillOccurrence;

#[test]
fn bosstiary_two_kills_replay_and_stale_session_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("bosstiary", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "bosstiary", false, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let fence = owner
                .current_quest_fence(deps.admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("session")?;
            let boss = BosstiaryBoss {
                key: "oteryn:creature.ferumbras".into(),
                category: "nemesis".into(),
            };
            let (first, second) = {
                let mut runtime = deps.runtime.lock().await;
                (death(&mut runtime, fence)?, death(&mut runtime, fence)?)
            };
            assert_ne!(first.occurrence, second.occurrence);
            let rewards_before = rewards(&h).await?;
            for (facts, expected) in [(first, 1), (second, 2), (first, 2)] {
                settle(&owner, facts, &boss).await?;
                assert_eq!(count(&h).await?, expected);
            }
            assert_eq!(
                rewards(&h).await?,
                rewards_before,
                "unadmitted rewards stay unwritten"
            );
            // Semantic reuse of a retained occurrence for a different boss is refused.
            let other = BosstiaryBoss {
                key: "oteryn:creature.morshabaal".into(),
                category: "nemesis".into(),
            };
            assert!(
                h.root
                    .credit_bosstiary_kill(
                        &authority,
                        &h.node,
                        fence,
                        BestiaryKillOccurrence::from_bytes(first.occurrence).map_err(debug)?,
                        &other
                    )
                    .await
                    .is_err()
            );
            assert_eq!(count(&h).await?, 2);
            let points: i64 = sqlx::query_scalar("SELECT boss_points FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2")
            .bind(id(41).as_slice()).bind(&boss.key).fetch_one(&h.pool).await?;
            assert_eq!(points, 10);
            assert_eq!(progress("nemesis", 2).ok_or("category")?.stage, 1);
            let mut stale = fence;
            stale.connection_generation =
                crate::foundation::ConnectionGeneration::new(999).map_err(debug)?;
            assert!(
                h.root
                    .credit_bosstiary_kill(
                        &authority,
                        &h.node,
                        stale,
                        BestiaryKillOccurrence::from_bytes(id(103)).map_err(debug)?,
                        &boss
                    )
                    .await
                    .is_err()
            );
            assert_eq!(count(&h).await?, 2);
            let payload = h
                .root
                .read_boss_cooldown_list(&authority, &h.node, fence.item_fence(), &[])
                .await
                .map_err(debug)?
                .ok_or("boss query rejected")?;
            assert!(
                payload
                    .windows(boss.key.len())
                    .any(|bytes| bytes == boss.key.as_bytes())
            );
            assert!(payload.ends_with(&[0x28, 10]), "total boss points are ten");
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_bosstiary_kill_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(receipts, 2);
            assert!(sqlx::query("UPDATE game_character_bosstiary_kill_receipts SET category='bane' WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).execute(&h.pool).await.is_err());
            let mut uncredited = {
                let mut runtime = deps.runtime.lock().await;
                death(&mut runtime, fence)?
            };
            uncredited.principal_last_damage_at_ms = None;
            settle(&owner, uncredited, &boss).await?;
            assert_eq!(count(&h).await?, 2);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT kill_count FROM game_character_bosstiary_progress WHERE character_id=encode($1,'hex')::uuid AND boss_key='oteryn:creature.ferumbras'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) fn death(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
) -> TestResult<crate::combat::ProjectedCreatureDeathFacts> {
    let actor = runtime
        .admit_monster_lab_creature(
            crate::foundation::MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            "oteryn:creature.ferumbras",
            20,
        )
        .map_err(debug)?;
    runtime
        .commit_monster_lab_damage(actor, b"fixture:bosstiary.lethal", 20)
        .map_err(debug)?;
    let mut owner = runtime.borrow_combat_death();
    crate::combat::project_fixed_one_creature_death(&mut owner, actor).map_err(debug)?;
    let principal = crate::combat::CapturedRewardPrincipal {
        character: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
            .map_err(debug)?,
        lease_generation: fence.character_lease_generation,
        session: fence.game_session_id,
        actor,
    };
    let facts = crate::combat::capture_projected_death_facts(
        &mut owner,
        actor,
        principal,
        Some(1000),
        1000,
    )
    .map_err(debug)?;
    runtime.remove_test_actor(actor).map_err(debug)?;
    Ok(facts)
}

pub(super) async fn settle(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    facts: crate::combat::ProjectedCreatureDeathFacts,
    boss: &BosstiaryBoss,
) -> TestResult {
    use crate::gameplay_transport::kill_reward::{
        AppendOutcome, DrainOutcome, PendingKillSettlement,
    };
    let queue = owner.states.attack.lock().await.kills().clone();
    assert_eq!(
        queue.append(PendingKillSettlement::for_bosstiary(
            facts,
            boss.clone(),
            crate::combat::DeathGroundContext {
                boss_spawn: None,
                map_revision: "fixture:map".into(),
                content_revision: "fixture:content".into(),
                ruleset_revision: "fixture:ruleset".into(),
                sim_revision: "fixture:sim".into(),
                native_room_placement_context: b"fixture:room".to_vec(),
            },
            facts.principal.actor,
            None,
        )),
        AppendOutcome::Queued
    );
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while owner.drain_kill_rewards(facts.principal.session).await != DrainOutcome::Drained {
        if tokio::time::Instant::now() >= deadline {
            return Err("boss settlement did not drain".into());
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

async fn rewards(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_xp_receipts), (SELECT count(*) FROM game_item_mint_receipts)")
        .fetch_one(&h.pool).await?)
}
