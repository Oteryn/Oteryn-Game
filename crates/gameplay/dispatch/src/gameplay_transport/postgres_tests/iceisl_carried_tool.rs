//! Crystal actions_charm.lua:3–22 through framed USE and the fenced World writer.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::{Bindings, BoundQuest, Stage, Track, Trigger},
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const NAME: &str = "iceisl-carried-tool";
const TARGET: Position = Position {
    x: 32138,
    y: 31113,
    floor: -14,
};
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const TOOL: &str = "oteryn:item.tibia.i7289";
const MARK: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/obelisk01";
const STAGE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_charm.lua:3-22";

#[test]
fn iceisl_frost_charm_use_proves_selected_carried_custody() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/ice-islands/frost-charm-obelisk-1",
        "quest": QUEST,
        "trigger": "use",
        "selector": {"native":{"position":TARGET,"ordinal":2,"appearance":1841}, "appearance":1841},
        "lever": null, "reversible": false, "passages": [], "teleport": null,
        "requires_world_admission": true,
        "storage": [
            {"track": STAGE, "minimum":39, "maximum":39, "set":39, "preserve":true},
            {"track": MARK, "minimum":-1, "maximum":4, "set":5}
        ],
        "offerings": [],
        "item_on_target": {"item":TOOL, "count":1, "reusable":true, "selected_instance":true},
        "return_on_failure": false, "sources": [SOURCE]
    }))?)
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    native_scene(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z14-x125-y121.b3"
            )),
            region: (14, 125, 121),
            bounds: (32136, 31112, 32139, 31114),
            start: (32137, 31113),
            expected: vec![ExpectedTile {
                position: (TARGET.x, TARGET.y),
                ids: vec![7062, 6373, 1841],
                counted_entry: None,
            }],
        }],
        &[],
    )
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    catalogue_with_tracks(
        callback,
        "retaliation",
        vec![
            Track {
                key: MARK.into(),
                initial: -1,
                minimum: -1,
                maximum: 5,
            },
            Track {
                key: STAGE.into(),
                initial: 39,
                minimum: -1,
                maximum: 40,
            },
        ],
    )
}

pub(super) fn catalogue_with_tracks(
    callback: &Callback,
    stage: &str,
    tracks: Vec<Track>,
) -> TestResult<QuestStateCatalogue> {
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture base catalogue")?;
    Ok(Bindings {
        schema: 1,
        quests: vec![BoundQuest {
            quest: QUEST.into(),
            recipe_sha256: "a0862a3bb15e6de9707e77425cd947b8b8eb8fada25e3dd14228451a092982ca"
                .into(),
            occurrence: None,
            admission: None,
            tracks,
            stages: vec![Stage {
                key: stage.into(),
                completes: false,
                trigger: Trigger::MapUse {
                    callback: Box::new(callback.clone()),
                },
            }],
            sources: callback.sources.clone(),
        }],
    }
    .attach(&base)?)
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
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("item index")?;
    let definition = index
        .definition_ref("Item", TOOL, "definition-r1")
        .ok_or("frost charm")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(TARGET.floor, TARGET.x, TARGET.y, 2)
        .ok_or("obelisk placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, without_charm, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        None,
    )
    .await?;
    *admitted = next;
    assert_ne!(without_charm, UseDisposition::Committed);

    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TARGET,
            item: TOOL.into(),
            count: 1,
        },
        961,
        true,
    )
    .await?;
    let expected_tool: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
    ).bind(TOOL).fetch_one(&h.pool).await?;
    let (next, marked, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        marked,
        UseDisposition::Committed,
        "registered carried charm marks the obelisk"
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(MARK), Some(&5));
    let proof: (Vec<u8>, i16) = sqlx::query_as(
        "SELECT uuid_send(t.item_instance_id), i.lifecycle
         FROM game_quest_world_tool_reads t JOIN game_item_instances i USING (item_instance_id)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(proof, (expected_tool, 1));
    let changed_tracks: Vec<String> = sqlx::query_scalar(
        "SELECT track_keys FROM game_character_quest_receipts WHERE quest_key=$1",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(changed_tracks, [MARK], "questline was a guard, not a write");
    Ok(())
}
