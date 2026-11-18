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
pub(super) const CURE_NAME: &str = "qcx60-geyser-carried-tool";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission03";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_anthill.lua:3-11";
const CURE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_cure.lua:2-10";
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

#[test]
fn ice_islands_geyser_transforms_the_vial_without_advancing_the_quest() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CURE_NAME, || {
        super::quest_world_callbacks::scenario(CURE_NAME, TARGET, false)
    })
}

pub(super) fn callback(cure: bool) -> TestResult<Callback> {
    let (appearance, item, replacement, sources) = if cure {
        (
            7106,
            "oteryn:item.tibia.i7286",
            "oteryn:item.tibia.i7246",
            CURE,
        )
    } else {
        (
            1079,
            "oteryn:item.tibia.i7243",
            "oteryn:item.tibia.i7244",
            SOURCE,
        )
    };
    let storage = if cure {
        json!([{"track": QUESTLINE, "minimum": 21, "maximum": 44, "set": 21, "preserve": true}])
    } else {
        json!([
            {"track": QUESTLINE, "minimum": 6, "maximum": 6, "set": 6, "preserve": true},
            {"track": MISSION, "minimum": 1, "maximum": 2, "set": 2}
        ])
    };
    Ok(serde_json::from_value(json!({
        "key": if cure { "oteryn:quest-callback/qcx60/geyser-fixture" } else { "oteryn:quest-callback/qcx60/anthill-fixture" },
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "appearance": appearance,
            "native": { "position": TARGET, "ordinal": 1, "appearance": appearance }
        },
        "reversible": false,
        "requires_world_admission": true,
        "passages": [],
        "storage": storage,
        "item_on_target": {
            "item": item,
            "count": 1,
            "replacement": replacement,
            "selected_instance": true
        },
        "offerings": [],
        "return_on_failure": false,
        "sources": [sources]
    }))?)
}

pub(super) fn catalogue(callback: &Callback, cure: bool) -> TestResult<QuestStateCatalogue> {
    let (source, digest, initial) = if cure {
        (
            CURE,
            "29edaa2a436dd2a59444df3bacc77205f8e5804a0dc4b46a29c3e96f3c63618f",
            21,
        )
    } else {
        (
            SOURCE,
            "8bd1d2778fbf439ce4e03d8621ed5e979ed294182769f5f73fdeb0b446448738",
            6,
        )
    };
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": digest,
            "tracks": [
                {"key": QUESTLINE, "initial": initial, "minimum": -1, "maximum": 44},
                {"key": MISSION, "initial": 1, "minimum": 1, "maximum": 3}
            ],
            "stages": [{
                "key": "fill-tool",
                "completes": false,
                "trigger": {"kind": "map_use", "callback": callback}
            }],
            "sources": [source]
        }]
    }))?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    cure: bool,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let (tool, wrong_tool, transformed) = if cure {
        (7286, 7243, 7246)
    } else {
        (7243, 7286, 7244)
    };
    let item_key = |id| format!("oteryn:item.tibia.i{id}");
    for (nonce, id) in [(951, tool), (952, tool), (953, wrong_tool)] {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: TARGET,
                item: item_key(id),
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
        Some(item(&item_key(wrong_tool))?),
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
        Some(item(&item_key(tool))?),
        1,
    )
    .await?;
    *admitted = next;
    assert_eq!(accepted, UseDisposition::Committed);

    let mut expected = [item_key(tool), item_key(transformed), item_key(wrong_tool)];
    expected.sort_unstable();
    let inventory: Vec<String> = sqlx::query_scalar(
        "SELECT definition_production_key FROM game_item_instances WHERE lifecycle=1 \
         AND definition_production_key=ANY($1::text[]) ORDER BY definition_production_key",
    )
    .bind(expected.as_slice())
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(inventory, expected);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    if cure {
        assert_eq!(state.tracks().get(QUESTLINE), Some(&21));
        assert!(!state.tracks().contains_key(MISSION));
    } else {
        assert_eq!(state.tracks().get(MISSION), Some(&2));
    }
    Ok(())
}
