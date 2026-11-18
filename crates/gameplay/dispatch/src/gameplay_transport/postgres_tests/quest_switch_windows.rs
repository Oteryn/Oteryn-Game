//! Shared deadlines through native dispatch and its Channel owner.
use super::TestResult;

#[test]
fn timed_encounter_moves_the_admitted_group_at_expiry_and_resets_the_lever() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::annihilator_route::timed_group_transfer(admin)
}

#[test]
fn ashmunrah_step_counts_only_the_stepping_characters_durable_switches() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::annihilator_route::personal_switch_threshold(admin)
}

pub(super) async fn ashmunrah(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [super::AdmittedSession],
    pool: &sqlx::PgPool,
) -> TestResult {
    use super::{annihilator_route::{step, use_target}, fixture::debug, l6_shared_gate_native::relocate};
    use crate::map::{overlay::TilePos, view::{MapFacts, placement_key}};
    use oteryn_protocol_oteryn::{world_object::{UseDisposition, WorldObjectTarget}, world_spatial::StepDirection};

    // Crystal 201189f8 actions_active_teleport_switches.lua:18-21,39-50
    // and movements_enter_ashmunrah_teleport_switche_done.lua:9-16.
    let switches = [
        ((33176, 32880), (33177, 32880)),
        ((33182, 32880), (33181, 32880)),
        ((33175, 32884), (33176, 32884)),
        ((33183, 32884), (33182, 32884)),
        ((33176, 32889), (33177, 32889)),
        ((33181, 32889), (33180, 32889)),
    ];
    let origin = (33181, 32884, 11);
    for (index, ((x, y), (from_x, from_y))) in switches.into_iter().enumerate() {
        let member = &mut admitted[usize::from(index == 4)];
        let actor = member.runtime_actor.ok_or("switch operator")?;
        relocate(owner, actor, member.game_session_id, (from_x, from_y, 11)).await?;
        let map = owner.bundle_map().ok_or("Ashmunrah map")?;
        let key = placement_key(TilePos { x, y, floor: -11 }, 2).ok_or("switch placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        assert_eq!(use_target(owner, member, &target).await?, UseDisposition::Committed);
        if index >= 4 {
            let member = &mut admitted[0];
            let actor = member.runtime_actor.ok_or("stepping actor")?;
            relocate(owner, actor, member.game_session_id, origin).await?;
            step(owner, member, StepDirection::East).await?;
            let at = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
            assert_eq!((at.x, at.y, at.floor), if index == 4 { (33181, 32884, 11) } else { (33198, 32885, 11) });
        }
    }
    let per_character: Vec<i64> = sqlx::query_scalar(
        "SELECT count(*) AS switches FROM game_shared_switch_leases
         WHERE recipe='oteryn:quest-callback/ancient-tombs-ashmunrah-switch'
         GROUP BY character_id ORDER BY switches",
    ).fetch_all(pool).await?;
    assert_eq!(per_character, [1, 5]);
    Ok(())
}

#[test]
fn reversible_switch_expires_and_an_old_deadline_cannot_close_a_reopened_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::annihilator_route::timed_passage(admin)
}

pub(super) async fn reversible_passage(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    member: &mut super::AdmittedSession,
    map: &crate::map::boot::BundleMap,
) -> TestResult {
    use super::annihilator_route::{target, use_target};
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    use std::sync::atomic::Ordering;

    let binding = oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter()
        .find(|callback| callback.key.ends_with("/annihilator-entry"))
        .ok_or("fixture lever")?;
    let mut authored = serde_json::to_value(binding)?;
    authored["effects"] = serde_json::json!({"passages_reset_after_ms": 100_000});
    authored["reversible"] = true.into();
    // Crystal 201189f8 action-1-first_seal_lever.lua:24-36. Use the fixture's
    // real lever and remote door to prove the shared world reset in dispatch.
    authored["passages"] = serde_json::json!([{
        "position": {"x": 33225, "y": 31659, "floor": -13}, "appearance": 5113
    }]);
    let callback = serde_json::from_value(authored)?;
    assert!(owner.states.quest_callbacks.fixture.set(vec![callback]).is_ok());
    let wall = target(map, 33225, 31659)?;
    let wall_key = u64::from_be_bytes(wall.placement[32..].try_into()?);
    owner.states.test_owner_time.store(100_000_001, Ordering::Relaxed);
    assert_eq!(use_target(owner, member, &target(map, 33226, 31671)?).await?, UseDisposition::Committed);
    assert!(map.facts.base_hidden(wall_key));
    assert_eq!(map.facts.next_movement_decay_time(), Some(200_000_000));

    owner.states.test_owner_time.store(110_000_001, Ordering::Relaxed);
    assert_eq!(use_target(owner, member, &target(map, 33226, 31671)?).await?, UseDisposition::Committed);
    assert!(!map.facts.base_hidden(wall_key));

    owner.states.test_owner_time.store(120_000_001, Ordering::Relaxed);
    assert_eq!(use_target(owner, member, &target(map, 33226, 31671)?).await?, UseDisposition::Committed);
    assert!(map.facts.base_hidden(wall_key));
    map.facts.expire_movement_items(200_000_000);
    assert!(map.facts.base_hidden(wall_key), "old window cannot close the reopened wall");
    map.facts.expire_movement_items(219_999_999);
    assert!(map.facts.base_hidden(wall_key));
    map.facts.expire_movement_items(220_000_000);
    assert!(!map.facts.base_hidden(wall_key));
    assert_eq!(map.facts.next_movement_decay_time(), None);
    Ok(())
}

