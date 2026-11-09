//! A timer owns its spawned actors, even after movement; other room occupants survive.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::MovementLocalPosition;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const CASE: &str = "qengine-rewards-questpc4-native-tower-owned-cleanup";

#[test]
fn engenc_timed_cleanup_postgres_preserves_foreign_and_removes_escaped_spawn() -> TestResult {
    super::qengine_rewards::scenario(CASE)
}

fn advance(owner: &mut ComposedFreshAdmission<'_, '_, '_>, now: u64) -> TestResult {
    assert!(
        now >= owner.owner_now().get(),
        "semantic clock never rewinds"
    );
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_micros(now))
        .ok_or("semantic clock")?;
    Ok(())
}

async fn writes(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
         (SELECT count(*) FROM game_item_mint_receipts)",
    )
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("native capabilities")?;
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/paradox-ghoul-switch"))
        .ok_or("timed source fixture")?;
    let mut fixture = serde_json::to_value(callback)?;
    fixture["effects"]["encounter"]["timing"]["cleanup_spawned_only"] = true.into();
    let callback: Callback = serde_json::from_value(fixture)?;
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback])
            .is_ok()
    );
    let lever = Position {
        x: 32481,
        y: 31904,
        floor: -5,
    };
    let key = oteryn_world_bundle::bundle::placement_key(lever.floor, lever.x, lever.y, 1)
        .ok_or("lever key")?;
    let revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    let before = writes(h).await?;
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, lever, revision).await?,
        UseDisposition::Committed
    );
    let spawn_at = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("spawn deadline")?;
    advance(owner, spawn_at + 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    let (spawned, foreign) = {
        let mut runtime = owner.runtime.lock().await;
        let actors = runtime
            .current_live_creature_candidates(4096)
            .map_err(debug)?;
        assert_eq!(actors.len(), 1, "the encounter created exactly one actor");
        let spawned = actors[0];
        let pin = runtime.content_pin().clone();
        let foreign = runtime
            .realize_native_qualification_spawn_for_source(
                u16::MAX - 9,
                pin,
                "oteryn:creature.rat",
                MovementLocalPosition {
                    x: 32478,
                    y: 31900,
                    floor: 5,
                },
            )
            .map_err(debug)?
            .actor;
        let expected = runtime.read_actor_position(spawned).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                spawned,
                expected,
                MovementLocalPosition {
                    x: i32::from(lever.x),
                    y: i32::from(lever.y),
                    floor: 5,
                },
            )
            .map_err(debug)?;
        (spawned, foreign)
    };
    let reset_at = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("reset deadline")?;
    advance(owner, reset_at + 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            !runtime.contains_live_creature(spawned),
            "expired source owns its escaped spawn"
        );
        assert!(
            runtime.contains_live_creature(foreign),
            "a foreign room occupant is not owned"
        );
        assert!(
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("player")?)
                .is_ok()
        );
    }
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        revision + 2
    );
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_none()
    );
    assert_eq!(
        writes(h).await?,
        before,
        "administrative cleanup grants no value or progress"
    );
    owner.drain_quest_encounter_deadlines().await;
    assert!(owner.runtime.lock().await.contains_live_creature(foreign));
    assert_eq!(writes(h).await?, before, "timer replay is value-free");

    // A later source-object mutation cancels its own occurrence without touching guests.
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, lever, revision + 2).await?,
        UseDisposition::Committed
    );
    {
        // Keep the independent visitor inside the room and off the next spawn tile.
        let mut runtime = owner.runtime.lock().await;
        let expected = runtime.read_actor_position(foreign).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                foreign,
                expected,
                MovementLocalPosition {
                    x: 32478,
                    y: 31900,
                    floor: 5,
                },
            )
            .map_err(debug)?;
    }
    let spawn_at = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("second spawn")?;
    advance(owner, spawn_at + 100_000)?;
    owner.drain_quest_encounter_deadlines().await;
    let canceled = {
        let runtime = owner.runtime.lock().await;
        let actors = runtime
            .current_live_creature_candidates(4096)
            .map_err(debug)?;
        assert_eq!(actors.len(), 2);
        let canceled = actors
            .into_iter()
            .find(|actor| *actor != foreign)
            .ok_or("second spawn actor")?;
        let facts = &owner.bundle_map().ok_or("map")?.facts;
        let restore = facts
            .quest_callback_replacement(key, 9125)
            .ok_or("source restore")?;
        assert!(facts.apply_quest_callback(
            key,
            facts.object_revision(key),
            restore,
            &[],
            &[],
            false,
            None
        ));
        canceled
    };
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(!runtime.contains_live_creature(canceled));
        assert!(runtime.contains_live_creature(foreign));
    }
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_none()
    );
    assert_eq!(writes(h).await?, before, "cancellation is value-free");
    Ok(())
}
