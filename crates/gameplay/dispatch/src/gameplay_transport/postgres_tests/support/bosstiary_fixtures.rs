use super::fixture::debug;

use super::TestResult;

use crate::domain::bosstiary::BosstiaryBoss;

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