use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) const LORD_START: Position = at(33263, 31831, -12);
const LORD_EXIT: Position = at(33265, 31838, -10);

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

#[test]
fn lord_chamber_last_switch_starts_the_window_and_expiry_resets_the_entire_circuit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::annihilator_route::lord_switches(admin)
}

pub(super) async fn lord(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [super::AdmittedSession],
    map: &crate::map::boot::BundleMap,
) -> TestResult {
    use super::{annihilator_route::use_target, fixture::debug, quest_world_callbacks::local};
    use crate::map::{floor::BundleDoors, overlay::TilePos, view::{MapFacts, placement_key}};
    use oteryn_gameplay_quests::quest::world_callbacks::Callback;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    use std::sync::atomic::Ordering;

    // Crystal 201189f8 actions_lord_of_elements_lever.lua:1-30,63-99,143-188.
    let switches = [
        (at(33273,31840,-12), at(33272,31840,-12), at(33275,31834,-12), at(33275,31838,-12), 5072),
        (at(33262,31840,-12), at(33263,31840,-12), at(33266,31843,-12), at(33270,31843,-12), 5071),
        (at(33262,31831,-12), LORD_START, at(33260,31834,-12), at(33260,31838,-12), 5072),
        (at(33273,31831,-12), at(33272,31831,-12), at(33266,31828,-12), at(33270,31828,-12), 5071),
    ];
    let window: Vec<_> = switches.iter().map(|&(lever, station, from, to, appearance)| {
        serde_json::json!({
            "lever": lever, "station": station, "wall_from": from, "wall_to": to,
            "wall_appearance": appearance
        })
    }).collect();
    let mut callbacks = Vec::new();
    for (index, &(lever, _, _, _, _)) in switches.iter().enumerate() {
        callbacks.push(serde_json::from_value::<Callback>(serde_json::json!({
            "key": format!("oteryn:quest-callback/lord-of-elements-switch-{}", index + 1),
            "quest": "oteryn:quest.elemental_spheres_quest",
            "trigger": "use",
            "selector": {"position": lever, "appearance": 2772},
            "lever": [2772, 2773],
            "reversible": false,
            "passages": [], "storage": [], "offerings": [],
            "return_on_failure": false,
            "effects": {
                "encounter": {
                    "minimum_level": 1,
                    "players": [],
                    "monsters": [{"position": at(33267,31836,-12), "creature": "oteryn:creature.lord_of_the_elements"}],
                    "room_min": at(33238,31806,-12),
                    "room_max": at(33297,31865,-12)
                },
                "timed_group_transfer": {"after_ms": 600_000, "destination": LORD_EXIT, "switches": window}
            },
            "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_lord_of_elements_lever.lua:63-188"],
        }))?);
    }
    assert!(owner.states.quest_callbacks.fixture.set(callbacks).is_ok());

    let target_for = |lever: Position| -> TestResult<WorldObjectTarget> {
        let tile = TilePos { x: lever.x, y: lever.y, floor: lever.floor };
        let base = map.overlay.base().tile(tile.x, tile.y, tile.floor).ok_or("lever tile")?;
        let found = base.ids().iter().zip(base.depths()).filter(|(_, depth)| **depth == 0)
            .enumerate().find_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let facts = map.facts.base_entry(tile, ordinal, *id)?;
                [2772, 2773].contains(&facts.appearance_id)
                    .then(|| placement_key(tile, ordinal)).flatten()
            }).ok_or("source lever")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&found.to_be_bytes());
        Ok(WorldObjectTarget { placement, expected_revision: map.facts.object_revision(found) })
    };

    // The east switch is deliberately used last. Only that use starts the 10-minute timer.
    for (turn, (which_switch, which_member)) in [(2, 0), (3, 1), (1, 1), (0, 0)].into_iter().enumerate() {
        let (lever, station, wall, _, _) = switches[which_switch];
        let member = &mut members[which_member];
        {
            let actor = member.runtime_actor.ok_or("actor")?;
            let mut runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(actor).map_err(debug)?;
            if expected.position() != local(station) {
                let doors = BundleDoors(&map.facts);
                let proof = crate::map::floor::prepare_quest_encounter_ejection(
                    &runtime, owner.movement_cells(), actor, member.game_session_id, expected,
                    local(station), &doors,
                ).map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)?;
            }
        }
        owner.states.test_owner_time.store(100_000_001 + u64::try_from(turn)? * 1_000_000, Ordering::Relaxed);
        assert_eq!(use_target(owner, member, &target_for(lever)?).await?, UseDisposition::Committed);
        let wall = TilePos { x: wall.x, y: wall.y, floor: wall.floor };
        assert!(map.facts.quest_callback_blocked(map.overlay.base(), wall, false));
        assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), (turn == 3).then_some(703_000_000));
    }
    assert_eq!(owner.runtime.lock().await.visible_entities().creatures.len(), 1);
    owner.states.test_owner_time.store(703_000_001, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    let runtime = owner.runtime.lock().await;
    for member in members {
        let actor = member.runtime_actor.ok_or("actor")?;
        assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), local(LORD_EXIT));
    }
    assert!(runtime.visible_entities().creatures.is_empty());
    assert_eq!(owner.states.quest_callbacks.next_encounter_due_us(), None);
    for (lever, _, wall, _, _) in switches {
        assert_eq!(target_for(lever)?.expected_revision, 2);
        let wall = TilePos { x: wall.x, y: wall.y, floor: wall.floor };
        assert!(!map.facts.quest_callback_blocked(map.overlay.base(), wall, false));
    }
    Ok(())
}
