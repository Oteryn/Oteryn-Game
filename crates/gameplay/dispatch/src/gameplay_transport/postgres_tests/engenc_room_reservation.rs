//! Timed room reservations fence competing source objects before their map write.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const CASE: &str = "qengine-rewards-questpc4-native-tower-room-reservation-second";

#[test]
fn engenc_room_reservation_postgres_fences_other_sources_and_releases_after_reset() -> TestResult {
    super::qengine_rewards::scenario(CASE)
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
        .ok_or("capabilities")?;
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/paradox-ghoul-switch"))
        .ok_or("timer fixture")?;
    let mut value = serde_json::to_value(callback)?;
    value["effects"]["encounter"]["timing"]["exclusive_room"] = true.into();
    value["effects"]["encounter"]["timing"]["cleanup_spawned_only"] = true.into();
    let mut callback: Callback = serde_json::from_value(value)?;
    callback.lever = Some([2773, 2772]);
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.room_min.floor = -6;
    encounter.room_max.floor = -6;
    for monster in &mut encounter.monsters {
        monster.position.floor = -6;
    }
    let first = Position {
        x: 32476,
        y: 31900,
        floor: -6,
    };
    let second = Position { x: 32477, ..first };
    let callbacks = [first, second]
        .into_iter()
        .enumerate()
        .map(|(index, source)| -> TestResult<_> {
            let mut c = callback.clone();
            c.key = format!("engenc/room-reservation-{index}");
            c.selector = oteryn_gameplay_quests::quest::world_callbacks::Selector {
                position: Some(source),
                unique: None,
                action: None,
                appearance: Some(2773),
                native: Some(
                    oteryn_gameplay_quests::quest::world_callbacks::NativeSelector {
                        position: source,
                        ordinal: 1,
                        appearance: 2773,
                    },
                ),
            };
            let mut value = serde_json::to_value(c)?;
            if index == 1 {
                value["effects"]["encounter"]["timing"]
                    .as_object_mut()
                    .ok_or("timing object")?
                    .remove("exclusive_room");
            }
            Ok(serde_json::from_value(value)?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    assert!(owner.states.quest_callbacks.fixture.set(callbacks).is_ok());
    super::l6_paradox_native::walk(
        owner,
        admitted,
        Position {
            x: 32476,
            y: 31905,
            floor: -6,
        },
    )
    .await?;
    super::l6_paradox_native::walk(
        owner,
        admitted,
        Position {
            x: 32476,
            y: 31901,
            floor: -6,
        },
    )
    .await?;
    let key = oteryn_world_bundle::bundle::placement_key(second.floor, second.x, second.y, 1)
        .ok_or("second key")?;
    let revision = owner.bundle_map().ok_or("map")?.facts.object_revision(key);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, first, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, second, revision).await?,
        UseDisposition::Rejected,
        "a different source cannot enter an overlapping reserved room"
    );
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        revision,
        "refusal leaves the competing lever unchanged"
    );
    let reset_ms = callback
        .effects
        .encounter
        .as_ref()
        .and_then(|e| e.timing)
        .ok_or("timing")?
        .reset_after_ms;
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_micros(
            owner.owner_now().get() + u64::from(reset_ms) * 1000 + 100_000,
        ))
        .ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, second, revision).await?,
        UseDisposition::Committed,
        "reset releases the room reservation for another source"
    );
    let first_key = oteryn_world_bundle::bundle::placement_key(first.floor, first.x, first.y, 1)
        .ok_or("first key")?;
    let first_revision = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .object_revision(first_key);
    assert_eq!(
        super::l6_world_admission::use_source(owner, admitted, first, first_revision).await?,
        UseDisposition::Rejected,
        "an exclusive request cannot enter a room held by an ordinary timer"
    );
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .object_revision(first_key),
        first_revision
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        before, after,
        "reservation and cleanup grant no quest progress"
    );
    Ok(())
}
