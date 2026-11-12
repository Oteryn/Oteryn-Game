//! Reusable tools must keep their physical identity when a quest action commits.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{input_item, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const TRIGGER: Position = Position {
    x: 32700,
    y: 31200,
    floor: -7,
};
pub(super) const TRACK: &str =
    "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/sewer_pipe03";

#[test]
fn l6_reusable_tool_preserves_instance_and_refuses_missing_tool() -> TestResult {
    run_case("l6-reusable-tool")
}

#[test]
fn l6_reusable_multitool_preserves_instance_and_refuses_missing_tool() -> TestResult {
    run_case("l6-reusable-tool-multitool")
}

fn run_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, TRIGGER, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "reusable tool scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn fixture(mut callback: Callback) -> Callback {
    callback.key = "oteryn:quest-callback/l6-reusable-tool-fixture".into();
    callback.quest = "oteryn:quest.in_service_of_yalahar_quest".into();
    callback.selector.position = None;
    callback.selector.unique = Some(3073);
    callback.selector.action = None;
    callback.selector.appearance = None;
    callback.teleport = None;
    callback.storage.clear();
    callback.item_on_target = None;
    callback
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    tool: &str,
) -> TestResult {
    eprintln!("POSTGRES_DATABASE_READY l6_reusable_tool");
    let mission = "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/mission01";
    let initial = owner
        .quest_catalogue
        .as_ref()
        .ok_or("catalogue")?
        .track(mission)
        .ok_or("mission track")?
        .initial;
    // The accepted runtime projection starts Mission01 at zero (quest-state.json),
    // so this action must earn one and replay must never increment it again.
    assert_eq!(initial, 0);
    let before_state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert!(!before_state.tracks().contains_key(mission));
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(TRIGGER.floor, TRIGGER.x, TRIGGER.y, 1)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let refused = wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
        UseDisposition::Rejected
    );
    input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: TRIGGER,
            item: tool.into(),
            count: 1,
        },
        60,
        true,
    )
    .await?;
    let snapshot = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(i) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 LIMIT 1"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(tool).fetch_one(&harness.pool).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let committed = wire(owner, admitted, 2, 2, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&committed).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        wire(owner, admitted, 2, 2, payload.clone()).await?,
        committed
    );
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let stale = wire(owner, admitted, 3, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&stale).map_err(debug)?,
        UseDisposition::StaleState
    );
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let completed_payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 1,
        })
        .map_err(debug)?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let completed = wire(owner, admitted, 4, 2, completed_payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&completed).map_err(debug)?,
        UseDisposition::Rejected
    );
    let after = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(i) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3 LIMIT 1"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(tool).fetch_one(&harness.pool).await?;
    assert_eq!(after, snapshot);
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(TRACK), Some(&1));
    assert_eq!(state.tracks().get(mission), Some(&1));
    let consumed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(consumed, 0);
    Ok(())
}
