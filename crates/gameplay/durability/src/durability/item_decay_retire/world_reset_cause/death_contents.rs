//! Every retained death-tree edge shares the Ground root's atomic retirement.
use super::*;
use crate::durability::world_reset::ground_step::DeathEntryKind;
#[cfg(any(test, feature = "test-support"))]
use crate::durability::world_reset::test_support::{RetirementPhase, RetirementProbe};
use sqlx::types::Uuid;

pub(in crate::durability::item_decay_retire) async fn retire_death_contents(
    tx: &mut mint_audit::Type2Transaction<'_>,
    permit: &SpellLanePermit,
    fence: &CorpseDecayFence,
    node: &NodeIncarnationProof,
    step: DecayRetireStep,
    parent: &Admitted,
    #[cfg(any(test, feature = "test-support"))] mut profile: Option<&mut RetirementProbe>,
) -> std::result::Result<(), DurabilityError> {
    #[cfg(any(test, feature = "test-support"))]
    if let Some(profile) = profile.as_mut() {
        profile.enter(RetirementPhase::TreeLoad);
    }
    if !matches!(
        step.cause,
        RetireCause::WorldReset(_) | RetireCause::ActivityRetirement(_)
    ) || !step.is_corpse_step()
        || !crate::durability::world_reset::ground_step::is_death_container(
            tx,
            step.corpse_item_instance_id,
        )
        .await?
    {
        return Ok(());
    }
    let tree =
        crate::durability::world_reset::ground_step::death_tree(tx, step.item_instance_id).await?;
    #[cfg(any(test, feature = "test-support"))]
    if let Some(profile) = profile.as_mut() {
        profile.descendants(tree.len());
        profile.enter(RetirementPhase::ItemLocks);
    }
    let ids: Vec<_> = tree
        .iter()
        .map(|entry| sqlx::types::Uuid::from_bytes(entry.item))
        .collect();
    // The root is already locked by admission. Lock every descendant before
    // re-reading the edges, then mutate only the exact revalidated graph.
    let rows = sqlx::query("SELECT i.item_instance_id::text,i.world_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,(SELECT coalesce(max(key_number),0)::bigint FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AS key_number,game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id FROM game_item_instances i WHERE i.item_instance_id=ANY($1) ORDER BY i.item_instance_id FOR UPDATE OF i")
        .bind(ids)
        .fetch_all(&mut ***tx).await?;
    #[cfg(any(test, feature = "test-support"))]
    if let Some(profile) = profile.as_mut() {
        profile.enter(RetirementPhase::GraphRecheck);
    }
    if rows.len() != tree.len()
        || crate::durability::world_reset::ground_step::death_tree(tx, step.item_instance_id)
            .await?
            != tree
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    #[cfg(any(test, feature = "test-support"))]
    if let Some(profile) = profile.as_mut() {
        profile.enter(RetirementPhase::Preparation);
    }
    let mut locked = std::collections::BTreeMap::new();
    for row in rows {
        locked.insert(uuid_text(row.try_get("item_instance_id")?)?, row);
    }
    let count = tree.len();
    if count == 0 {
        return Ok(());
    }
    let mut items = Vec::with_capacity(count);
    let mut parents = Vec::with_capacity(count);
    let mut transactions = Vec::with_capacity(count);
    let mut events = Vec::with_capacity(count);
    let mut quantities = Vec::with_capacity(count);
    let mut ordinals = Vec::with_capacity(count);
    let mut envelopes = Vec::with_capacity(count);
    let mut entry_kinds = Vec::with_capacity(count);
    let tuple = tx.tuple();
    for entry in tree {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(profile) = profile.as_mut() {
            profile.begin_child();
        }
        let row = locked
            .remove(&entry.item)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if row.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(row.try_get("world_id")?)? != parent.world_id
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let child = DecayRetireStep {
            item_instance_id: entry.item,
            ..step
        };
        let admitted = Admitted {
            world_id: parent.world_id,
            runtime_scope: parent.runtime_scope,
            definition: OneItemTypedDefinitionRevisionV1 {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            quantity_before: u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            key_number: u32::try_from(row.try_get::<i64, _>("key_number")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            placement_ordinal: Some(entry.ordinal),
            death_entry: Some(entry),
            corpse_ground: parent.corpse_ground.clone(),
            deadline_unix_ms: 0,
            database_now_unix_ms: parent.database_now_unix_ms,
        };
        let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
        let event_id = uuid_text(row.try_get("event_id")?)?;
        let envelope = audit::encode_decay_retire_event(
            DecayRetireEventIdentity {
                event_id,
                transaction_id,
                occurred_at_unix_ms: admitted.database_now_unix_ms,
                server_build_id: SERVER_BUILD_ID,
            },
            decay_message(child, fence, &admitted),
            tuple.tuple(),
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        items.push(Uuid::from_bytes(entry.item));
        parents.push(Uuid::from_bytes(entry.parent));
        transactions.push(Uuid::from_bytes(transaction_id));
        events.push(Uuid::from_bytes(event_id));
        quantities.push(i64::from(admitted.quantity_before));
        ordinals.push(entry.ordinal.to_string());
        envelopes.push(envelope);
        entry_kinds.push(match entry.kind {
            DeathEntryKind::Direct => 1_i16,
            DeathEntryKind::Nested => 2_i16,
        });
    }
    // The held SpellLane cannot change scope or acquire an unresolved attempt.
    // Check it immediately before the batch, which keeps each reservation and
    // its custody/Item/receipt/audit writes in the revalidated postorder. Only
    // transport is batched: every original trigger and live proof still runs.
    permit.check_scope(parent.runtime_scope)?;
    let fact = node.fact();
    let node_id = fact.node_id();
    #[cfg(any(test, feature = "test-support"))]
    if let Some(profile) = profile.as_mut() {
        profile.begin_batch(count);
    }
    let retired: i64 = sqlx::query_scalar(
        "SELECT game_player_death_retire_entries($1, $2, $3, $4, $5, $6, $7, $8, \
         encode($9,'hex')::uuid, $10::text::numeric(20,0), encode($11,'hex')::uuid, \
         encode($12,'hex')::uuid, $13, encode($14,'hex')::uuid, $15, \
         encode($16,'hex')::uuid, $17::text::numeric(20,0), $18, $19, $20::text::numeric, \
         $21, $22, encode($23,'hex')::uuid, $24::text::numeric, $25, $26, $27)",
    )
    .bind(items)
    .bind(parents)
    .bind(transactions)
    .bind(events)
    .bind(quantities)
    .bind(ordinals)
    .bind(envelopes)
    .bind(entry_kinds)
    .bind(step.corpse_item_instance_id.as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(parent.world_id.as_slice())
    .bind(
        parent
            .runtime_scope
            .channel_id()
            .map(|id| id.as_bytes().to_vec()),
    )
    .bind(scope_kind(parent.runtime_scope))
    .bind(parent.runtime_scope.instance_id().map(|id| id.to_vec()))
    .bind(parent.database_now_unix_ms)
    .bind(node_id.as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .bind(i16::try_from(tuple.schema_revision()).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(step.cause.tag())
    .bind(step.cause.epoch())
    .bind(step.cause.binding())
    .bind(step.cause.target())
    .bind(step.cause.handoff_id().map(|id| id.to_vec()))
    .bind(
        step.cause
            .activity_generation()
            .map(|generation| generation.to_string()),
    )
    .bind(EVENT_TYPE_ID)
    .bind(tuple.retention_profile_id())
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .fetch_one(&mut ***tx)
    .await?;
    if usize::try_from(retired).ok() != Some(count) {
        return Err(DurabilityError::InvalidStoredState);
    }
    #[cfg(any(test, feature = "test-support"))]
    if let Some(profile) = profile.as_mut() {
        profile.batch_finished(count);
    }
    Ok(())
}
