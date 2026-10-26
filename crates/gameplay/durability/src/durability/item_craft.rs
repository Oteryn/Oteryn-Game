//! CRAFT-1: one fenced, receipt-keyed consume/mint transaction. Basic fixed cooking and
//! REWARD-CHEST-WIRE-1 reward bags and SOULPIT two-core fusion.
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
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_craft::cooking_conversion;
use oteryn_gameplay_items::reward_bag::{reward_bag_conversion, reward_bag_draw};
use oteryn_gameplay_items::soul_core::{soul_core_fusion, soulpit_transformation};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::{Value, json};
use sqlx::Row;

pub(crate) mod audit;
mod custody;

#[derive(Debug, Clone)]
pub struct ItemCraftRequest {
    pub command: CommandRef,
    pub source: [u8; 16],
    /// SOULPIT: a second input in carried or observed Ground custody; one stack consumes two units.
    pub target: Option<[u8; 16]>,
    /// Ground observations resolved within reach by the dispatch owner; at most two inputs.
    pub ground_inputs: Vec<([u8; 16], Vec<u8>)>,
    /// The cooking station; without it, `target` selects fusion, otherwise a reward-bag draw.
    pub station: Option<TypedDefinitionRef>,
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
            || request.ground_inputs.len() > 2
            || request.ground_inputs.iter().any(|(item, cell)| {
                cell.len() != 10 || (*item != request.source && Some(*item) != request.target)
            })
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
    // The root fence serializes custody. Lock both inputs in UUID order before reading either.
    sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid OR item_instance_id=encode($2,'hex')::uuid ORDER BY item_instance_id FOR UPDATE")
        .bind(request.source.as_slice()).bind(request.target.unwrap_or(request.source).as_slice()).fetch_all(&mut ***tx).await?;
    let source = custody::source(tx, fence, request, request.source).await?;
    let Some(source) = source else {
        return Ok(UseDisposition::StaleState);
    };
    let definition = TypedDefinitionRef {
        family: source.try_get("definition_family")?,
        production_key: source.try_get("definition_production_key")?,
        revision_ref: source.try_get("definition_revision_ref")?,
    };
    let source_units = if request.target == Some(request.source) {
        2_i64
    } else {
        1
    };
    let secondary = match request.target.filter(|id| *id != request.source) {
        Some(id) => match custody::source(tx, fence, request, id).await? {
            Some(row) => Some(row),
            None => return Ok(UseDisposition::StaleState),
        },
        None => None,
    };
    let conversion = if request.target.is_some() {
        if request.station.is_some() {
            return Ok(UseDisposition::Rejected);
        }
        let target = match &secondary {
            Some(row) => TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            },
            None => definition.clone(),
        };
        // Server-generated entropy is never supplied by the client. Only committed draws are
        // observable; replay returns the retained receipt before this code can draw again.
        let draw: Vec<u8> = sqlx::query_scalar("SELECT game_soulpit_entropy()")
            .fetch_one(&mut ***tx)
            .await?;
        let ominous_draw = u64::from_be_bytes(
            draw[8..16]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        );
        let draw = u64::from_be_bytes(
            draw[..8]
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        );
        soul_core_fusion(native, &definition, &target, draw)
            .or_else(|| soulpit_transformation(native, &definition, &target, draw, ominous_draw))
    } else {
        match &request.station {
            Some(station) => cooking_conversion(native, &definition, station),
            None => reward_bag_conversion(
                native,
                &definition,
                reward_bag_draw(
                    *fence.game_session_id.as_bytes(),
                    request.command.command_id().get(),
                    request.source,
                ),
            ),
        }
    };
    let Some(conversion) = conversion else {
        return Ok(UseDisposition::NothingToUse);
    };
    let before: Value = source.try_get("before_item")?;
    let quantity: i64 = source.try_get("quantity")?;
    if quantity < source_units {
        // The current core is valid, but there are not two units to combine.
        return Ok(UseDisposition::NotPossible);
    }
    // Outputs enter the main backpack. Input custody is independently retained in the receipt.
    let Some(parent) = sqlx::query_scalar::<_, Vec<u8>>("SELECT uuid_send(item_instance_id) FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await? else {
        return Ok(UseDisposition::NoRoom);
    };
    let direct = |row: &sqlx::postgres::PgRow| -> Result<bool, sqlx::Error> {
        let location: Value = row.try_get("custody")?;
        Ok(location["kind"] == "main" && row.try_get::<Vec<u8>, _>("parent")? == parent)
    };
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let tree_count: i64 = sqlx::query_scalar(
        "WITH RECURSIVE tree(item_instance_id,depth) AS ( \
           SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid \
           UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e \
             ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8 \
         ) SELECT count(*) FROM (SELECT item_instance_id FROM tree LIMIT 501) bounded",
    ).bind(&parent).fetch_one(&mut ***tx).await?;
    let secondary_frees_slot = secondary
        .as_ref()
        .is_some_and(|row| row.try_get::<i64, _>("quantity").ok() == Some(1));
    let freed = i64::from(quantity == source_units && direct(&source)?)
        + i64::from(
            secondary_frees_slot && secondary.as_ref().map(direct).transpose()?.unwrap_or(false),
        );
    let in_tree = |row: &sqlx::postgres::PgRow| -> Result<bool, sqlx::Error> {
        let location: Value = row.try_get("custody")?;
        Ok(matches!(location["kind"].as_str(), Some("main" | "nested")))
    };
    let tree_freed = i64::from(quantity == source_units && in_tree(&source)?)
        + i64::from(
            secondary_frees_slot
                && secondary
                    .as_ref()
                    .map(in_tree)
                    .transpose()?
                    .unwrap_or(false),
        );
    if tree_count
        + i64::try_from(conversion.outputs.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
        - tree_freed
        > 500
    {
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
        - freed
        > i64::from(capacity)
    {
        return Ok(UseDisposition::NoRoom);
    }
    // Whole carried weight and capacity use the same locked projection as the item loop.
    let carried = match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await {
        Ok(w) => w,
        Err(_) => return Ok(UseDisposition::NotPossible),
    };
    let Some(unit_weight) = content.loop_facts(&definition).and_then(|f| f.weight) else {
        return Ok(UseDisposition::NotPossible);
    };
    let mut used_weight = u64::from(unit_weight)
        * u64::try_from(source_units).map_err(|_| DurabilityError::InvalidStoredState)?;
    if let Some(row) = &secondary {
        let target = TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        };
        let Some(weight) = content.loop_facts(&target).and_then(|f| f.weight) else {
            return Ok(UseDisposition::NotPossible);
        };
        used_weight += u64::from(weight);
    }
    if source.try_get::<Value, _>("custody")?["kind"] == "ground" {
        used_weight -= u64::from(unit_weight)
            * u64::try_from(source_units).map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    if let Some(row) = &secondary
        && row.try_get::<Value, _>("custody")?["kind"] == "ground"
    {
        let target = TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        };
        used_weight -= u64::from(
            content
                .loop_facts(&target)
                .and_then(|f| f.weight)
                .ok_or(DurabilityError::InvalidStoredState)?,
        );
    }
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
        .checked_sub(used_weight)
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
        let output_ordinal = if quantity == source_units && direct(&source)? && position == 0 {
            source.try_get::<String, _>("ordinal")?
        } else {
            ordinal = ordinal
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            ordinal.to_string()
        };
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        outputs.push(json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref, "count": count, "ordinal": output_ordinal}));
    }
    let (transaction, event, occurred): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let tuple = tx.tuple();
    let secondary_before = secondary
        .as_ref()
        .map(|row| row.try_get::<Value, _>("before_item"))
        .transpose()?;
    let envelope = audit::encode(
        fence,
        request,
        &conversion.recipe,
        native.source_digest(),
        &before,
        source_units,
        secondary_before.as_ref(),
        &source.try_get::<Value, _>("custody")?,
        secondary
            .as_ref()
            .map(|row| row.try_get::<Value, _>("custody"))
            .transpose()?
            .as_ref(),
        &outputs,
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    let secondary_parent = secondary
        .as_ref()
        .map(|row| row.try_get::<Vec<u8>, _>("parent"))
        .transpose()?;
    let secondary_ordinal = secondary
        .as_ref()
        .map(|row| row.try_get::<String, _>("ordinal"))
        .transpose()?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,source_units,secondary_item_instance_id,secondary_parent_item_instance_id,secondary_ordinal,secondary_before,source_custody,secondary_custody,input_channel_id) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15),$16,encode($17,'hex')::uuid,encode($18,'hex')::uuid,$19::text::numeric,$20,$21,$22,encode($23,'hex')::uuid)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(&conversion.recipe).bind(request.source.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).bind(i16::try_from(source_units).map_err(|_| DurabilityError::InvalidStoredState)?).bind(request.target.filter(|id| *id != request.source).map(|id| id.to_vec())).bind(&secondary_parent).bind(&secondary_ordinal).bind(&secondary_before).bind(source.try_get::<Value, _>("custody")?).bind(secondary.as_ref().map(|r| r.try_get::<Value, _>("custody")).transpose()?).bind(scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?.1.as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=quantity-$3,lifecycle=CASE WHEN quantity=$3 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(request.source.as_slice()).bind(&transaction).bind(source_units).execute(&mut ***tx).await?;
    if quantity == source_units {
        custody::remove(tx, request.source, &source).await?;
    }
    if let Some(id) = request.target.filter(|id| *id != request.source) {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-1,lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id.as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        if secondary_frees_slot {
            custody::remove(
                tx,
                id,
                secondary
                    .as_ref()
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .await?;
        }
    }
    sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$2::uuid WHERE character_id=encode($1,'hex')::uuid AND EXISTS(SELECT 1 FROM game_item_craft_records WHERE transaction_id=$2::uuid AND equipment_revision_before IS NOT NULL)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
    for output in outputs {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,($1->>'ordinal')::numeric,$5::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(&transaction).execute(&mut ***tx).await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(request.source.as_slice()).bind(occurred).bind(super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(UseDisposition::Committed)
}
