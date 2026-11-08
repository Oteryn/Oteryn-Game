//! Stag's source reward has neutral armour and an ordinary ammo-slot move event.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const TILE: Position = Position {
    x: 7,
    y: 1,
    floor: -7,
};

#[test]
fn l6_stag_source_sword_has_native_custody_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6-stag-item-profile");
    super::kill_reward_e2e::scenario_thread::run("l6-stag-item-profile", || {
        super::quest_world_callbacks::scenario("l6-stag-item-profile", TILE, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6-stag-item-profile duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn l6_stag_source_compass_has_native_custody_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6-stag-compass-profile");
    super::kill_reward_e2e::scenario_thread::run("l6-stag-compass-profile", || {
        super::quest_world_callbacks::scenario("l6-stag-compass-profile", TILE, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6-stag-compass-profile duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn l6_stag_material_certificate_native_custody_and_replay() -> TestResult {
    run_material_case("l6-stag-certificate-profile")
}

#[test]
fn l6_stag_material_iron_native_custody_and_replay() -> TestResult {
    run_material_case("l6-stag-iron-profile")
}

#[test]
fn l6_stag_material_silver_native_custody_and_replay() -> TestResult {
    run_material_case("l6-stag-silver-profile")
}

#[test]
fn l6_stag_material_bronze_native_custody_and_replay() -> TestResult {
    run_material_case("l6-stag-bronze-profile")
}

fn run_material_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Stag material case requires PostgreSQL"
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

pub(super) fn material_source_id(name: &str) -> Option<u32> {
    match name {
        "l6-stag-certificate-profile" => Some(52586),
        "l6-stag-iron-profile" => Some(52635),
        "l6-stag-silver-profile" => Some(52743),
        "l6-stag-bronze-profile" => Some(52758),
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
    assert!(matches!(
        source_id,
        52586 | 52635 | 52743 | 52758 | 52759 | 52824
    ));
    let maximum = if matches!(source_id, 52635 | 52743 | 52758) {
        100
    } else {
        1
    };
    exercise_custody(
        owner,
        admitted,
        harness,
        fence,
        source_id,
        matches!(source_id, 52759 | 52824),
        maximum,
    )
    .await
}

pub(super) async fn exercise_plain(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    source_id: u32,
) -> TestResult {
    exercise_custody(owner, admitted, harness, fence, source_id, false, 1).await
}

pub(super) async fn exercise_legs(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    exercise_custody(owner, admitted, harness, fence, 3398, false, 1).await
}

pub(super) async fn exercise_typed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    source_id: u32,
) -> TestResult {
    assert!(matches!(source_id, 5949 | 24382 | 21506 | 7785 | 52632));
    let maximum = if source_id == 5949 { 1 } else { 100 };
    exercise_custody(owner, admitted, harness, fence, source_id, false, maximum).await
}

pub(super) async fn exercise_special(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    source_id: u32,
) -> TestResult {
    assert!(matches!(source_id, 8862 | 21389 | 5552 | 9171 | 21572));
    exercise_custody(owner, admitted, harness, fence, source_id, false, 1).await
}

async fn exercise_custody(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    source_id: u32,
    neutral_equipment: bool,
    expected_stack_maximum: u32,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6-stag-item-profile {version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let key = format!("oteryn:item.tibia.i{source_id}");
    let policy = native
        .item_policy_for_source_id(source_id)
        .ok_or("Stag source item native policy missing")?;
    let semantics = serde_json::to_value(&policy.record().semantics)?;
    if neutral_equipment {
        assert_eq!(
            semantics["protection"]["value"]["armor"],
            serde_json::json!({"state":"KNOWN","value":0})
        );
        assert_eq!(
            semantics["equipment"]["value"]["patterns"]["value"][0]["primary_slot"]["value"],
            "EXTRA"
        );
    } else if source_id == 8862 {
        assert_eq!(
            semantics["protection"]["value"]["armor"],
            serde_json::json!({"state":"KNOWN","value":16})
        );
        assert_eq!(
            semantics["protection"]["value"]["resistances"]["value"][0],
            serde_json::json!({"kind":"DEATH","percent":{"state":"KNOWN","value":{"numerator":3,"denominator":1}}})
        );
        let pattern = &semantics["equipment"]["value"]["patterns"]["value"][0];
        assert_eq!(pattern["primary_slot"]["value"], "TORSO");
        assert_eq!(pattern["level"]["value"], 80);
        assert_eq!(pattern["vocations"]["value"], serde_json::json!(["KNIGHT"]));
    } else if source_id == 5949 {
        assert_eq!(semantics["protection"]["state"], "UNKNOWN");
        assert_eq!(
            semantics["container"]["value"]["capacity"],
            serde_json::json!({"state":"KNOWN","value":20})
        );
        assert_eq!(
            semantics["equipment"]["value"]["patterns"]["value"][0]["primary_slot"]["value"],
            "CONTAINER"
        );
    } else if source_id == 3398 {
        assert_eq!(
            semantics["protection"],
            serde_json::json!({
                "state":"KNOWN", "value": {
                    "armor":{"state":"KNOWN","value":7},
                    "resistances":{"state":"KNOWN","value":[{
                        "kind":"PHYSICAL", "percent":{"state":"KNOWN","value":{"denominator":1,"numerator":3}}
                    }]}
                }
            })
        );
        assert_eq!(
            semantics["equipment"]["value"]["patterns"]["value"][0]["primary_slot"]["value"],
            "LEGS"
        );
    } else {
        assert_eq!(semantics["protection"]["state"], "UNKNOWN");
        assert_eq!(semantics["equipment"]["state"], "NOT_APPLICABLE");
    }
    let qualified =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert_eq!(qualified.definition.production_key, key.as_str());
    if source_id == 21572 {
        assert!(qualified.materializable && qualified.ground_destination);
        assert!(!qualified.movable && !qualified.inventory_destination);
        assert_eq!(qualified.stack_maximum, 1);
        assert_eq!(
            semantics["physical"]["value"]["weight"],
            serde_json::json!({"state":"KNOWN","value":0})
        );
        let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
            native,
            &qualified.definition,
        )?;
        assert!(matches!(
            facts.stack,
            oteryn_gameplay_domain::domain::item_definition::ItemStackClass::Unknown
        ));
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: TILE,
                item: key.clone(),
                count: 1,
            },
            60,
            false,
        )
        .await?;
        let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND i.quantity=1").bind(key.as_str()).fetch_one(&harness.pool).await?;
        assert_eq!(ground, 1);
        return Ok(());
    }
    assert!(
        qualified.materializable
            && qualified.movable
            && qualified.inventory_destination
            && qualified.ground_destination
    );
    assert_eq!(qualified.stack_maximum, expected_stack_maximum);
    if source_id == 21389 {
        let decay = qualified
            .decay
            .as_ref()
            .ok_or("flower source decay missing")?;
        assert_eq!(decay.duration_millis, 3_600_000);
        assert!(decay.target.is_none());
    } else {
        assert!(qualified.decay.is_none());
    }
    if source_id == 5552 {
        assert_eq!(
            semantics["fluid"],
            serde_json::json!({"state":"KNOWN","value":{"fluid_type":{"state":"NOT_APPLICABLE"}}})
        );
    }
    if source_id == 9171 {
        assert_eq!(
            semantics["readable_writeable"]["value"]["max_text_length"],
            serde_json::json!({"state":"KNOWN","value":1023})
        );
        assert_eq!(
            semantics["readable_writeable"]["value"]["readable"]["state"],
            "UNKNOWN"
        );
    }
    if source_id == 21572 {
        assert_eq!(
            semantics["physical"]["value"]["weight"],
            serde_json::json!({"state":"KNOWN","value":0})
        );
    }
    assert_eq!(
        qualified.container_capacity,
        if source_id == 5949 { Some(20) } else { None }
    );
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
            item: key.clone(),
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
        .ok_or("Stag item reference")?;
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
