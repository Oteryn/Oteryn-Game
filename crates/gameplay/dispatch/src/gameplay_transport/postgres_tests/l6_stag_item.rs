//! Stag's source reward has neutral armour and an ordinary ammo-slot move event.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const KEY: &str = "oteryn:item.tibia.i52824";
const TILE: Position = Position { x: 7, y: 1, floor: -7 };

#[test]
fn l6_stag_source_sword_has_native_custody_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() { return Ok(()); }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6-stag-item-profile");
    super::kill_reward_e2e::scenario_thread::run("l6-stag-item-profile", ||
        super::quest_world_callbacks::scenario("l6-stag-item-profile", TILE, false))?;
    eprintln!("POSTGRES_CASE_COMPLETED l6-stag-item-profile duration={:?}", started.elapsed());
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()").fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY l6-stag-item-profile {version}");
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
    let policy = native.item_policy_for_source_id(52824).ok_or("source sword52824 native policy missing")?;
    let semantics = serde_json::to_value(&policy.record().semantics)?;
    assert_eq!(semantics["protection"]["value"]["armor"], serde_json::json!({"state":"KNOWN","value":0}));
    assert_eq!(semantics["equipment"]["value"]["patterns"]["value"][0]["primary_slot"]["value"], "EXTRA");
    let qualified = crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert_eq!(qualified.definition.production_key, KEY);
    assert!(qualified.materializable && qualified.movable && qualified.inventory_destination && qualified.ground_destination);
    assert_eq!(qualified.stack_maximum, 1);
    assert!(qualified.decay.is_none() && qualified.container_capacity.is_none());
    admitted.continuity.selected_capabilities = crate::gameplay_transport::capabilities::SelectedCapabilities::select(
        owner.offered_capabilities(), &[4,6,11,12,14,15,18,oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1],
    ).ok_or("custody capabilities")?;
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering { position:TILE,item:KEY.into(),count:1 },60,true).await?;
    let before: String = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(KEY).fetch_one(&harness.pool).await?;
    let reference = native.item_index().and_then(|i| i.definition_ref("Item",KEY,"definition-r1")).ok_or("sword reference")?;
    let destination = oteryn_protocol_oteryn::item_view::WorldTilePosition {x:i32::from(TILE.x),y:i32::from(TILE.y),floor:i16::from(TILE.floor)};
    let (next,outcome,payload) = super::map_seed_planting::drop_exchange_at(owner,*admitted,1,None,reference,1,false,destination).await?;
    *admitted=next;
    assert_eq!(outcome,Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved));
    let (next,replay,_) = super::map_seed_planting::drop_exchange_at(owner,*admitted,1,Some(payload),reference,1,false,destination).await?;
    *admitted=next;
    assert_eq!(replay,outcome);
    let actual: Vec<(String,i64)> = sqlx::query_as("SELECT i.item_instance_id::text,i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1")
        .bind(KEY).fetch_all(&harness.pool).await?;
    assert_eq!(actual,vec![(before,1)]);
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=1").fetch_one(&harness.pool).await?;
    assert_eq!(receipts,1);
    Ok(())
}
