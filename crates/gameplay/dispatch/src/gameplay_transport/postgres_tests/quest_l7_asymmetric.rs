//! L7 source-literal lever/rock truth table through admitted USE dispatch.
use super::TestResult;
use super::fixture::debug;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

#[test]
fn quest_l7_giant_smithhammer_four_source_states() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("l7-smithhammer", || {
        super::quest_world_callbacks::scenario(
            "giant-smithhammer-asymmetric",
            Position {
                x: 32779,
                y: 32230,
                floor: -8,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    callback: &Callback,
    trigger: Position,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    let pos = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let key = crate::map::view::placement_key(pos, 1).ok_or("lever key")?;
    let rock = callback.passages.first().ok_or("rock")?;
    let rock_pos = crate::map::overlay::TilePos {
        x: rock.position.x,
        y: rock.position.y,
        floor: rock.position.floor,
    };
    let rock_key = crate::map::view::placement_key(rock_pos, 1).ok_or("rock key")?;
    // Seed each independent source state through the existing projection owner.
    // Open+absent and closed+present must leave BOTH objects untouched.
    for (index, (closed, absent)) in [(false, true), (true, false), (false, false), (true, true)]
        .into_iter()
        .enumerate()
    {
        // Drive the fixture clock beyond USE exhaustion, preserving the real
        // command pacing checks and avoiding a wall-clock sleep.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("fixture clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        let mut before = map.facts.base_entry(pos, 1, 1).ok_or("lever")?;
        before.appearance_id = if closed { 2773 } else { 2772 };
        assert!(map.facts.apply_quest_callback(
            key,
            map.facts.object_revision(key),
            before,
            &[rock_key],
            &[],
            !absent, None));
        assert_eq!(map.facts.base_hidden(rock_key), absent);
        let revision = map.facts.object_revision(key);
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            u64::try_from(index)? + 1,
            2,
            encode_use_intent(&WorldObjectTarget {
                placement,
                expected_revision: revision,
            })
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "USE branch closed={closed} rock_absent={absent}"
        );
        let should_change = closed == absent;
        assert_eq!(
            map.facts.object_revision(key),
            revision + u64::from(should_change),
            "source branch closed={closed} rock_absent={absent}"
        );
        assert_eq!(
            map.facts.base_hidden(rock_key),
            if should_change { !closed } else { absent }
        );
        assert_eq!(
            world.enterable(rock_pos),
            map.facts.base_hidden(rock_key),
            "rock visibility and collision must agree",
        );
        assert_eq!(
            map.facts
                .base_entry(pos, 1, 1)
                .ok_or("lever after")?
                .appearance_id,
            if closed ^ should_change { 2773 } else { 2772 }
        );
    }
    eprintln!("POSTGRES_CASE_COMPLETED L7 giant-smithhammer four source states via USE");
    Ok(())
}
