//! Crystal 201189f8 register_actions.lua:698-750, Breaking the Ice.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use serde_json::json;

pub(super) const NAME: &str = "qcx60-pick-chakoya";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/register_actions.lua:698-750";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const PICK: &str = "oteryn:item.tibia.i3456";
const COUNT: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/pick_amount";
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission02";
const CRACK: Position = Position {
    x: 32399,
    y: 31051,
    floor: -7,
};
const SPAWNS: [Position; 2] = [
    Position {
        x: 32397,
        y: 31048,
        floor: -7,
    },
    Position {
        x: 32399,
        y: 31048,
        floor: -7,
    },
];
const CREATURES: [&str; 3] = [
    "oteryn:creature.chakoya_toolshaper",
    "oteryn:creature.chakoya_tribewarden",
    "oteryn:creature.chakoya_windcaller",
];

#[test]
fn picking_the_ice_spawns_chakoyas_and_preserves_the_existing_crack_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, CRACK, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key": "oteryn:quest-callback/qcx60/breaking-ice-fixture",
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "appearance": 7185,
            "native": { "position": CRACK, "ordinal": 1, "appearance": 7185 }
        },
        "reversible": false,
        "requires_world_admission": true,
        "passages": [],
        "storage": [
            {"track": COUNT, "minimum": 0, "maximum": 2, "set": 1, "increment": true},
            {"track": MISSION, "minimum": 1, "maximum": 3, "set": 1, "increment": true}
        ],
        "item_on_target": {
            "item": PICK,
            "count": 1,
            "reusable": true,
            "selected_instance": true
        },
        "effects": {
            "transform": 7186,
            "reset_after_ms": 60000,
            "use_spawn": {"positions": SPAWNS, "creatures": CREATURES, "effect": 11}
        },
        "offerings": [],
        "return_on_failure": false,
        "sources": [SOURCE]
    }))?)
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": "5121124402c29aead613174c80e94e6b8d5c7db1e62ad152ad68f1280fc3df6d",
            "tracks": [
                {"key": COUNT, "initial": 0, "minimum": 0, "maximum": 3},
                {"key": MISSION, "initial": 1, "minimum": -1, "maximum": 4}
            ],
            "stages": [{
                "key": "pick",
                "completes": false,
                "trigger": {"kind": "map_use", "callback": callback}
            }],
            "sources": [SOURCE]
        }]
    }))?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: CRACK,
            item: PICK.into(),
            count: 1,
        },
        953,
        true,
    )
    .await?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let key = oteryn_world_bundle::bundle::placement_key(CRACK.floor, CRACK.x, CRACK.y, 1)
        .ok_or("crack placement")?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", PICK, "definition-r1"))
        .ok_or("pick ref")?;

    let mut reset = None;
    for count in 1..=2_i64 {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let map = owner.bundle_map().ok_or("map")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
            use_with_position: None,
        })
        .map_err(debug)?;
        let (next, accepted, _) = super::item_craft::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(reference),
        )
        .await?;
        *admitted = next;
        assert_eq!(accepted, UseDisposition::Committed);

        let due = map.facts.next_movement_decay_time().ok_or("crack reset")?;
        assert_eq!(*reset.get_or_insert(due), due);
        let creatures = owner.runtime.lock().await.visible_entities().creatures;
        for point in SPAWNS {
            assert_eq!(
                creatures
                    .iter()
                    .filter(|creature| {
                        creature.position == super::quest_world_callbacks::local(point)
                    })
                    .count(),
                1
            );
        }
        let state = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(COUNT), Some(&count));
    }
    let burns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(burns, 0);
    Ok(())
}
