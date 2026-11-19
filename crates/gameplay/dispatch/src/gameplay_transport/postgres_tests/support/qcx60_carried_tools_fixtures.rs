use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::FreshAdmissionAuthority;
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
pub(super) const RETALIATION_NAME: &str = "qcx60-ice-charm-carried-tool";
pub(super) const MAST_WITH_JUG: &str = "qcx60-mast-with-empty-jug";
pub(super) const MAST_WITHOUT_JUG: &str = "qcx60-mast-without-empty-jug";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission03";
const RETALIATION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission12";
const OBELISK: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/obelisk01";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_anthill.lua:3-11";
const CURE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_cure.lua:2-10";
const CHARM: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_charm.lua:3-23";
const RELEASE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_anthill.lua:13-20";
pub(super) const ANTHILL_TARGET: Position = Position {
    x: 32604,
    y: 32850,
    floor: -7,
};
pub(super) const GEYSER_TARGET: Position = Position {
    x: 32231,
    y: 31391,
    floor: -7,
};
pub(super) const MAST_TARGET: Position = Position {
    x: 32360,
    y: 31365,
    floor: -7,
};
pub(super) const CHARM_TARGET: Position = Position {
    x: 32138,
    y: 31113,
    floor: -14,
};

pub(super) fn matches_name(name: &str) -> bool {
    matches!(
        name,
        NAME | CURE_NAME | RETALIATION_NAME | MAST_WITH_JUG | MAST_WITHOUT_JUG
    )
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let cure = name == CURE_NAME;
    let retaliation = name == RETALIATION_NAME;
    let mast = matches!(name, MAST_WITH_JUG | MAST_WITHOUT_JUG);
    let (appearance, item, replacement, sources, position, ordinal) = if retaliation {
        (
            1841,
            "oteryn:item.tibia.i7289",
            None,
            CHARM,
            CHARM_TARGET,
            2,
        )
    } else if cure {
        (
            7106,
            "oteryn:item.tibia.i7286",
            Some("oteryn:item.tibia.i7246"),
            CURE,
            GEYSER_TARGET,
            2,
        )
    } else if mast {
        (
            4940,
            "oteryn:item.tibia.i7244",
            None,
            RELEASE,
            MAST_TARGET,
            1,
        )
    } else {
        (
            1079,
            "oteryn:item.tibia.i7243",
            Some("oteryn:item.tibia.i7244"),
            SOURCE,
            ANTHILL_TARGET,
            1,
        )
    };
    let storage = if retaliation {
        json!([
            {"track": QUESTLINE, "minimum": 39, "maximum": 39, "set": 39, "preserve": true},
            {"track": OBELISK, "minimum": 0, "maximum": 4, "set": 5},
            {"track": RETALIATION, "minimum": 0, "maximum": i32::MAX - 1, "set": 1, "increment": true}
        ])
    } else if cure {
        json!([{"track": QUESTLINE, "minimum": 21, "maximum": 44, "set": 21, "preserve": true}])
    } else if mast {
        json!([
            {"track": QUESTLINE, "minimum": 6, "maximum": 6, "set": 7},
            {"track": MISSION, "minimum": 2, "maximum": 2, "set": 3}
        ])
    } else {
        json!([
            {"track": QUESTLINE, "minimum": 6, "maximum": 6, "set": 6, "preserve": true},
            {"track": MISSION, "minimum": 1, "maximum": 2, "set": 2}
        ])
    };
    let effects = if mast {
        json!({
            "optional_carried_inputs": [{"item": "oteryn:item.tibia.i7243", "count": 1}],
            "magic": {"position": MAST_TARGET, "effect": 10}
        })
    } else {
        json!({})
    };
    Ok(serde_json::from_value(json!({
        "key": format!("oteryn:quest-callback/qcx60/{name}"),
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "appearance": appearance,
            "native": { "position": position, "ordinal": ordinal, "appearance": appearance }
        },
        "reversible": false,
        "requires_world_admission": true,
        "passages": [],
        "storage": storage,
        "item_on_target": {
            "item": item,
            "count": 1,
            "replacement": replacement,
            "reusable": retaliation || mast,
            "selected_instance": true
        },
        "offerings": [],
        "effects": effects,
        "return_on_failure": false,
        "sources": [sources]
    }))?)
}

