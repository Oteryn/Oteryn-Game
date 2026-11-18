//! Literal USE of the registered folio action in a source-identity map fixture.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const NAME: &str = "shadows-of-yalahar-folio";
const TRIGGER: Position = Position {
    x: 33322,
    y: 32173,
    floor: -6,
};

const RELOCATION: Position = Position {
    x: 33324,
    y: 32173,
    floor: -6,
};
const MISSION: &str = "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09";
const STREAK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_books_success";
const HEADACHE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_headache_until";
const OUTCOME: &str = "oteryn:quest-progress/shadows-of-yalahar/folio-outcome";

#[test]
fn shyal2_live_folio_use_projects_the_committed_ritual_outcome() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TRIGGER, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let mut placement = callback.clone();
    placement.selector.appearance = Some(9193);
    super::quest_world_callbacks::world_with_cells(&placement, TRIGGER, &[RELOCATION])
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let now = 1_800_000_000;
    h.root.set_test_quest_clock(now).map_err(debug)?;
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(MISSION.into(), 1), (STREAK.into(), 7)],
        125,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let actor = admitted.runtime_actor.ok_or("folio reader")?;
    let (before, before_creatures) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            runtime.visible_entities().creatures.len(),
        )
    };
    let key = oteryn_world_bundle::bundle::placement_key(TRIGGER.floor, TRIGGER.x, TRIGGER.y, 1)
        .ok_or("folio placement key")?;
    let map = owner.bundle_map().ok_or("folio source map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let result = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "folio quest fence")?
        .ok_or("folio fence")?;
    let state = h
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let outcome = state
        .tracks()
        .get(OUTCOME)
        .copied()
        .ok_or("committed roll")?;
    let (streak, deadline) = match outcome {
        1 => (0, 0),
        2..=5 => (0, now + 60),
        6 => (8, 0),
        _ => return Err(format!("invalid folio outcome: {outcome}").into()),
    };
    assert_eq!(state.tracks().get(STREAK), Some(&streak));
    assert_eq!(state.tracks().get(HEADACHE), Some(&deadline));
    let (after, creatures) = {
        let runtime = owner.runtime.lock().await;
        let creatures = runtime.visible_entities().creatures;
        let hostile = match outcome {
            3 => Some("oteryn:creature.war_golem"),
            4 => Some("oteryn:creature.fire_devil"),
            5 => Some("oteryn:creature.enraged_bookworm"),
            _ => None,
        };
        assert_eq!(
            creatures.len(),
            before_creatures + usize::from(hostile.is_some())
        );
        if let Some(hostile) = hostile {
            assert!(creatures.iter().any(|creature| {
                runtime.matches_live_creature_identity(creature.actor, hostile.as_bytes())
            }));
        }
        (
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            creatures.len(),
        )
    };
    assert_eq!(
        after,
        if outcome == 1 {
            super::quest_world_callbacks::local(RELOCATION)
        } else {
            before
        }
    );
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?,
        result
    );
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        after
    );
    assert_eq!(runtime.visible_entities().creatures.len(), creatures);
    Ok(())
}
