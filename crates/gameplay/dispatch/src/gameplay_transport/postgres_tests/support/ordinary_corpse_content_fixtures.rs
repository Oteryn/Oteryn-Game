use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) const NAME: &str = "qengine-rewards-ordinary-corpse-content";

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native policies")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.acid_blob",
        60_000,
    )
    .await?;
    let (lifetime, stages): (i64, Vec<String>) = sqlx::query_as(
        "SELECT game_corpse_retirement_deadline(m.item_instance_id)-m.materialized_at,
         ARRAY(SELECT target_production_key FROM game_spell_field_temporal_schedules
               WHERE item_instance_id=m.item_instance_id ORDER BY source_stage)
         FROM game_item_mint_receipts m WHERE loot_purpose_key='CORPSE_MATERIALIZATION'
         AND draw_ordinal=0 AND loot_table_production_key='oteryn:item.tibia.i9049'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        (lifetime, stages),
        (910_000, vec!["oteryn:item.tibia.i9052".to_owned()])
    );
    super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.blue_butterfly",
        60_001,
    )
    .await?;
    let plain_body: bool = sqlx::query_scalar(
        "SELECT game_item_plain_corpse_source(item_instance_id)
         FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'
         AND draw_ordinal=0 AND loot_table_production_key='oteryn:item.tibia.i4993'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert!(
        plain_body,
        "the blue butterfly remains ordinary Ground custody"
    );

    let policies: Vec<(Vec<i32>, Vec<i64>)> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/ordinary-corpses/decay.json"
    )))?;
    assert_eq!(policies.len(), 262);
    for (chain, durations) in policies {
        let root = chain.first().ok_or("empty source corpse chain")?;
        let key = format!("oteryn:item.tibia.i{root}");
        for stage in &chain {
            let stage_key = format!("oteryn:item.tibia.i{stage}");
            assert!(
                native.item_policy(&stage_key, "definition-r1").is_some(),
                "native corpse stage {stage_key}"
            );
        }
        let actual: (Vec<i32>, Vec<i64>) = sqlx::query_as(
            "SELECT game_summer_corpse_chain($1), game_source_corpse_durations_v1($1)",
        )
        .bind(&key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(actual, (chain, durations), "{key}");
    }
    Ok(())
}
