//! Source talisman custody; assembly and live-target cleansing are separate mechanics.
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
fn questpc3_tainted_talismans_have_native_custody_and_exact_drop_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("QUESTPC3_TAINTED_ITEMS POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc3-tainted-items", || {
        super::quest_world_callbacks::scenario("questpc3-tainted-items", TILE, false)
    });
    eprintln!(
        "QUESTPC3_TAINTED_ITEMS POSTGRES_CASE_COMPLETED duration_ms={} success={}",
        started.elapsed().as_millis(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&harness.pool)
        .await?;
    eprintln!("QUESTPC3_TAINTED_ITEMS POSTGRES_CASE_STARTED server_version={version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
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
    for (source, count, maximum, mint_ordinal, command) in
        [(22008, 17, 100, 60, 1), (22009, 1, 1, 61, 2)]
    {
        let key = format!("oteryn:item.tibia.i{source}");
        let policy = native
            .item_policy_for_source_id(source)
            .ok_or("source talisman native policy missing")?;
        let qualified =
            crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(
                policy,
            )?;
        assert_eq!(qualified.definition.production_key, key);
        assert!(
            qualified.materializable
                && qualified.movable
                && qualified.inventory_destination
                && qualified.ground_destination
        );
        assert_eq!(qualified.stack_maximum, maximum);
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: TILE,
                item: key.clone(),
                count,
            },
            mint_ordinal,
            true,
        )
        .await?;
        let before: String = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(&key).fetch_one(&harness.pool).await?;
        let reference = native
            .item_index()
            .and_then(|i| i.definition_ref("Item", &key, "definition-r1"))
            .ok_or("talisman reference")?;
        let destination = oteryn_protocol_oteryn::item_view::WorldTilePosition {
            x: i32::from(TILE.x),
            y: i32::from(TILE.y),
            floor: i16::from(TILE.floor),
        };
        let (next, outcome, payload) = super::map_seed_planting::drop_exchange_at(
            owner,
            *admitted,
            command,
            None,
            reference,
            u8::try_from(count)?,
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
            command,
            Some(payload),
            reference,
            u8::try_from(count)?,
            false,
            destination,
        )
        .await?;
        *admitted = next;
        assert_eq!(replay, outcome);
        let actual: Vec<(String,i64)> = sqlx::query_as("SELECT i.item_instance_id::text,i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1")
            .bind(&key).fetch_all(&harness.pool).await?;
        assert_eq!(actual, vec![(before, i64::from(count))]);
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=$1",
        )
        .bind(i64::try_from(command)?)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(receipts, 1);
    }
    Ok(())
}
