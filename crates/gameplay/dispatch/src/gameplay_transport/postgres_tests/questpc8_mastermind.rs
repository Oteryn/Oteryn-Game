//! Chosen two-cache potion journey, Premium gates and consolidated gold.
use super::fixture::{Harness, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn questpc8_mastermind_two_native_caches_premium_and_exact_201_gold() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-mastermind", || {
        quest_world_callbacks::scenario(
            "authored-mastermind",
            Position {
                x: 32216,
                y: 31170,
                floor: -11,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mastermind_two_native_caches_premium_and_exact_201_gold duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let ids = [7440, 7159, 7158, 3031];
    let mut before = Vec::new();
    for item in ids {
        before.push(quantity(h, item).await?);
    }
    let premium = owner.premium_coordinator.take();
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(
        progress(h).await?,
        -1,
        "absent native Premium authority cannot start the journey"
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    step(owner, admitted, 2, StepDirection::West).await?;
    step(owner, admitted, 3, StepDirection::East).await?;
    assert_eq!(progress(h).await?, 1);
    step(owner, admitted, 4, StepDirection::North).await?;
    let first = TilePos {
        x: 32216,
        y: 31168,
        floor: -11,
    };
    let target = object_target(owner, first, 1)?;
    assert_eq!(
        use_target(owner, admitted, 5, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 2);
    assert_eq!(quantity(h, 7440).await? - before[0], 1);
    assert_eq!(quantity(h, 7159).await? - before[1], 1);
    assert_eq!(quantity(h, 7158).await?, before[2]);
    assert_eq!(quantity(h, 3031).await?, before[3]);
    let target = object_target(owner, first, 1)?;
    assert_eq!(
        use_target(owner, admitted, 6, &target).await?,
        UseDisposition::Rejected
    );
    for command in 7..10 {
        step(owner, admitted, command, StepDirection::South).await?;
    }
    step(owner, admitted, 10, StepDirection::West).await?;
    assert_eq!(progress(h).await?, 3);
    let second = TilePos {
        x: 32216,
        y: 31172,
        floor: -11,
    };
    let target = object_target(owner, second, 1)?;
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        use_target(owner, admitted, 11, &target).await?,
        UseDisposition::Rejected
    );
    assert_eq!(progress(h).await?, 3);
    assert_eq!(quantity(h, 7440).await? - before[0], 1);
    assert_eq!(quantity(h, 7159).await? - before[1], 1);
    assert_eq!(quantity(h, 7158).await?, before[2]);
    assert_eq!(quantity(h, 3031).await?, before[3]);
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    assert_eq!(
        use_target(owner, admitted, 12, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 4);
    for ((item, amount), initial) in [(7440, 2), (7159, 1), (7158, 2), (3031, 201)]
        .into_iter()
        .zip(&before)
    {
        assert_eq!(quantity(h, item).await? - initial, amount);
    }
    assert_eq!(
        use_target(owner, admitted, 12, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(owner, second, 1)?;
    assert_eq!(
        use_target(owner, admitted, 13, &target).await?,
        UseDisposition::Rejected
    );
    for ((item, amount), initial) in [(7440, 2), (7159, 1), (7158, 2), (3031, 201)]
        .into_iter()
        .zip(&before)
    {
        assert_eq!(quantity(h, item).await? - initial, amount);
    }
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.mastermind_potion_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    {
        let mut sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest cache")?;
        sessions.remove(&admitted.game_session_id);
    }
    owner.admit_quest_session(admitted).await;
    let target = object_target(owner, second, 1)?;
    assert_eq!(
        use_target(owner, admitted, 14, &target).await?,
        UseDisposition::Rejected
    );
    for ((item, amount), initial) in [(7440, 2), (7159, 1), (7158, 2), (3031, 201)]
        .into_iter()
        .zip(&before)
    {
        assert_eq!(quantity(h, item).await? - initial, amount);
    }
    Ok(())
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/mastermind/stage'),-1)").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
