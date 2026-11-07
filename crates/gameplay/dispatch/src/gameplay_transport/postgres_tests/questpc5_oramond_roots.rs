//! Physical custody for the exact source root; daily route qualification is separate.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

#[test]
fn questpc5_oramond_five_exact_juicy_roots_enter_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-oramond-roots")
}

#[test]
fn questpc5_cook_fryclops_axe_exact_item_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-axe")
}
#[test]
fn questpc5_cook_ratha_empty_exact_item_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-empty")
}
#[test]
fn questpc5_cook_ratha_full_exact_item_enters_native_backpack() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-cook-full")
}

pub(super) async fn exercise_cook(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let item = match name {
        "qengine-rewards-cook-axe" => 44525,
        "qengine-rewards-cook-empty" => 44527,
        "qengine-rewards-cook-full" => 44528,
        _ => return Err("unknown cook material case".into()),
    };
    exercise_item(owner, admitted, h, item, 1, "cook_material_custody").await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    exercise_item(owner, admitted, h, 21291, 5, "oramond_root_custody").await
}

async fn exercise_item(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    item: u32,
    quantity: u16,
    marker: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_CASE_STARTED {marker} item={item} server_version={version}");
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: quantity,
        },
        91,
        true,
    )
    .await?;
    let amount: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$2 AND i.lifecycle=1 AND e.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
    assert_eq!(
        amount,
        i64::from(quantity),
        "exact source quantity survives native backpack custody"
    );
    let quests: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        quests, 0,
        "physical item admission does not complete a quest"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED {marker} item={item} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
