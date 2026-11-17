//! Native mirror USE through the admitted PostgreSQL-backed command path.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

pub(super) const NAME: &str = "blood-brothers-lersatio-mirror-1-first";
const MIRROR: Position = Position {
    x: 32965,
    y: 31457,
    floor: -2,
};

#[test]
fn lersatio_first_mirror_use_advances_durable_counter() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("lersatio-first-use", || {
        quest_world_callbacks::scenario(NAME, MIRROR, false)
    })
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let cells: Vec<_> = (-2..=-1)
        .flat_map(|floor| {
            (32954..=32972)
                .flat_map(move |x| (31447..=31465).map(move |y| Position { x, y, floor }))
        })
        .collect();
    quest_world_callbacks::world_with_scene_cells(
        callback,
        MIRROR,
        &[
            (
                Position {
                    x: 32967,
                    y: 31457,
                    floor: -2,
                },
                8325,
            ),
            (
                Position {
                    x: 32966,
                    y: 31456,
                    floor: -2,
                },
                8667,
            ),
            (
                Position {
                    x: 32968,
                    y: 31456,
                    floor: -2,
                },
                8667,
            ),
        ],
        &[8326, 8721],
        &cells,
    )
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use super::questpc8_authored_programmes::{object_target, use_target};
    use crate::map::overlay::TilePos;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let target = object_target(
        owner,
        TilePos {
            x: MIRROR.x,
            y: MIRROR.y,
            floor: MIRROR.floor,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, 1, &target).await?,
        UseDisposition::Committed
    );
    assert_ne!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Committed,
        "a broken mirror cannot advance the counter again"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(super::fixture::debug)?;
    assert_eq!(
        state
            .tracks()
            .get("oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mirror_counter"),
        Some(&1)
    );
    use crate::map::view::MapFacts;
    let map = owner.bundle_map().ok_or("mirror map")?;
    let key = crate::map::view::placement_key(
        TilePos {
            x: MIRROR.x,
            y: MIRROR.y,
            floor: MIRROR.floor,
        },
        1,
    )
    .ok_or("mirror key")?;
    assert_eq!(
        map.facts.object_revision(key),
        1,
        "USE publishes mirror transform"
    );
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("mirror reset timer")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(
        map.facts.object_revision(key),
        2,
        "current mirror reset publishes once"
    );
    let spectator_position = crate::foundation::MovementLocalPosition {
        x: 32963,
        y: 31458,
        floor: 2,
    };
    let monster_spectator = owner
        .runtime
        .lock()
        .await
        .admit_pinned_test_creature(spectator_position)
        .map_err(super::fixture::debug)?;
    let departed_position = crate::foundation::MovementLocalPosition {
        x: 32973,
        y: 31460,
        floor: 2,
    };
    let departed = {
        let mut runtime = owner.runtime.lock().await;
        let session = crate::foundation::GameSessionId::decode(&super::fixture::id(99))
            .map_err(super::fixture::debug)?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(super::fixture::debug)?;
        let actor = runtime
            .commit_fresh_session(reservation)
            .map_err(super::fixture::debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                crate::foundation::MovementLocalPosition {
                    x: 32972,
                    y: 31460,
                    floor: 2,
                },
            )
            .map_err(super::fixture::debug)?;
        let before = runtime
            .read_actor_position(actor)
            .map_err(super::fixture::debug)?;
        runtime
            .borrow_movement_position()
            .commit_cardinal(before, departed_position)
            .map_err(super::fixture::debug)?;
        actor
    };
    let mut present = Vec::new();
    for ordinal in 100..109 {
        let mut runtime = owner.runtime.lock().await;
        let session = crate::foundation::GameSessionId::decode(&super::fixture::id(ordinal))
            .map_err(super::fixture::debug)?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(super::fixture::debug)?;
        let actor = runtime
            .commit_fresh_session(reservation)
            .map_err(super::fixture::debug)?;
        runtime
            .initialize_pinned_test_position(actor, spectator_position)
            .map_err(super::fixture::debug)?;
        let mut states = owner.states.spell_states.lock().await;
        let facts = states
            .get(
                &runtime,
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
            )
            .ok_or("admitted player state")?
            .character_facts();
        states
            .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
            .ok_or("spectator player state")?;
        present.push(actor);
    }
    let destination = crate::foundation::MovementLocalPosition {
        x: 32967,
        y: 31459,
        floor: 1,
    };
    let destination_player = {
        let mut runtime = owner.runtime.lock().await;
        let session = crate::foundation::GameSessionId::decode(&super::fixture::id(110))
            .map_err(super::fixture::debug)?;
        let reservation = runtime
            .reserve_fresh_session(session)
            .map_err(super::fixture::debug)?;
        let actor = runtime
            .commit_fresh_session(reservation)
            .map_err(super::fixture::debug)?;
        runtime
            .initialize_pinned_test_position(actor, destination)
            .map_err(super::fixture::debug)?;
        actor
    };
    let restored = object_target(
        owner,
        TilePos {
            x: MIRROR.x,
            y: MIRROR.y,
            floor: MIRROR.floor,
        },
        1,
    )?;
    assert_eq!(
        use_target(owner, admitted, 3, &restored).await?,
        UseDisposition::Committed
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(super::fixture::debug)?;
    assert_eq!(
        state
            .tracks()
            .get("oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mirror_counter"),
        Some(&0)
    );
    assert_eq!(
        use_target(owner, admitted, 3, &restored).await?,
        UseDisposition::Committed,
        "replay returns the original committed result"
    );
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(super::fixture::debug)?
            .position(),
        destination
    );
    assert_eq!(
        runtime
            .read_actor_position(monster_spectator)
            .map_err(super::fixture::debug)?
            .position(),
        spectator_position,
        "monster spectators must remain in the source room"
    );
    for present in present {
        assert_eq!(
            runtime
                .read_actor_position(present)
                .map_err(super::fixture::debug)?
                .position(),
            destination,
            "every player currently in the room transfers, including one sharing a monster tile"
        );
    }
    assert_eq!(
        runtime
            .read_actor_position(departed)
            .map_err(super::fixture::debug)?
            .position(),
        departed_position,
        "a player who left the room must remain outside"
    );
    assert_eq!(
        runtime
            .read_actor_position(destination_player)
            .map_err(super::fixture::debug)?
            .position(),
        destination,
        "a player already at the destination neither joins the source census nor blocks it"
    );
    let census = runtime
        .positioned_melee_census()
        .map_err(super::fixture::debug)?;
    for (identity, expected) in [
        ("oteryn:creature.lersatio", 1),
        ("oteryn:creature.vampire", 2),
    ] {
        assert_eq!(
            census
                .iter()
                .filter(|(actor, at, session)| {
                    session.is_none()
                        && at.position().floor == 1
                        && runtime.matches_live_creature_identity(*actor, identity.as_bytes())
                })
                .count(),
            expected,
            "one donor cohort even after command replay"
        );
    }
    drop(runtime);
    let map = owner.bundle_map().ok_or("mirror map")?;
    let current = map.facts.object_revision(key);
    let stale_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("second mirror reset")?;
    let newer = map
        .facts
        .quest_callback_replacement(key, 8721)
        .ok_or("broken mirror facts")?;
    assert!(
        map.facts
            .apply_quest_callback(key, current, newer, &[], &[], false, None)
    );
    assert_eq!(map.facts.object_revision(key), current + 1);
    map.facts.expire_movement_items(stale_due);
    assert_eq!(
        map.facts.object_revision(key),
        current + 1,
        "the mirror timer created by real USE cannot overwrite a newer source revision"
    );
    Ok(())
}
