use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) const NAME: &str = "qengine-rewards-ordinary-corpse-slice2";

#[derive(serde::Deserialize)]
struct SourceChain {
    chain: Vec<i32>,
    durations_ms: Vec<i64>,
}

#[derive(serde::Deserialize)]
struct StagePhysicalFacts {
    pickupable: bool,
}

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
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemStackClass;
    use oteryn_gameplay_items::item_ref_admission::{
        generation_ground_item_facts, generation_item_facts,
    };

    let stages: Vec<(u32, StagePhysicalFacts)> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/ordinary-corpses/movable-stages.json"
    )))?;
    for (id, captured) in stages {
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "definition-r1".into(),
        };
        let facts = generation_ground_item_facts(native, &definition).map_err(debug)?;
        assert_eq!(
            facts.stack,
            ItemStackClass::NonStackable,
            "corpse stage {id}"
        );
        if captured.pickupable {
            let facts = generation_item_facts(native, &definition).map_err(debug)?;
            assert_eq!(
                facts.stack,
                ItemStackClass::NonStackable,
                "corpse stage {id}"
            );
        }
    }
    super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.energy_elemental",
        60000,
    )
    .await?;
    let (corpse, materialized, lifetime): (String, i64, i64) = sqlx::query_as(
        "SELECT item_instance_id::text,materialized_at,
                game_corpse_retirement_deadline(item_instance_id)-materialized_at
         FROM game_item_mint_receipts
         WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0
           AND loot_table_production_key='oteryn:item.tibia.i8138'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifetime, 510_000);
    let stages: Vec<(i32, String, String, i64, i64)> = sqlx::query_as(
        "SELECT source_stage,definition_production_key,target_production_key,
                duration_millis,expires_at_unix_ms
         FROM game_spell_field_temporal_schedules
         WHERE item_instance_id=$1::uuid AND source_ordinal=0
         ORDER BY source_stage",
    )
    .bind(&corpse)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        stages,
        [(
            1,
            "oteryn:item.tibia.i8138".to_owned(),
            "oteryn:item.tibia.i8139".to_owned(),
            10_000,
            materialized + 10_000
        )]
    );

    super::qengine_kill_stories_fixtures::death_actor(
        owner,
        admitted,
        "oteryn:creature.insect_swarm",
        60001,
    )
    .await?;
    let plain_body: (bool, i64) = sqlx::query_as(
        "SELECT game_item_plain_corpse_source(item_instance_id),
                game_corpse_retirement_deadline(item_instance_id)-materialized_at
         FROM game_item_mint_receipts
         WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND draw_ordinal=0
           AND loot_table_production_key='oteryn:item.tibia.i10403'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(plain_body, (true, 3_000));

    let chains: Vec<SourceChain> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/ordinary-corpse-decay-slice2.json"
    )))?;
    assert_eq!(chains.len(), 255);
    for source in chains {
        let id = source.chain.first().ok_or("source chain root")?;
        let key = format!("oteryn:item.tibia.i{id}");
        let (chain, durations): (Vec<i32>, Vec<i64>) = sqlx::query_as(
            "SELECT game_summer_corpse_chain($1),game_source_corpse_durations_v1($1)",
        )
        .bind(&key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(chain, source.chain, "{key}");
        assert_eq!(durations, source.durations_ms, "{key}");
    }
    Ok(())
}
