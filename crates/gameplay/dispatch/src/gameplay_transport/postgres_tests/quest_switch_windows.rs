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