pub(super) fn catalogue(callback: &Callback, name: &str) -> TestResult<QuestStateCatalogue> {
    let (source, digest, initial) = if name == RETALIATION_NAME {
        (
            CHARM,
            "5c82f124d02cdb7d2db6a971b586f68cdfacd008b406e02a3409943475982316",
            39,
        )
    } else if name == CURE_NAME {
        (
            CURE,
            "29edaa2a436dd2a59444df3bacc77205f8e5804a0dc4b46a29c3e96f3c63618f",
            21,
        )
    } else {
        (
            if matches!(name, MAST_WITH_JUG | MAST_WITHOUT_JUG) {
                RELEASE
            } else {
                SOURCE
            },
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
                {"key": MISSION, "initial": if matches!(name, MAST_WITH_JUG | MAST_WITHOUT_JUG) { 2 } else { 1 }, "minimum": 1, "maximum": 3},
                {"key": OBELISK, "initial": 0, "minimum": 0, "maximum": 5},
                {"key": RETALIATION, "initial": 1, "minimum": 0, "maximum": i32::MAX}
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
    name: &str,
    callback: &Callback,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let retaliation = name == RETALIATION_NAME;
    let cure = name == CURE_NAME;
    let mast = matches!(name, MAST_WITH_JUG | MAST_WITHOUT_JUG);
    let native = callback.selector.native.ok_or("native placement")?;
    let (tool, wrong_tool, transformed) = if retaliation {
        (7289, 7243, 7289)
    } else if cure {
        (7286, 7243, 7246)
    } else if mast {
        (7244, 7286, 7244)
    } else {
        (7243, 7286, 7244)
    };
    let item_key = |id| format!("oteryn:item.tibia.i{id}");
    let inputs = [(951, tool), (952, tool), (953, wrong_tool)];
    for (nonce, id) in inputs
        .into_iter()
        .chain((name == MAST_WITH_JUG).then_some((954, 7243)))
    {
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: native.position,
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
    let at = native.position;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, native.ordinal)
        .ok_or("carried-tool placement")?;
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
    let (next, wrong, _) = super::item_craft_fixtures::exchange(
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
    let (next, accepted, _) = super::item_craft_fixtures::exchange_selected(
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
    if retaliation {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let (next, repeated, _) = super::item_craft_fixtures::exchange_selected(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(item(&item_key(tool))?),
            1,
        )
        .await?;
        *admitted = next;
        assert_ne!(repeated, UseDisposition::Committed);
    }

    let mut expected = [item_key(tool), item_key(transformed), item_key(wrong_tool)];
    expected.sort_unstable();
    let mut definitions = expected.to_vec();
    if mast {
        definitions.push(item_key(7243));
    }
    let inventory: Vec<String> = sqlx::query_scalar(
        "SELECT definition_production_key FROM game_item_instances WHERE lifecycle=1 \
         AND definition_production_key=ANY($1::text[]) ORDER BY definition_production_key",
    )
    .bind(definitions.as_slice())
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(inventory, expected);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    if retaliation {
        assert_eq!(state.tracks().get(OBELISK), Some(&5));
        assert_eq!(state.tracks().get(RETALIATION), Some(&2));
    } else if cure {
        assert_eq!(state.tracks().get(QUESTLINE), Some(&21));
        assert!(!state.tracks().contains_key(MISSION));
    } else if mast {
        assert_eq!(state.tracks().get(QUESTLINE), Some(&7));
        assert_eq!(state.tracks().get(MISSION), Some(&3));
    } else {
        assert_eq!(state.tracks().get(MISSION), Some(&2));
    }
    Ok(())
}
