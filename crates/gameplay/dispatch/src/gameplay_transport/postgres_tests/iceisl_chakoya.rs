//! Crystal register_actions.lua:721–753 through framed pick USE.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Track,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const NAME: &str = "iceisl-chakoya";
const TARGET: Position = Position {
    x: 32399,
    y: 31051,
    floor: -7,
};
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const TOOL: &str = "oteryn:item.tibia.i3456";
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission02";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/register_actions.lua:697-732";
const CREATURES: [&str; 3] = [
    "oteryn:creature.chakoya_toolshaper",
    "oteryn:creature.chakoya_tribewarden",
    "oteryn:creature.chakoya_windcaller",
];

#[test]
fn iceisl_pick_spawns_chakoyas_once_and_resets_only_the_crack() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/ice-islands/breaking-the-ice-north",
        "quest": QUEST,
        "trigger": "use",
        "selector": {"native":{"position":TARGET,"ordinal":1,"appearance":7185},"appearance":7185},
        "lever": null, "reversible": false, "passages": [], "teleport": null,
        "requires_world_admission": true,
        "storage": [{"track":MISSION,"minimum":1,"maximum":2,"set":1,"increment":true}],
        "offerings": [],
        "item_on_target": {"item":TOOL,"count":1,"reusable":true,"selected_instance":true},
        "effects": {
            "transform":7186, "reset_after_ms":60000,
            "magic":{"position":TARGET,"effect":3},
            "encounter": {
                "minimum_level":0,
                "players":[],
                "monsters":[
                    {"position":{"x":32397,"y":31048,"floor":-7},"creature":CREATURES[0],"alternatives":[CREATURES[1],CREATURES[2]]},
                    {"position":{"x":32399,"y":31048,"floor":-7},"creature":CREATURES[0],"alternatives":[CREATURES[1],CREATURES[2]]}
                ],
                "room_min":{"x":32397,"y":31048,"floor":-7},
                "room_max":{"x":32399,"y":31048,"floor":-7}
            }
        },
        "return_on_failure": false, "sources": [SOURCE]
    }))?)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x126-y121.b3"
            )),
            region: (7, 126, 121),
            bounds: (32396, 31047, 32400, 31053),
            start: (32398, 31051),
            expected: vec![ExpectedTile {
                position: (TARGET.x, TARGET.y),
                ids: vec![6686, 7185],
                counted_entry: None,
            }],
        }],
        &[],
        &[7186],
    )
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    super::iceisl_carried_tool::catalogue_with_tracks(
        callback,
        "breaking-the-ice",
        vec![Track {
            key: MISSION.into(),
            initial: 1,
            minimum: -1,
            maximum: 4,
        }],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "quest fence")?
        .ok_or("quest fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TARGET,
            item: TOOL.into(),
            count: 1,
        },
        962,
        true,
    )
    .await?;

    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 1)
        .ok_or("crack placement")?;
    let payload = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<Vec<u8>> {
        let map = owner.bundle_map().ok_or("map")?;
        let definition = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.item_index())
            .and_then(|index| index.definition_ref("Item", TOOL, "definition-r1"))
            .ok_or("pick reference")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(definition),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
            use_with_position: None,
        })
        .map_err(debug)?)
    };

    let command = admitted.continuity.next_command_id;
    let before = payload(owner)?;
    let (next, first, _) =
        super::item_craft::exchange(owner, *admitted, command, &before, None).await?;
    *admitted = next;
    assert_eq!(first, UseDisposition::Committed, "fenced crack USE commits");
    {
        let runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        let mut points = census
            .iter()
            .map(|&actor| {
                let key = runtime.creature_spell_target_atom(actor).map_err(debug)?;
                assert!(CREATURES.contains(&key.as_str()));
                let position = runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position();
                Ok((position.x, position.y, position.floor))
            })
            .collect::<TestResult<Vec<_>>>()?;
        points.sort_unstable();
        assert_eq!(points, [(32397, 31048, 7), (32399, 31048, 7)]);
    }
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(MISSION), Some(&2));
    let (next, replay, _) =
        super::item_craft::exchange(owner, *admitted, command, &before, None).await?;
    *admitted = next;
    assert_eq!(replay, UseDisposition::Committed);

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let (next, busy, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload(owner)?,
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(busy, UseDisposition::NothingToUse);

    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(
        super::engitem_seal_paint::source_appearances(map, TARGET)?,
        [6686, 7186]
    );
    let due = map.facts.next_movement_decay_time().ok_or("crack reset")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(
        super::engitem_seal_paint::source_appearances(map, TARGET)?,
        [6686, 7185]
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?
            .len(),
        2
    );
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipts, 1);
    Ok(())
}
