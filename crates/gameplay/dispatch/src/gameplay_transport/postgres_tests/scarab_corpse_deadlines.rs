//! Crystal 201189f8 data-global/monster/vermins/scarab.lua:33 and
//! data/items/items.xml:13001-13016,16567-16572.
use std::{sync::Arc, time::Duration};

use super::TestResult;
use super::archivist_vote::{FixtureCorpseLoot, FixtureItem, with_npc_items_and_loot};
use super::fixture::{Harness, debug, run};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;

const SCARAB: &str = "oteryn:creature.scarab";
const CORPSE: &str = "oteryn:item.tibia.i6024";

#[test]
fn native_scarab_death_reaches_mature_shell_and_retires_after_its_final_stage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "scarab_corpse_deadlines",
                    false,
                    false,
                )
                .await?;
                let result = Box::pin(exercise(&h)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Scarab corpse deadline case panicked")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let catalogue = Arc::new(QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)?);
    let root = FixtureItem {
        id: 6024,
        name: "dead scarab",
        weight: 0,
        stackable: false,
    };
    let mut bodies = vec![root];
    bodies.extend([4194, 4195, 4196].map(|stage| FixtureItem {
        id: stage,
        name: "dead scarab",
        weight: 2000,
        stackable: false,
    }));
    let loot = FixtureCorpseLoot {
        corpse: CORPSE,
        table: "oteryn:loot.creature.scarab",
    };
    with_npc_items_and_loot(h, catalogue, "oteryn:npc.tonar", &bodies, Some(loot), async |owner, admitted, _| {
        let native = owner.active_generation.and_then(|generation| generation.native_gameplay()).ok_or("native Scarab policy")?;
        native.reward_table().row(SCARAB).map_err(debug)?;
        native.install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
        super::qengine_kill_stories::death(owner, admitted, SCARAB, 81).await?;

        let (corpse, materialized, retire_at): (String, i64, i64) = sqlx::query_as(
            "SELECT item_instance_id::text,materialized_at,game_corpse_retirement_deadline(item_instance_id) \
             FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' \
             AND draw_ordinal=0 AND loot_table_production_key=$1",
        ).bind(CORPSE).fetch_one(&h.pool).await?;
        assert_eq!(retire_at - materialized, 670_000);

        let stages: Vec<(i32, i64, i64, String, String)> = sqlx::query_as(
            "SELECT source_stage,duration_millis,expires_at_unix_ms-$2,definition_production_key,target_production_key \
             FROM game_spell_field_temporal_schedules WHERE item_instance_id=$1::uuid ORDER BY source_stage",
        ).bind(&corpse).bind(materialized).fetch_all(&h.pool).await?;
        assert_eq!(stages, [
            (1, 10_000, 10_000, CORPSE.into(), "oteryn:item.tibia.i4194".into()),
            (2, 300_000, 310_000, "oteryn:item.tibia.i4194".into(), "oteryn:item.tibia.i4195".into()),
            (3, 300_000, 610_000, "oteryn:item.tibia.i4195".into(), "oteryn:item.tibia.i4196".into()),
        ]);
        let read = || async {
            sqlx::query_as::<_, (String, i16, i64)>(
                "SELECT definition_production_key,lifecycle,quantity FROM game_item_instances WHERE item_instance_id=$1::uuid",
            ).bind(&corpse).fetch_one(&h.pool).await
        };
        age(h, &corpse, 311_000).await?;
        owner.drain_source_item_deadlines().await;
        assert_eq!(read().await?, ("oteryn:item.tibia.i4195".into(), 1, 1));

        age(h, &corpse, 300_000).await?;
        owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_secs(2)).ok_or("decay owner clock")?;
        owner.drain_source_item_deadlines().await;
        assert_eq!(read().await?, ("oteryn:item.tibia.i4196".into(), 1, 1));

        age(h, &corpse, 60_000).await?;
        owner.drain_corpse_deadlines().await;
        assert_eq!(read().await?, ("oteryn:item.tibia.i4196".into(), 2, 0));
        Ok(())
    }).await
}

async fn age(h: &Harness, corpse: &str, millis: i64) -> TestResult {
    // Shift the fixture's original clock and schedule together. The live deadline
    // passes still use the PostgreSQL clock and the immutable receipt/custody proofs.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-$2 WHERE item_instance_id=$1::uuid")
        .bind(corpse).bind(millis).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=expires_at_unix_ms-$2 WHERE item_instance_id=$1::uuid")
        .bind(corpse).bind(millis).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
