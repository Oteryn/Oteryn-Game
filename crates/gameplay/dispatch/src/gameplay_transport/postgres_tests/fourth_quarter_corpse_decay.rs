//! Crystal 201189f8 death-body transforms and terminal lifetimes installed in PostgreSQL.
use super::TestResult;
use super::fixture::{Harness, run};
use std::collections::BTreeMap;

#[test]
fn fourth_quarter_corpse_decay_uses_exact_source_registry_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("fourth-quarter-corpse-decay", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "fourth_quarter_corpse_decay", true, false)
                .await?;
            sea_serpent_mint_schedules_source_decay(&h).await?;
            #[derive(serde::Deserialize)]
            struct Source {
                decay_chains: BTreeMap<u32, Vec<(i32, i64)>>,
            }
            let source: Source = serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/native-gameplay/fourth-quarter-corpse-source.json"
            )))?;
            assert_eq!(source.decay_chains.len(), 246);
            for (item, stages) in source.decay_chains {
                let key = format!("oteryn:item.tibia.i{item}");
                let (chain, durations): (Option<Vec<i32>>, Option<Vec<i64>>) = sqlx::query_as(
                    "SELECT game_summer_corpse_chain($1), game_source_corpse_durations_v1($1)",
                )
                .bind(&key)
                .fetch_one(&h.pool)
                .await?;
                let (expected_chain, expected_durations) = stages.into_iter().unzip();
                assert_eq!(chain, Some(expected_chain), "{key} stages");
                assert_eq!(durations, Some(expected_durations), "{key} lifetimes");
            }
            h.cleanup().await
        })
    })
}

async fn sea_serpent_mint_schedules_source_decay(h: &Harness) -> TestResult {
    use super::fixture::{debug, fence, id};
    use super::{Dependencies, prepare_current_session};
    use crate::durability::item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
        ItemMintRequest, TypedDefinitionRef,
    };
    use crate::durability::map_item_mint_audit::map_item_spatial_position;

    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let dependencies = Dependencies::new()?;
    let owner = dependencies.owner(h, &authority);
    dependencies.initialize(&owner).await;

    let expected = fence(1)?;
    let item = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i949".into(),
        revision_ref: "definition-r1".into(),
    };
    let cause = ItemMintCause::for_test(
        dependencies.admitted.world_id,
        dependencies.admitted.channel_id,
        expected.scope_ownership_generation,
        99,
        1,
        item.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    );
    let mut candidate = h
        .root
        .freeze_item_mint(
            &authority,
            &h.node,
            ItemMintRequest {
                cause,
                item,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position: map_item_spatial_position(7, 0, 7),
                    corpse_ref: id(99).to_vec(),
                    map_revision: "map-1".into(),
                    content_revision: "content-1".into(),
                    native_room_placement_context: id(6).to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("corpse mint lane")?;
    let result = h
        .root
        .commit_corpse_mint(&permit, &authority, &h.node, &mut candidate, id(41))
        .await
        .map_err(debug)?;
    let ItemMintOutcome::Committed(corpse) = result else {
        return Err(debug(result));
    };

    let stages: Vec<(String, i64)> = sqlx::query_as(
        "SELECT s.target_production_key,s.expires_at_unix_ms-m.materialized_at
         FROM game_spell_field_temporal_schedules s JOIN game_item_mint_receipts m USING(item_instance_id)
         WHERE s.item_instance_id=encode($1,'hex')::uuid ORDER BY s.source_stage",
    )
    .bind(corpse.item_instance_id.as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(stages, [
        ("oteryn:item.tibia.i951".to_owned(), 10_000),
        ("oteryn:item.tibia.i955".to_owned(), 310_000),
        ("oteryn:item.tibia.i956".to_owned(), 610_000),
    ]);
    let lifetime: i64 = sqlx::query_scalar(
        "SELECT game_corpse_retirement_deadline(item_instance_id)-materialized_at
         FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(corpse.item_instance_id.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifetime, 670_000);
    Ok(())
}
