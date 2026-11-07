//! SUMMER decision 20: shared corpse owner capacity, mint replay and full drain.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::{scenario_thread, scenario_with_capacity};
use crate::durability::item_mint::{
    CorpseContainerPlacement, CorpseLootMintRequest, ItemMintCause, ItemMintError, ItemMintOutcome,
    TypedDefinitionRef,
};
use crate::foundation::ScopeOwnershipGeneration;
use crate::gameplay_transport::ComposedFreshAdmission;
use sqlx::Row;

#[test]
fn summer_corpse_capacity_thirty_entries_mint_replay_and_dispatch_decay() -> TestResult {
    scenario_thread::run("summer-corpse-capacity", || {
        scenario_with_capacity(Some(false), true)
    })
}

#[test]
fn summer_corpse_capacity_thirty_entries_reconciles_missed_runtime_drain() -> TestResult {
    scenario_thread::run("summer-corpse-reconcile", || {
        scenario_with_capacity(Some(true), true)
    })
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult<i64> {
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or("a real auto-attack kill produced the corpse")?;
    let parent = corpse.item.item_instance_id;
    // Use the actual committed death identity. Additional entries are boundary fixtures
    // for the shared owner, not a claim that the rat's content capacity or loot grew.
    let row = sqlx::query(
        "SELECT death_scope_ownership_generation::text AS generation, \
                death_actor_local_id, death_actor_local_generation::text AS actor_generation, \
                loot_table_family, loot_table_production_key, loot_table_revision_ref \
         FROM game_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(parent.as_slice())
    .fetch_one(&h.pool)
    .await?;
    let generation = ScopeOwnershipGeneration::new(row.try_get::<String, _>("generation")?.parse()?)
        .map_err(debug)?;
    let actor = u32::try_from(row.try_get::<i64, _>("death_actor_local_id")?)?;
    let actor_generation = row.try_get::<String, _>("actor_generation")?.parse()?;
    let table = TypedDefinitionRef {
        family: row.try_get("loot_table_family")?,
        production_key: row.try_get("loot_table_production_key")?,
        revision_ref: row.try_get("loot_table_revision_ref")?,
    };
    let start: i64 = sqlx::query_scalar(
        "SELECT COALESCE(max(placement_ordinal),0)::bigint \
         FROM game_item_corpse_container_entries WHERE parent_item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(parent.as_slice())
    .fetch_one(&h.pool)
    .await?;
    let request = |ordinal| CorpseLootMintRequest {
        cause: ItemMintCause::for_test(
            owner.world_id,
            owner.channel_id,
            generation,
            actor,
            actor_generation,
            table.clone(),
            "fixture:purpose.summer-capacity".into(),
            ordinal,
        ),
        item: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3031".into(),
            revision_ref: "definition-r1".into(),
        },
        quantity: 1,
        placement: CorpseContainerPlacement {
            corpse_item_instance_id: parent,
            placement_ordinal: ordinal,
            fluid_subtype: None,
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    for ordinal in u32::try_from(start)? + 1..=30 {
        let mut candidate = h
            .root
            .freeze_corpse_loot_mint(owner.character, &h.node, request(ordinal))
            .await
            .map_err(debug)?;
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        let outcome = h
            .root
            .commit_corpse_loot_mint(&permit, owner.character, &h.node, &mut candidate)
            .await
            .map_err(debug)?;
        let ItemMintOutcome::Committed(committed) = outcome else {
            return Err(debug(outcome));
        };
        // Reconstruct from the durable reservation, as after a lost response.
        let mut replay = h
            .root
            .freeze_corpse_loot_mint(owner.character, &h.node, request(ordinal))
            .await
            .map_err(debug)?;
        let replayed = h
            .root
            .commit_corpse_loot_mint(&permit, owner.character, &h.node, &mut replay)
            .await
            .map_err(debug)?;
        assert!(matches!(replayed, ItemMintOutcome::AlreadyCommitted(item)
            if item.item_instance_id == committed.item_instance_id
                && item.transaction_id == committed.transaction_id));
    }
    assert!(
        matches!(
            h.root
                .freeze_corpse_loot_mint(owner.character, &h.node, request(31))
                .await,
            Err(ItemMintError::InvalidInput)
        ),
        "the 31st entry is refused before reserving a candidate"
    );
    let children: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_corpse_container_entries \
         WHERE parent_item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(parent.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(children, 30, "replay creates no duplicate entries");
    Ok(children)
}
