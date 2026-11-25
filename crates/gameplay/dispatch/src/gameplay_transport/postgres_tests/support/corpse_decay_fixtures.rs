use super::TestResult;

use super::fixture::{Harness, debug};

use crate::durability::item_decay_retire::CorpseDecayFence;

use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};

use crate::gameplay_transport::ComposedFreshAdmission;

use std::time::Duration;

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

    let mut tx = h.pool.begin().await?;
    let aged_at = age_past_corpse_retirement(&mut tx, corpse.item.item_instance_id).await?;
    // Ordinary Ground has no materialization deadline column: age its creation receipt only.
    sqlx::query("UPDATE game_item_mint_receipts SET occurred_at = $2 WHERE item_instance_id = encode($1,'hex')::uuid")
        .bind(ground.as_slice())
        .bind(aged_at)
        .execute(&mut *tx).await?;
    tx.commit().await?;

    if reconcile {
        owner.drain_source_item_deadlines().await;
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        h.root
            .retire_decayed_corpse(
                &permit,
                owner.character,
                &h.node,
                CorpseDecayFence {
                    // W02 keeps this pinned Channel fixture in its exact tagged scope.
                    runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                        owner.world_id,
                        owner.channel_id,
                    ),
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

pub(super) async fn retire_hunted_corpse(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    corpse: [u8; 16],
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    age_past_corpse_retirement(&mut tx, corpse).await?;
    tx.commit().await?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_item_deadline_pass_us = 0;
    owner.drain_source_item_deadlines().await;
    // The scoped owner retires one due corpse per pass. An earlier
    // boss corpse can become due while this hunt is in progress.
    let mut retired = false;
    for _ in 0..crate::durability::item_decay_retire::COMBAT01_CORPSES_PER_SCOPE {
        owner.drain_corpse_deadlines().await;
        retired = sqlx::query_scalar(
            "SELECT lifecycle=2 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(corpse.as_slice())
        .fetch_one(&h.pool)
        .await?;
        if retired {
            break;
        }
    }
    assert!(retired, "the real decay owner must retire the hunted corpse");
    Ok(())
}

/// Shift the fixture's MINT anchor and stage deadlines by the same amount.
/// The authored durations, stage proofs and retirement guards stay in force.
pub(super) async fn age_past_corpse_retirement(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item: [u8; 16],
) -> TestResult<i64> {
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut **tx)
        .await?;
    let shift: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint - 1000 \
             - game_corpse_retirement_deadline(encode($1,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let aged_at = sqlx::query_scalar(
        "UPDATE game_item_mint_receipts SET materialized_at = materialized_at + $2 \
         WHERE item_instance_id = encode($1,'hex')::uuid RETURNING materialized_at",
    )
    .bind(item.as_slice())
    .bind(shift)
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms = expires_at_unix_ms + $2 \
         WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .bind(shift)
    .execute(&mut **tx)
    .await?;
    Ok(aged_at)
}

async fn retire_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_decay_retire_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
