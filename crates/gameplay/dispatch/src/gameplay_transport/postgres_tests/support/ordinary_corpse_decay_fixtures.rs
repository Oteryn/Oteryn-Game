use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

pub(super) const MINT_SCENARIO: &str = "qengine-rewards-ordinary-orc-corpse-decay";

pub(super) async fn exercise_mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.orc",
        60000,
    )
    .await?;
    let mints: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item_instance_id::text, game_corpse_retirement_deadline(item_instance_id)-materialized_at
         FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'
         AND draw_ordinal=0 AND loot_table_production_key='oteryn:item.tibia.i5966'",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(mints.len(), 1);
    assert_eq!(mints[0].1, 670_000);
    let stages: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT stage.definition_production_key, stage.target_production_key,
                stage.expires_at_unix_ms-mint.materialized_at
         FROM game_spell_field_temporal_schedules stage JOIN game_item_mint_receipts mint USING(item_instance_id)
         WHERE stage.item_instance_id=$1::uuid AND stage.source_transaction_id=mint.transaction_id
           AND stage.source_ordinal=0
         ORDER BY stage.source_stage",
    )
    .bind(&mints[0].0)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        stages
            .iter()
            .map(|(stage, target, time)| (stage.as_str(), target.as_str(), *time))
            .collect::<Vec<_>>(),
        [
            ("oteryn:item.tibia.i5966", "oteryn:item.tibia.i4001", 10_000),
            (
                "oteryn:item.tibia.i4001",
                "oteryn:item.tibia.i4002",
                310_000
            ),
            (
                "oteryn:item.tibia.i4002",
                "oteryn:item.tibia.i4317",
                610_000
            ),
        ],
    );
    Ok(())
}
