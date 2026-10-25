//! NIGHT-W1 decision 11 through a real auto-attack kill and the server owner timer.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::{scenario_thread, scenario_with_decay};
use crate::durability::item_decay_retire::CorpseDecayFence;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use std::time::Duration;

#[test]
fn corpse_decay_real_kill_timer_retires_contents_preserves_ordinary_ground() -> TestResult {
    scenario_thread::run("corpse-decay-dispatch", || scenario_with_decay(Some(false)))
}

#[test]
fn corpse_decay_timer_reconciles_retired_corpse_after_missed_runtime_update() -> TestResult {
    scenario_thread::run("corpse-decay-reconcile", || scenario_with_decay(Some(true)))
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    children: i64,
    reconcile: bool,
) -> TestResult {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or("the committed kill has a visible corpse")?;
    let generation = owner
        .runtime
        .lock()
        .await
        .owner_fence()
        .map_err(debug)?
        .generation();
    // An unrelated ordinary item on a neighboring tile in the same scope, through MINT.
    let ground = {
        let mut candidate = h
            .root
            .freeze_item_mint(
                owner.character,
                &h.node,
                ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        owner.world_id,
                        owner.channel_id,
                        generation,
                        1_000,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:loot.corpse-decay-control".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:purpose.corpse-decay-control".into(),
                        0,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i3031".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 1,
                    ground: GroundPlacement {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                corpse
                                    .position
                                    .x
                                    .checked_add(1)
                                    .ok_or("Ground x")?
                                    .try_into()?,
                                corpse.position.y.try_into()?,
                                corpse.position.floor.try_into()?,
                            ),
                        corpse_ref: vec![3; 16],
                        map_revision: "map-1".into(),
                        content_revision: "content-1".into(),
                        native_room_placement_context: vec![6; 16],
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(|error| format!("ordinary Ground MINT freeze: {error:?}"))?;
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        match h
            .root
            .commit_item_mint(&permit, owner.character, &h.node, &mut candidate)
            .await
            .map_err(|error| format!("ordinary Ground MINT commit: {error:?}"))?
        {
            ItemMintOutcome::Committed(item) => item.item_instance_id,
            other => return Err(debug(other)),
        }
    };
    owner.tick_timed_items().await;
    assert_eq!(
        retire_count(h).await?,
        0,
        "a young corpse never retires early"
    );
    assert_eq!(
        owner.runtime.lock().await.visible_corpses(),
        vec![corpse.clone()]
    );

    // Move only the fixture's durable clock anchor, using the same maintenance technique as
    // corpse_decay_postgres_cases. The production deadline checks and writer stay enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at = floor(extract(epoch FROM clock_timestamp())*1000)::bigint - 61000 WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(corpse.item.item_instance_id.as_slice())
        .execute(&mut *tx).await?;
    // Ordinary Ground has no materialization deadline column: age its creation receipt only.
    sqlx::query("UPDATE game_item_mint_receipts SET occurred_at = floor(extract(epoch FROM clock_timestamp())*1000)::bigint - 61000 WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(ground.as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;

    if reconcile {
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        h.root
            .retire_decayed_corpse(
                &permit,
                owner.character,
                &h.node,
                CorpseDecayFence {
                    world_id: owner.world_id,
                    channel_id: owner.channel_id,
                    scope_ownership_generation: generation,
                },
                corpse.item.item_instance_id,
            )
            .await
            .map_err(debug)?;
        assert_eq!(owner.runtime.lock().await.visible_corpses().len(), 1);
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    // Run the same timer loop serve_gameplay polls, with no client packet or connection tick.
    let stop = oteryn_foundation::CancellationToken::new();
    let observe = async {
        loop {
            if retire_count(h).await? == children + 1
                && owner.runtime.lock().await.visible_corpses().is_empty()
            {
                stop.cancel();
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let (_, result) =
            super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe).await;
        result
    })
    .await??;
    assert_eq!(retire_count(h).await?, children + 1);
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining, 0, "no loot is spilled to Ground");
    let grounds: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(grounds, 1, "only the ordinary Ground item remains");
    let retired: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE lifecycle = 2 AND quantity = 0",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(retired, children + 1);
    let ordinary: (i16, i64) = sqlx::query_as("SELECT i.lifecycle, i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id = encode($1,'hex')::uuid")
        .bind(ground.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        ordinary,
        (1, 1),
        "ordinary Ground survives the same elapsed time"
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .bound_corpse(corpse.identity)
            .is_none()
    );
    // A subsequent item clock pass cannot emit another retirement for any of these items.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("owner clock")?;
    owner.tick_timed_items().await;
    assert_eq!(retire_count(h).await?, children + 1);
    Ok(())
}

async fn retire_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
