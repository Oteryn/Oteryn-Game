//! Crystal 201189f8 Ice Islands tool USE, with a synthetic map and authored bindings.
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

pub(super) const NAME: &str = "qcx60-anthill-carried-tool";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission03";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_anthill.lua:3-11";
const TARGET: Position = Position {
    x: 32360,
    y: 31365,
    floor: -7,
};

#[test]
fn ice_islands_anthill_requires_the_carried_jug_and_transforms_one() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key": "oteryn:quest-callback/qcx60/anthill-fixture",
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "appearance": 1079,
            "native": { "position": TARGET, "ordinal": 1, "appearance": 1079 }
        },
        "reversible": false,
        "requires_world_admission": true,
        "passages": [],
        "storage": [
            {"track": QUESTLINE, "minimum": 6, "maximum": 6, "set": 6, "preserve": true},
            {"track": MISSION, "minimum": 1, "maximum": 2, "set": 2}
        ],
        "item_on_target": {
            "item": "oteryn:item.tibia.i7243",
            "count": 1,
            "replacement": "oteryn:item.tibia.i7244",
            "selected_instance": true
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
            "recipe_sha256": "8bd1d2778fbf439ce4e03d8621ed5e979ed294182769f5f73fdeb0b446448738",
            "tracks": [
                {"key": QUESTLINE, "initial": 6, "minimum": 6, "maximum": 7},
                {"key": MISSION, "initial": 1, "minimum": 1, "maximum": 3}
            ],
            "stages": [{
                "key": "fill-jug",
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
    for (nonce, id) in [(951, 7243), (952, 7243), (953, 7286)] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: TARGET,
                item: format!("oteryn:item.tibia.i{id}"),
                count: 1,
            },
            nonce,
            true,
        )
        .await?;
    }
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 1)
        .ok_or("anthill placement")?;
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
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("item index")?;

    let item = |key| {
        index
            .definition_ref("Item", key, "definition-r1")
            .ok_or("item reference")
    };
    let (next, wrong, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(item("oteryn:item.tibia.i7286")?),
    )
    .await?;
    *admitted = next;
    assert_eq!(wrong, UseDisposition::NothingToUse);
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let (next, accepted, _) = super::item_craft::exchange_selected(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(item("oteryn:item.tibia.i7243")?),
        1,
    )
    .await?;
    *admitted = next;
    assert_eq!(accepted, UseDisposition::Committed);

    let inventory: Vec<String> = sqlx::query_scalar(
        "SELECT definition_production_key FROM game_item_instances WHERE lifecycle=1 \
         AND definition_production_key IN ('oteryn:item.tibia.i7243','oteryn:item.tibia.i7244','oteryn:item.tibia.i7286') \
         ORDER BY definition_production_key",
    ).fetch_all(&harness.pool).await?;
    assert_eq!(
        inventory,
        [
            "oteryn:item.tibia.i7243",
            "oteryn:item.tibia.i7244",
            "oteryn:item.tibia.i7286"
        ]
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(MISSION), Some(&2));
    Ok(())
}
