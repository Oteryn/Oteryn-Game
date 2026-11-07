//! Original shared World switch deadlines drive the existing Channel decay owner.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
#[test]
fn l6_switch_timer_expired_12121_restores_source_lever() -> TestResult {
    run("l6-tomb-switch-timer-expired-12121")
}
#[test]
fn l6_switch_timer_expired_12122_restores_source_lever() -> TestResult {
    run("l6-tomb-switch-timer-expired-12122")
}
#[test]
fn l6_switch_timer_expired_12123_restores_source_lever() -> TestResult {
    run("l6-tomb-switch-timer-expired-12123")
}
#[test]
fn l6_switch_timer_future_does_not_restore_early() -> TestResult {
    run("l6-tomb-switch-timer-fresh-12121")
}
#[test]
fn l6_switch_timer_repeated_cycles_keep_source_policy() -> TestResult {
    run("l6-tomb-switch-timer-repeat-12121")
}
#[test]
fn l6_switch_timer_future_occurrence_is_rejected_by_pg() -> TestResult {
    run("l6-tomb-switch-timer-forged-12121")
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
        .map_err(|_| "switch timer fixture panicked")?;
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
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_switch_timer version={version}");
    let fresh = name.contains("-fresh-");
    let forged = name.contains("-forged-");
    let repeat = name.contains("-repeat-");
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("timer switch")?;
    let map = owner.bundle_map().ok_or("timer room")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let position = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let id = map
        .overlay
        .base()
        .tile(trigger.x, trigger.y, trigger.floor)
        .ok_or("timer tile")?
        .ids()[1];
    for cycle in 0..if repeat { 2 } else { 1 } {
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
        )
        .fetch_one(&h.pool)
        .await?;
        let occurred = if fresh {
            None
        } else if forged {
            Some(now + 60_000)
        } else if repeat && cycle == 0 {
            Some(now - 2_401_000)
        } else {
            Some(now - 1_200_500)
        };
        *owner
            .states
            .quest_callbacks
            .shared_switch_occurred_test
            .lock()
            .map_err(|_| "test occurrence")? = occurred;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("timer room")?;
        let expected_revision = map.facts.object_revision(key);
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision,
        })
        .map_err(debug)?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, 1 + cycle, 2, payload).await?;
        let disposition =
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?;
        if forged {
            assert_eq!(disposition, UseDisposition::Rejected);
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(count, 0);
            assert_eq!(map.facts.object_revision(key), 0);
            return Ok(());
        }
        assert_eq!(disposition, UseDisposition::Committed);
        let deadlines:Vec<(i64,bool)>=sqlx::query_as("SELECT expires_at_unix_ms-occurred_at_unix_ms,expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000) FROM game_shared_switch_leases ORDER BY occurred_at_unix_ms").fetch_all(&h.pool).await?;
        assert_eq!(
            deadlines,
            vec![(1_200_000, !fresh); usize::try_from(cycle + 1)?],
            "original absolute20min policy and actual PG due predicate"
        );
        map.facts.expire_movement_items(owner.owner_now().get());
        let facts = map
            .facts
            .base_entry(position, 1, id)
            .ok_or("timer current facts")?;
        if fresh {
            assert_eq!(facts.appearance_id, 2773);
            assert_eq!(map.facts.object_revision(key), 1);
            let due = map
                .facts
                .next_movement_decay_time()
                .ok_or("original timer is armed")?;
            let remaining = due.saturating_sub(owner.owner_now().get());
            assert!((1_100_000_000..=1_200_100_000).contains(&remaining));
        } else {
            assert_eq!(
                facts.appearance_id, 2772,
                "an actually due PG lease must restore its exact source lever"
            );
            assert_eq!(map.facts.object_revision(key), 2 * (cycle + 1));
            assert!(map.facts.next_movement_decay_time().is_none());
            assert_eq!(
                map.facts.expire_movement_items(owner.owner_now().get()),
                0,
                "the same source timer cannot restore twice"
            );
        }
    }
    Ok(())
}
