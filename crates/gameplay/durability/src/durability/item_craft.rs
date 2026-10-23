//! CRAFT-1: one fenced, receipt-keyed consume/mint transaction. Basic fixed cooking only.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::begin_type2_transaction;
use super::item_loop_move::{ItemLoopContent, carried_weight};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{
    CurrentCharacterItemFence, read_derived_transfer_capacity_centi_oz, scope_of, stack_maximum,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::CommandRef;
use oteryn_gameplay_items::item_craft::cooking_conversion;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::{Value, json};
use sqlx::Row;

pub(crate) mod audit;

#[derive(Debug, Clone)]
pub struct ItemCraftRequest {
    pub command: CommandRef,
    pub source: [u8; 16],
    pub station: TypedDefinitionRef,
    /// Original bounded USE bytes. Replay compares these before resolving current handles.
    pub intent: Vec<u8>,
}

impl DurabilityRoot {
    /// Probe or commit a basic cooking command. A probe needs no current handles or station;
    /// an exact historical receipt is returned before live custody checks after a restart.
    #[allow(clippy::too_many_arguments)]
    pub async fn craft_item<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(Some(UseDisposition::Rejected));
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            // The same lock serializes probes and commits, including a cancelled acknowledgement.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-craft:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT intent,uuid_send(character_id) FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((intent, character)) = previous {
                let result = if intent == request.intent && character == fence.character_id.as_bytes() { UseDisposition::Committed } else { UseDisposition::Rejected };
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            if probe { tx.commit(deadline).await?; return Ok(None); }
            if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(request.command), context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(UseDisposition::Rejected));
            }
            let result = write(&mut tx, fence, &request, context.0, context.1).await?;
            if result == UseDisposition::Committed { tx.commit(deadline).await?; }
            Ok(Some(result))
        })).await
    }
}

#[allow(clippy::too_many_lines)]
async fn write<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    content: &C,
) -> Result<UseDisposition, DurabilityError> {
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    // Root serialization precedes all item locks. Basic cooking accepts main-backpack leaves.
    let source = sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,uuid_send(e.parent_item_instance_id) AS parent,e.placement_ordinal::text AS ordinal FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND e.world_id=i.world_id AND b.world_id=i.world_id FOR UPDATE OF i")
        .bind(request.source.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(source) = source else {
        return Ok(UseDisposition::StaleState);
    };
    let definition = TypedDefinitionRef {
        family: source.try_get("definition_family")?,
        production_key: source.try_get("definition_production_key")?,
        revision_ref: source.try_get("definition_revision_ref")?,
    };
    let Some(conversion) = cooking_conversion(native, &definition, &request.station) else {
        return Ok(UseDisposition::NothingToUse);
    };
    let before: Value = source.try_get("before_item")?;
    let quantity: i64 = source.try_get("quantity")?;
    let parent: Vec<u8> = source.try_get("parent")?;
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let tree_count: i64 = sqlx::query_scalar(
        "WITH RECURSIVE tree(item_instance_id,depth) AS ( \
           SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid \
           UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e \
             ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8 \
         ) SELECT count(*) FROM (SELECT item_instance_id FROM tree LIMIT 501) bounded",
    ).bind(&parent).fetch_one(&mut ***tx).await?;
    if tree_count + i64::try_from(conversion.outputs.len()).map_err(|_| DurabilityError::InvalidStoredState)? - i64::from(quantity == 1) > 500 {
        return Ok(UseDisposition::NoRoom);
    }
    let bag = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(&parent).fetch_one(&mut ***tx).await?;
    let bag_definition = TypedDefinitionRef {
        family: bag.try_get("definition_family")?,
        production_key: bag.try_get("definition_production_key")?,
        revision_ref: bag.try_get("definition_revision_ref")?,
    };
    let capacity = content
        .loop_facts(&bag_definition)
        .and_then(|f| f.container_capacity)
        .unwrap_or(0)
        .min(20);
    if occupied
        + i64::try_from(conversion.outputs.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
        - i64::from(quantity == 1)
        > i64::from(capacity)
    {
        return Ok(UseDisposition::NoRoom);
    }
    // Whole carried weight and capacity use the same locked projection as the item loop.
    let carried = match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await {
        Ok(w) => w,
        Err(_) => return Ok(UseDisposition::NotPossible),
    };
    let Some(used_weight) = content.loop_facts(&definition).and_then(|f| f.weight) else {
        return Ok(UseDisposition::NotPossible);
    };
    let mut output_weight = 0_u64;
    for (definition, count) in &conversion.outputs {
        let Some(facts) = content.loop_facts(definition) else {
            return Ok(UseDisposition::NotPossible);
        };
        let Some(weight) = facts.weight else {
            return Ok(UseDisposition::NotPossible);
        };
        if facts.container_capacity.is_some()
            || *count
                > stack_maximum(facts.stack)
                    .map(|(maximum, _)| maximum)
                    .unwrap_or(0)
        {
            return Ok(UseDisposition::NotPossible);
        }
        output_weight += u64::from(weight) * u64::from(*count);
    }
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id).await?;
    if carried
        .checked_sub(u64::from(used_weight))
        .and_then(|w| w.checked_add(output_weight))
        .is_none_or(|w| w > capacity)
    {
        return Ok(UseDisposition::NotPossible);
    }
    let highest: String = sqlx::query_scalar("SELECT coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let mut ordinal: u64 = highest
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut outputs = Vec::new();
    for (position, (definition, count)) in conversion.outputs.iter().enumerate() {
        let output_ordinal = if quantity == 1 && position == 0 {
            source.try_get::<String, _>("ordinal")?
        } else {
            ordinal = ordinal.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            ordinal.to_string()
        };
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        outputs.push(json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref, "count": count, "ordinal": output_ordinal}));
    }
    let (transaction, event, occurred): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let envelope = audit::encode(
        fence,
        request,
        &conversion.recipe,
        native.source_digest(),
        &before,
        &outputs,
        &transaction,
        &event,
        occurred,
        tx.tuple().tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15))")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(&conversion.recipe).bind(request.source.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(request.source.as_slice()).bind(&transaction).execute(&mut ***tx).await?;
    if quantity == 1 {
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(request.source.as_slice())
        .execute(&mut ***tx)
        .await?;
    }
    for output in outputs {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,($1->>'ordinal')::numeric,$5::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(&transaction).execute(&mut ***tx).await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tx.tuple().schema_revision())).bind(tx.tuple().retention_profile_id()).bind(request.source.as_slice()).bind(occurred).bind(super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(UseDisposition::Committed)
}
