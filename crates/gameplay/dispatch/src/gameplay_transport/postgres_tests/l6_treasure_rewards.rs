//! Poem and telescope physical custody for the owned, selected Treasure Hunt route.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Offering;

const TILE: oteryn_gameplay_quests::quest::world_callbacks::Position =
    oteryn_gameplay_quests::quest::world_callbacks::Position {
        x: 7,
        y: 1,
        floor: -7,
    };

#[test]
fn l6_treasure_reward_9204_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9204")
}

#[test]
fn l6_treasure_reward_9616_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9616")
}

#[test]
fn l6_treasure_reward_9617_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9617")
}

#[test]
fn l6_treasure_reward_9618_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9618")
}

#[test]
fn l6_treasure_reward_9619_native_custody_and_replay() -> TestResult {
    run("l6-treasure-reward-9619")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Treasure reward custody requires PostgreSQL"
        );
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, TILE, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn source_id(name: &str) -> Option<u32> {
    match name {
        "l6-treasure-reward-9204" => Some(9204),
        "l6-treasure-reward-9616" => Some(9616),
        "l6-treasure-reward-9617" => Some(9617),
        "l6-treasure-reward-9618" => Some(9618),
        "l6-treasure-reward-9619" => Some(9619),
        _ => None,
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    source_id: u32,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_treasure_rewards {version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let policy = native
        .item_policy_for_source_id(source_id)
        .ok_or("treasure reward policy")?;
    let semantics = serde_json::to_value(&policy.record().semantics)?;
    if source_id == 9204 {
        let text = &semantics["readable_writeable"]["value"];
        assert_eq!(
            text["readable"],
            serde_json::json!({"state":"KNOWN","value":true})
        );
        for key in [
            "writeable",
            "distance_read",
            "max_text_length",
            "write_once_target",
        ] {
            assert_eq!(text[key]["state"], "UNKNOWN");
        }
    } else {
        let expected = if source_id < 9619 {
            source_id + 1
        } else {
            9616
        };
        let targets = semantics["use_transform"]["value"]["targets"]
            .as_array()
            .ok_or("transform targets")?;
        let rotation = targets
            .iter()
            .find(|t| t["kind"] == "ROTATE")
            .ok_or("rotation target")?;
        assert_eq!(
            rotation["target"],
            serde_json::json!({"state":"KNOWN","value":{"key":format!("oteryn:item.tibia.i{expected}"),"revision":"definition-r1"}})
        );
        assert!(
            targets
                .iter()
                .filter(|t| t["kind"] != "ROTATE")
                .all(|t| t["target"]["state"] == "UNKNOWN")
        );
    }
    let key = format!("oteryn:item.tibia.i{source_id}");
    let qualified =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert_eq!(qualified.definition.production_key, key.as_str());
    assert!(
        qualified.materializable
            && qualified.movable
            && qualified.inventory_destination
            && qualified.ground_destination
    );
    assert_eq!(qualified.stack_maximum, 1);
    assert!(qualified.decay.is_none() && qualified.container_capacity.is_none());
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                11,
                12,
                14,
                15,
                18,
                oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
            ],
        )
        .ok_or("custody capabilities")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TILE,
            item: key.as_str().into(),
            count: 1,
        },
        60,
        true,
    )
    .await?;
    let before: String = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(key.as_str()).fetch_one(&harness.pool).await?;
    let reference = native
        .item_index()
        .and_then(|i| i.definition_ref("Item", key.as_str(), "definition-r1"))
        .ok_or("reward reference")?;
    let destination = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: i32::from(TILE.x),
        y: i32::from(TILE.y),
        floor: i16::from(TILE.floor),
    };
    let (next, outcome, payload) = super::map_seed_planting::drop_exchange_at(
        owner,
        *admitted,
        1,
        None,
        reference,
        1,
        false,
        destination,
    )
    .await?;
    *admitted = next;
    assert_eq!(
        outcome,
        Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved)
    );
    let (next, replay, _) = super::map_seed_planting::drop_exchange_at(
        owner,
        *admitted,
        1,
        Some(payload),
        reference,
        1,
        false,
        destination,
    )
    .await?;
    *admitted = next;
    assert_eq!(replay, outcome);
    let actual: Vec<(String,i64)> = sqlx::query_as("SELECT i.item_instance_id::text,i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1")
        .bind(key.as_str()).fetch_all(&harness.pool).await?;
    assert_eq!(actual, vec![(before, 1)]);
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=1")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(receipts, 1);
    Ok(())
}
