//! Crystal 201189f8 data-global/monster/vermins/scarab.lua:33 and
//! data/items/items.xml:13001-13016,16567-16572.
use std::{sync::Arc, time::Duration};

use super::TestResult;
use super::archivist_vote_fixtures::{FixtureCorpseLoot, FixtureItem, with_npc_items_and_loot};
use super::classic_quests_fixtures::{talk, talk_with_disposition};
use super::fixture::{Harness, debug, run};
use crate::gameplay_transport::{
    connection::UseCommand,
    item_move::{LoopMove, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
    loop_move::move_loop_item,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const SCARAB: &str = "oteryn:creature.scarab";
const CORPSE: &str = "oteryn:item.tibia.i6024";

#[test]
fn native_scarab_death_reaches_mature_shell_and_retires_after_its_final_stage() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    case(admin, "scarab_corpse_deadlines", ShellUse::Ground)
}

#[test]
fn native_scarab_pickup_carries_the_absolute_deadline_and_tonar_accepts_only_the_mature_window()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    case(admin.clone(), "scarab_shell_hand_in", ShellUse::Trade)?;
    case(admin, "scarab_carried_deadlines", ShellUse::Expire)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShellUse {
    Ground,
    Trade,
    Expire,
}

fn case(admin: String, name: &'static str, use_: ShellUse) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
                let result = Box::pin(exercise(&h, use_)).await;
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

async fn exercise(h: &Harness, use_: ShellUse) -> TestResult {
    let catalogue = Arc::new(super::tonar_scarab_shell_exchange_fixtures::catalogue()?);
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
    bodies.push(FixtureItem {
        id: 43740,
        name: "scarab ocarina",
        weight: 120,
        stackable: false,
    });
    let loot = FixtureCorpseLoot {
        corpse: CORPSE,
        table: "oteryn:loot.creature.scarab",
    };
    with_npc_items_and_loot(h, catalogue, "oteryn:npc.tonar", &bodies, Some(loot), async |owner, admitted, tonar| {
        let native = owner.active_generation.and_then(|generation| generation.native_gameplay()).ok_or("native Scarab policy")?;
        native.reward_table().row(SCARAB).map_err(debug)?;
        native.install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
        super::qengine_kill_stories_fixtures::death(owner, admitted, SCARAB, 81).await?;

        let (corpse, materialized, retire_at): (String, i64, i64) = sqlx::query_as(
            "SELECT item_instance_id::text,materialized_at,game_corpse_retirement_deadline(item_instance_id) \
             FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' \
             AND draw_ordinal=0 AND loot_table_production_key=$1",
        ).bind(CORPSE).fetch_one(&h.pool).await?;
        assert_eq!(retire_at - materialized, 670_000);

        let stages: Vec<(i32, i64, i64, String, Option<String>)> = sqlx::query_as(
            "SELECT source_stage,duration_millis,expires_at_unix_ms-$2,definition_production_key,target_production_key \
             FROM game_spell_field_temporal_schedules WHERE item_instance_id=$1::uuid ORDER BY source_stage",
        ).bind(&corpse).bind(materialized).fetch_all(&h.pool).await?;
        assert_eq!(stages, [
            (1, 10_000, 10_000, CORPSE.into(), Some("oteryn:item.tibia.i4194".into())),
            (2, 300_000, 310_000, "oteryn:item.tibia.i4194".into(), Some("oteryn:item.tibia.i4195".into())),
            (3, 300_000, 610_000, "oteryn:item.tibia.i4195".into(), Some("oteryn:item.tibia.i4196".into())),
            (4, 60_000, 670_000, "oteryn:item.tibia.i4196".into(), None),
        ]);
        let read = || async {
            sqlx::query_as::<_, (String, i16, i64)>(
                "SELECT definition_production_key,lifecycle,quantity FROM game_item_instances WHERE item_instance_id=$1::uuid",
            ).bind(&corpse).fetch_one(&h.pool).await
        };
        age(h, &corpse, 311_000).await?;
        owner.drain_source_item_deadlines().await;
        assert_eq!(read().await?, ("oteryn:item.tibia.i4195".into(), 1, 1));

        if use_ != ShellUse::Ground {
            let actor = admitted.runtime_actor.ok_or("player holding the Scarab shell")?;
            let item = *corpse.parse::<sqlx::types::Uuid>()?.as_bytes();
            let fence = owner.current_quest_fence(admitted.game_session_id).await
                .map_err(|()| "current Scarab pickup fence")?.ok_or("Scarab pickup fence")?;
            assert_eq!(move_loop_item(owner, actor, UseCommand {
                game_session_id: admitted.game_session_id, command_id: 82, item_fence: Some(fence.item_fence()),
            }, LoopMove { item: ItemKey::Instance(item), count: None, target: LoopTarget::MainBackpack }).await, LoopMoveResult::Committed);
            let carried: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=$1::uuid)").bind(&corpse).fetch_one(&h.pool).await?;
            assert!(carried);
            super::tonar_scarab_shell_exchange_fixtures::install_offer(owner)?;
            talk(owner, admitted, tonar, 83, "hi").await?;
            talk(owner, admitted, tonar, 84, "scarab").await?;
            if use_ == ShellUse::Trade {
                assert_eq!(talk(owner, admitted, tonar, 85, "yes").await?, ["Here is your scarab ocarina."]);
                assert_eq!(read().await?.2, 0);
                assert_eq!(super::full_world_support::quantity(h, "oteryn:item.tibia.i43740").await?, 1);
                return Ok(());
            }
        }

        age(h, &corpse, 300_000).await?;
        if use_ == ShellUse::Expire {
            assert_eq!(talk_with_disposition(owner, admitted, tonar, 85, "yes", NpcIntentDisposition::Rejected).await?, ["Wait for it to decompose."]);
            assert_eq!(super::full_world_support::quantity(h, "oteryn:item.tibia.i43740").await?, 0);
        }
        owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_secs(2)).ok_or("decay owner clock")?;
        owner.drain_source_item_deadlines().await;
        assert_eq!(read().await?, ("oteryn:item.tibia.i4196".into(), 1, 1));

        age(h, &corpse, 60_000).await?;
        if use_ == ShellUse::Ground {
            owner.drain_corpse_deadlines().await;
        } else {
            owner.clock_origin = owner.clock_origin.checked_sub(Duration::from_secs(2)).ok_or("decay owner clock")?;
            owner.drain_source_item_deadlines().await;
        }
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
