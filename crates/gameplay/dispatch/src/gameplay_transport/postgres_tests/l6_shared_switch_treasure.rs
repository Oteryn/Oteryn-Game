//! Personal expiry consequences follow the original live actor and real shared leases.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
#[test]
fn l6_switch_treasure_thalas_expiry_grants_one() -> TestResult {
    run("l6-tomb-switch-treasure-expired-12121")
}
#[test]
fn l6_switch_treasure_diprath_expiry_grants_two() -> TestResult {
    run("l6-tomb-switch-treasure-expired-12122")
}
#[test]
fn l6_switch_treasure_ashmunrah_expiry_grants_no_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-expired-12123")
}
#[test]
fn l6_switch_treasure_repeat_cycles_do_not_repeat_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-repeat-12121")
}
#[test]
fn l6_switch_treasure_future_lease_grants_no_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-fresh-12121")
}
#[test]
fn l6_switch_treasure_absent_original_actor_grants_no_personal_progress() -> TestResult {
    run("l6-tomb-switch-treasure-offline-12122")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "personal timer fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
    name: &str,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_switch_treasure version={version}");
    let before = snapshot(h).await?;
    super::l6_shared_switch_timers::exercise(owner, admitted, h, trigger, name).await?;
    if let Ok(Some(fence)) = owner.current_quest_fence(admitted.game_session_id).await {
        let mut different_actor = actor.placement_identity();
        different_actor[15] ^= 1;
        if owner.root.has_ready_demand() {
            owner.root.maintain_ready_once().await?;
        }
        assert!(
            owner
                .root
                .read_due_quest_timers(owner.character, owner.holder, fence, different_actor)
                .await?
                .is_empty(),
            "source eligibility must filter the original actor before the bounded batch limit"
        );
    }
    if name.contains("-offline-") {
        owner
            .runtime
            .lock()
            .await
            .remove_test_actor(actor)
            .map_err(debug)?;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("timer clock")?;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    owner.tick_timed_items().await;
    let no_progress =
        name.contains("-fresh-") || name.contains("-offline-") || name.ends_with("12123");
    if no_progress {
        assert_eq!(
            snapshot(h).await?,
            before,
            "future, absent original actor and Ashmunrah cannot grant personal progress"
        );
    } else {
        let (track, goal) = if name.ends_with("12121") {
            ("oteryn:quest-track/ancient-tombs/thalas-treasure", 1_i64)
        } else {
            ("oteryn:quest-track/ancient-tombs/diprath-treasure", 2_i64)
        };
        let value:i64=sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)").bind(track).fetch_one(&h.pool).await?;
        assert_eq!(
            value, goal,
            "actual expired lease and original live actor must advance exact source treasure progress"
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='server_timer'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            receipts, 1,
            "even repeated physical cycles award source personal progress once"
        );
    }
    let settled = snapshot(h).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("timer clock")?;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    owner.tick_timed_items().await;
    assert_eq!(
        snapshot(h).await?,
        settled,
        "replaying a due owner pass never repeats personal progress"
    );
    Ok(())
}
async fn snapshot(h: &Harness) -> TestResult<(Vec<serde_json::Value>, Vec<serde_json::Value>)> {
    Ok((sqlx::query_scalar("SELECT to_jsonb(t) FROM game_character_quest_tracks t ORDER BY character_id,track_key").fetch_all(&h.pool).await?,sqlx::query_scalar("SELECT to_jsonb(r) FROM game_character_quest_receipts r ORDER BY character_id,committed_character_revision").fetch_all(&h.pool).await?))
}
