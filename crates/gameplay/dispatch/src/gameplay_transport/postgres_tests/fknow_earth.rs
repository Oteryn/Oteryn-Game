//! Runs the timed growing-birch projection through the normal USE dispatcher.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "fknow-earth-birch";
const AT: Position = Position {
    x: 32736,
    y: 32116,
    floor: -10,
};
const PLANTING_SOIL: u16 = 23814;
const SEEDLING: u16 = 23816;
const BIRCH: u16 = 23815;
const SOIL: u16 = 26482;

#[test]
fn fknow_earth_birch_expires_to_soil_after_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        quest_world_callbacks::scenario(NAME, AT, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    stage("plant-birch", PLANTING_SOIL, SEEDLING, "415-426")
}

pub(super) fn watering() -> TestResult<Callback> {
    stage("grow-birch", SEEDLING, BIRCH, "425-431")
}

fn stage(name: &str, from: u16, to: u16, lines: &str) -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": format!("oteryn:quest-callback/forgotten-knowledge/{name}"),
        "quest": "oteryn:quest.forgotten_knowledge_quest",
        "trigger": "use",
        "selector": { "position": AT, "appearance": from },
        "reversible": false,
        "passages": [],
        "effects": { "transform": to, "reset_after_ms": 600000, "reset_appearance": SOIL },
        "storage": [],
        "offerings": [],
        "return_on_failure": false,
        "sources": [format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/actions_plant.lua:{lines}")]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    quest_world_callbacks::world_with_objects(callback, AT, &[], &[BIRCH, SEEDLING, SOIL])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let pos = TilePos {
        x: AT.x,
        y: AT.y,
        floor: AT.floor,
    };
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("birch placement")?;
    let item = *map
        .overlay
        .base()
        .tile(AT.x, AT.y, AT.floor)
        .ok_or("birch tile")?
        .ids()
        .get(1)
        .ok_or("birch item")?;
    let appearance = || -> TestResult<u16> {
        Ok(map
            .facts
            .base_entry(pos, 1, item)
            .ok_or("birch facts")?
            .appearance_id)
    };
    assert_eq!(appearance()?, PLANTING_SOIL);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    for (revision, placed, now) in [(0, SEEDLING, 5_000_000), (1, BIRCH, 6_000_000)] {
        owner
            .states
            .test_owner_time
            .store(now + 1, std::sync::atomic::Ordering::Relaxed);
        super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let reply = quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "birch stage at placement revision {revision}"
        );
        assert_eq!(appearance()?, placed);
    }
    let planting_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("planting timer")?;
    assert_eq!(planting_due, 605_000_000);
    assert_eq!(map.facts.expire_movement_items(planting_due), 0);
    assert_eq!(appearance()?, BIRCH, "the first timer cannot undo watering");

    let due = map.facts.next_movement_decay_time().ok_or("birch timer")?;
    assert_eq!(due, 606_000_000);
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(appearance()?, SOIL);
    Ok(())
}
