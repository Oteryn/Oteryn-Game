//! W07 Soul Core custody, composed through the existing craft receipt and Type2 audit owner.
//! Crystal 201189f `soulpit.lua:155-174`, `soul_prism.lua:43-107`, `exalted_core.lua:34-95`.
//! Selection uses pinned source facts; custody still requires admitted native Item records.

use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::soulpit::{
    ConversionFacts, ConversionKind, ConversionPlan, EXALTED_CORE_SOURCE_ID, SOUL_CORE_SOURCE_IDS,
    SOUL_PRISM_SOURCE_ID, SourceMonsterFacts, conversion_difficulty, is_fusible_soul_core,
    is_soul_core, is_soul_core_output, plan_conversion, soul_core_for_monster,
    soul_core_monster_name, soul_core_name, source_bestiary_candidates, source_monster,
};
use crate::domain::uniform_draw::uniform_draw;
use crate::durability::character_equipment::EquipmentError;
use crate::durability::item_loop_move::{
    ITEM_LOOP_CHANNEL_DROPS_MAX, ITEM_LOOP_DROP_REF_PREFIX, ITEM_LOOP_TREE_ITEMS_MAX, LoopError,
};
use crate::durability::item_mint::{GroundPlacement, ItemMintError, uuid_text};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_content::content::native_gameplay::CreatureProfilesDocument;
use oteryn_gameplay_content::content::{ProjectV2AuthoringProfileData, ProjectV2CreatureAuthoring};
use sha2::{Digest, Sha256};

mod entry;
mod stack;
pub(crate) use entry::burn_entry_core;

#[derive(Debug, Clone)]
pub struct SoulCoreCraftRequest {
    pub command: CommandRef,
    pub source: [u8; 16],
    pub target: [u8; 16],
    pub intent: Vec<u8>,
    /// Absent only for a historical probe. A live command resolves the actor's native cell.
    pub ground: Option<GroundPlacement>,
}

#[derive(Debug)]
struct Input {
    item: [u8; 16],
    before: Value,
    definition: TypedDefinitionRef,
    quantity: u32,
    parent: [u8; 16],
    ordinal: String,
    nested: bool,
}

impl Input {
    fn proof(&self, consume: u32) -> Value {
        json!({"item":hex(&self.item),"before":self.before,"parent":hex(&self.parent),
            "ordinal":self.ordinal,"nested":self.nested,"consume":consume})
    }
}

struct CraftContext<'a, C> {
    fence: CurrentCharacterItemFence,
    request: &'a SoulCoreCraftRequest,
    native: &'a NativeGameplayState,
    content: &'a C,
    permit: &'a SpellLanePermit,
    recovery: &'a CharacterRecoveryFenceV1,
    node: &'a NodeIncarnationProof,
}

impl DurabilityRoot {
    /// One exact USE receipt covers both inputs, random selection, output placement and audit.
    /// Probe before resolving live handles: a retired source still replays its committed result.
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing craft boundary carries its independent authorities and immutable content"
    )]
    pub async fn craft_soul_core<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: SoulCoreCraftRequest,
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
        let recovery = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content, permit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx, &recovery).await?;
            // Same lock and receipt as cooking/reward bags: no competing replay authority.
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
            if request.ground.is_none() { return Ok(Some(UseDisposition::Rejected)); }
            match assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, Some(request.command), context.1.loop_content_digest()).await {
                Ok(_) => {}
                Err(EquipmentError::Durability(error)) => return Err(error),
                Err(EquipmentError::Database(error)) => return Err(error.into()),
                Err(EquipmentError::Rejected(_)) => return Ok(Some(UseDisposition::Rejected)),
            }
            let result = write(&mut tx, CraftContext { fence, request:&request, native:context.0, content:context.1, permit:context.2, recovery:&recovery, node:&node }).await?;
            if result == UseDisposition::Committed { tx.commit(deadline).await?; }
            Ok(Some(result))
        })).await
    }
}

async fn write<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    context: CraftContext<'_, C>,
) -> Result<UseDisposition, DurabilityError> {
    let fence = context.fence;
    let request = context.request;
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let inputs = load_inputs(tx, fence, request).await?;
    let Some(source) = inputs.iter().find(|input| input.item == request.source) else {
        return Ok(UseDisposition::StaleState);
    };
    let Some(target) = inputs.iter().find(|input| input.item == request.target) else {
        return Ok(UseDisposition::StaleState);
    };
    let Some(source_id) = native_source_id(context.native, &source.definition) else {
        return Ok(UseDisposition::NothingToUse);
    };
    let Some(target_id) = native_source_id(context.native, &target.definition) else {
        return Ok(UseDisposition::NothingToUse);
    };
    let Some(kind) = conversion_kind(source_id, target_id) else {
        return Ok(UseDisposition::NothingToUse);
    };
    let carried =
        match carried_weight(tx, fence.character_id, *world.as_bytes(), context.content).await {
            Ok(weight) => weight,
            Err(LoopError::Refused(_)) => return Ok(UseDisposition::NotPossible),
            Err(LoopError::Durability(error)) => return Err(error),
            Err(LoopError::Database(error)) => return Err(error.into()),
        };
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, context.content).await?;
    let selected_id = selected_output(
        context.native,
        kind,
        target_id,
        command_draw(request, b"species"),
    );
    let selected_weight = selected_id
        .and_then(|id| definition_for_source(context.native, id))
        .and_then(|definition| context.content.loop_facts(&definition))
        .and_then(|facts| facts.weight);
    let target_monster = monster_for_core(target_id);
    let facts = ConversionFacts {
        kind,
        source_is_soul_core: is_fusible_soul_core(source_id),
        target_is_soul_core: is_fusible_soul_core(target_id),
        target_monster_exists: target_monster.is_some(),
        source_count: source.quantity,
        target_count: target.quantity,
        same_item_type: source_id == target_id,
        same_instance: source.item == target.item,
        selected_item_id: selected_id,
        selected_item_weight: u64::from(selected_weight.unwrap_or(0)),
        free_capacity: capacity.saturating_sub(carried),
        ominous_roll: u8::try_from(uniform_draw(command_draw(request, b"ominous"), 1, 100))
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    if selected_weight.is_none() {
        return Ok(UseDisposition::NothingToUse);
    }
    let Ok(plan) = plan_conversion(facts) else {
        return Ok(UseDisposition::NotPossible);
    };
    let Some(output_definition) = definition_for_source(context.native, plan.output_item_id) else {
        return Ok(UseDisposition::NothingToUse);
    };
    let Some(output_weight) = context
        .content
        .loop_facts(&output_definition)
        .and_then(|facts| facts.weight)
    else {
        return Ok(UseDisposition::NotPossible);
    };
    let mut proof_inputs = Vec::with_capacity(inputs.len());
    for input in &inputs {
        let consume = consumption(input.item, request, plan);
        proof_inputs.push(input.proof(consume));
    }
    // player:addItem precedes both remove calls in all three donor scripts. A full or
    // overweight bag drops the output even when the later debit frees space/capacity.
    let received_weight = carried
        .checked_add(u64::from(output_weight))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (transaction, event, output_id, occurred): (String,String,String,i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut ***tx).await?;
    let credit = if received_weight <= capacity {
        stack::find(tx, &context, &output_definition).await?
    } else {
        None
    };
    let container = if credit.is_none() && received_weight <= capacity {
        choose_container(tx, &context).await?
    } else {
        None
    };
    let (delivery, ordinal) = if let Some(credit) = &credit {
        (json!({"kind":"stack"}), credit.ordinal.clone())
    } else if let Some((parent, nested, ordinal)) = container {
        (
            json!({"kind":"container","parent":hex(&parent),"nested":nested}),
            ordinal,
        )
    } else {
        let Some(mut ground) = request.ground.clone() else {
            return Ok(UseDisposition::Rejected);
        };
        let transaction_bytes = uuid_text(&transaction)?;
        ground.corpse_ref = [ITEM_LOOP_DROP_REF_PREFIX, transaction_bytes.as_slice()].concat();
        sqlx::query("INSERT INTO game_item_loop_drop_channels(world_id,channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ON CONFLICT(world_id,channel_id) DO UPDATE SET world_id=EXCLUDED.world_id")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut ***tx).await?;
        let drops: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_item_ground_locations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND game_item_loop_drop_ref(corpse_ref) LIMIT $3) bounded")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(ITEM_LOOP_CHANNEL_DROPS_MAX).fetch_one(&mut ***tx).await?;
        if drops >= ITEM_LOOP_CHANNEL_DROPS_MAX {
            return Ok(UseDisposition::NoRoom);
        }
        if let Err(error) =
            crate::durability::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
                tx,
                context.permit,
                context.recovery,
                context.node,
                (fence.runtime_scope, fence.scope_ownership_generation.get()),
                &ground,
                &uuid_text(&output_id)?,
            )
            .await
        {
            return match error.into_item_mint_error() {
                ItemMintError::CapacityExceeded => Ok(UseDisposition::NoRoom),
                ItemMintError::AuthorityRejected | ItemMintError::InvalidInput => {
                    Ok(UseDisposition::Rejected)
                }
                ItemMintError::Unavailable(error) => Err(error),
                _ => Err(DurabilityError::InvalidStoredState),
            };
        }
        (
            json!({"kind":"ground","position":hex(&ground.spatial_position),"corpse_ref":hex(&ground.corpse_ref),
            "map_revision":ground.map_revision,"content_revision":ground.content_revision,"context":hex(&ground.native_room_placement_context)}),
            "0".to_owned(),
        )
    };
    let mut proof = json!({"inputs":proof_inputs,"delivery":delivery,"channel":hex(channel.as_bytes()),"scope_generation":fence.scope_ownership_generation.get().to_string()});
    if let Some(credit) = &credit {
        proof["credit"] = credit.proof(0);
    }
    let output_id = credit
        .as_ref()
        .map_or(output_id, |credit| hex(&credit.item));
    let outputs = vec![
        json!({"item":output_id,"key":output_definition.production_key,"revision":output_definition.revision_ref,"count":1,"ordinal":ordinal}),
    ];
    let recipe = recipe(kind);
    let audit_request = ItemCraftRequest {
        command: request.command,
        source: request.source,
        tool: Some(request.target),
        station: None,
        intent: request.intent.clone(),
        gathering: None,
        enchanting: None,
    };
    let tuple = tx.tuple();
    // The Craft operation already carries a bounded opaque proof. Preserve the old operation
    // and envelope, with both input snapshots and destination inside this recipe's before data.
    let before_proof = json!({"primary_item":source.before,"soul_core":proof});
    let envelope = audit::encode(
        fence,
        &audit_request,
        recipe,
        context.native.source_digest(),
        &before_proof,
        &outputs,
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,soul_core_proof) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15),true,$16)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(recipe)
        .bind(request.source.as_slice()).bind(source.parent.as_slice()).bind(&source.ordinal).bind(&source.before)
        .bind(json!(outputs)).bind(occurred).bind(&envelope).bind(&proof).execute(&mut ***tx).await?;
    for input in &inputs {
        let consume = consumption(input.item, request, plan);
        if consume == 0 {
            continue;
        }
        let added = u32::from(
            credit
                .as_ref()
                .is_some_and(|credit| credit.item == input.item),
        );
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2+$4,lifecycle=CASE WHEN quantity-$2+$4=0 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(input.item.as_slice()).bind(i64::from(consume)).bind(&transaction).bind(i64::from(added)).execute(&mut ***tx).await?;
        if input.quantity + added == consume {
            let query = if input.nested {
                "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
            } else {
                "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
            };
            sqlx::query(query)
                .bind(input.item.as_slice())
                .execute(&mut ***tx)
                .await?;
        }
    }
    if let Some(credit) = &credit {
        if !inputs
            .iter()
            .any(|input| input.item == credit.item && consumption(input.item, request, plan) > 0)
        {
            sqlx::query("UPDATE game_item_instances SET quantity=quantity+1,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(credit.item.as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        }
    } else {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,$5::uuid)")
            .bind(&output_id).bind(world.as_bytes().as_slice()).bind(&output_definition.production_key).bind(&output_definition.revision_ref).bind(&transaction).execute(&mut ***tx).await?;
        place_output(tx, fence, &delivery, &output_id, &ordinal, &transaction).await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(request.source.as_slice())
        .bind(occurred).bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(UseDisposition::Committed)
}

async fn load_inputs(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &SoulCoreCraftRequest,
) -> Result<Vec<Input>, DurabilityError> {
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let rows=sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,uuid_send(entry.parent_item_instance_id) AS parent,entry.placement_ordinal::text AS ordinal,entry.nested FROM game_item_instances i JOIN (SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,false AS nested FROM game_item_container_entries WHERE character_id=encode($3,'hex')::uuid UNION ALL SELECT item_instance_id,world_id,parent_item_instance_id,placement_ordinal,true FROM game_item_nested_entries) entry USING(item_instance_id,world_id) JOIN LATERAL game_item_loop_owner(entry.parent_item_instance_id) owner ON owner.owner_character=encode($3,'hex')::uuid AND owner.depth BETWEEN 0 AND 7 WHERE i.item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND i.world_id=encode($4,'hex')::uuid AND i.lifecycle=1 ORDER BY i.item_instance_id FOR UPDATE OF i")
        .bind(request.source.as_slice()).bind(request.target.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
    rows.into_iter()
        .map(|row| {
            Ok(Input {
                item: row
                    .try_get::<Vec<u8>, _>("item")?
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                before: row.try_get("before_item")?,
                definition: TypedDefinitionRef {
                    family: row.try_get("definition_family")?,
                    production_key: row.try_get("definition_production_key")?,
                    revision_ref: row.try_get("definition_revision_ref")?,
                },
                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                parent: row
                    .try_get::<Vec<u8>, _>("parent")?
                    .try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                ordinal: row.try_get("ordinal")?,
                nested: row.try_get("nested")?,
            })
        })
        .collect()
}

async fn choose_container<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    context: &CraftContext<'_, C>,
) -> Result<Option<([u8; 16], bool, String)>, DurabilityError> {
    let rows=sqlx::query("WITH RECURSIVE tree(item,depth) AS (SELECT item_instance_id,0 FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT entry.item_instance_id,tree.depth+1 FROM tree JOIN (SELECT item_instance_id,parent_item_instance_id FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT item_instance_id,parent_item_instance_id FROM game_item_nested_entries) entry ON entry.parent_item_instance_id=tree.item WHERE tree.depth<8) SELECT uuid_send(item) AS item,depth,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM tree JOIN game_item_instances i ON i.item_instance_id=tree.item WHERE i.lifecycle=1 ORDER BY depth,item LIMIT 502")
        .bind(context.fence.character_id.as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
    let count = i64::try_from(rows.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
    if count > ITEM_LOOP_TREE_ITEMS_MAX + 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if count > ITEM_LOOP_TREE_ITEMS_MAX {
        return Ok(None);
    }
    for row in rows {
        let depth: i32 = row.try_get("depth")?;
        if depth >= 8 {
            continue;
        }
        let definition = TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        };
        let Some(capacity) = context
            .content
            .loop_facts(&definition)
            .and_then(|facts| facts.container_capacity)
        else {
            continue;
        };
        let parent: [u8; 16] = row
            .try_get::<Vec<u8>, _>("item")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let (occupied,highest):(i64,String)=sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM (SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) entries")
            .bind(parent.as_slice()).fetch_one(&mut ***tx).await?;
        if occupied >= i64::from(capacity.min(20)) {
            continue;
        }
        let ordinal = highest
            .parse::<u64>()
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(DurabilityError::InvalidStoredState)?;
        return Ok(Some((parent, depth > 0, ordinal.to_string())));
    }
    Ok(None)
}

async fn place_output(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    delivery: &Value,
    item: &str,
    ordinal: &str,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    if delivery["kind"] == "container" {
        if delivery["nested"] == true {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,($3->>'parent')::uuid,$4::text::numeric,$5::uuid)")
                .bind(item).bind(world.as_bytes().as_slice()).bind(delivery).bind(ordinal).bind(transaction).execute(&mut ***tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,($4->>'parent')::uuid,$5::text::numeric,$6::uuid)")
                .bind(item).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(delivery).bind(ordinal).bind(transaction).execute(&mut ***tx).await?;
        }
    } else {
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,decode($5->>'position','hex'),decode($5->>'corpse_ref','hex'),$5->>'map_revision',$5->>'content_revision',decode($5->>'context','hex'))")
            .bind(item).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).bind(delivery).execute(&mut ***tx).await?;
    }
    Ok(())
}

fn native_source_id(native: &NativeGameplayState, definition: &TypedDefinitionRef) -> Option<u32> {
    let record = native
        .item_policy(&definition.production_key, &definition.revision_ref)?
        .record();
    (record.production_definition.family == definition.family
        && record.production_definition.production_key == definition.production_key
        && record.production_definition.revision_ref == definition.revision_ref)
        .then_some(())?;
    record.production_binding.external_id.parse().ok()
}

fn definition_for_source(native: &NativeGameplayState, id: u32) -> Option<TypedDefinitionRef> {
    let record = native.item_policy_for_source_id(id)?.record();
    (record.admission.materializable && record.production_definition.family == "Item").then(|| {
        TypedDefinitionRef {
            family: record.production_definition.family.clone(),
            production_key: record.production_definition.production_key.clone(),
            revision_ref: record.production_definition.revision_ref.clone(),
        }
    })
}

fn conversion_kind(source: u32, target: u32) -> Option<ConversionKind> {
    if !is_fusible_soul_core(target) {
        return None;
    }
    match source {
        SOUL_PRISM_SOURCE_ID => Some(ConversionKind::Prism),
        EXALTED_CORE_SOURCE_ID => Some(ConversionKind::Exalted),
        value if is_fusible_soul_core(value) => Some(ConversionKind::Fuse),
        _ => None,
    }
}

fn monster_for_core(core: u32) -> Option<&'static SourceMonsterFacts> {
    let name = soul_core_name(core).and_then(soul_core_monster_name)?;
    source_monster(name)
}

fn monster_record_for_name<'a>(
    profiles: &'a CreatureProfilesDocument,
    name: &str,
) -> Option<(&'a str, &'a ProjectV2CreatureAuthoring)> {
    let mut matches = profiles.records.iter().filter_map(|record| {
        let key = record.profile.target.key.as_str();
        if !key.starts_with("oteryn:creature.") {
            return None;
        }
        // Crystal registers "Day Night Harpy" separately, then changes its display name
        // to "Night Harpy". MonsterType("night harpy") uses the registration map.
        // winter_update_2025/{night_harpy,day_night_harpy}.lua:1-4;
        // monster_type_functions.cpp:196-203, monsters.cpp:354-360.
        if name.eq_ignore_ascii_case("night harpy") && key != "oteryn:creature.night_harpy" {
            return None;
        }
        let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
            return None;
        };
        profile
            .details
            .as_ref()?
            .display_name
            .eq_ignore_ascii_case(name)
            .then_some((key, profile))
    });
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn selected_output(
    native: &NativeGameplayState,
    kind: ConversionKind,
    target: u32,
    draw: u64,
) -> Option<u32> {
    if kind == ConversionKind::Fuse {
        let last = i64::try_from(SOUL_CORE_SOURCE_IDS.len().checked_sub(1)?).ok()?;
        return SOUL_CORE_SOURCE_IDS
            .get(usize::try_from(uniform_draw(draw, 0, last)).ok()?)
            .copied();
    }
    let difficulty = conversion_difficulty(kind, monster_for_core(target)?.stars)?;
    // ANSWER6543406061: source registration/difficulty controls the full draw.
    // Missing native Creature profiles do not remove candidates, and Item admission
    // happens only after selection. A missing core never causes another draw.
    let candidates: Vec<_> = source_bestiary_candidates()
        .filter(|candidate| candidate.stars == difficulty)
        .collect();
    let last = i64::try_from(candidates.len().checked_sub(1)?).ok()?;
    let candidate = candidates.get(usize::try_from(uniform_draw(draw, 0, last)).ok()?)?;
    let id = soul_core_for_monster(candidate.name)?;
    (is_soul_core_output(id) && native.item_policy_for_source_id(id).is_some()).then_some(id)
}

fn consumption(item: [u8; 16], request: &SoulCoreCraftRequest, plan: ConversionPlan) -> u32 {
    u32::from(item == request.source) * plan.consume_source
        + u32::from(item == request.target) * plan.consume_target
}

fn recipe(kind: ConversionKind) -> &'static str {
    match kind {
        ConversionKind::Fuse => "oteryn:soulpit.fuse",
        ConversionKind::Prism => "oteryn:soulpit.prism",
        ConversionKind::Exalted => "oteryn:soulpit.exalted",
    }
}

fn command_draw(request: &SoulCoreCraftRequest, purpose: &[u8]) -> u64 {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:soulpit:craft:v1");
    digest.update(purpose);
    digest.update(request.command.game_session_id().as_bytes());
    digest.update(request.command.command_id().get().to_be_bytes());
    digest.update(request.source);
    digest.update(request.target);
    let digest = digest.finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{CommandId, GameSessionId};

    fn request() -> Result<SoulCoreCraftRequest, Box<dyn std::error::Error>> {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        Ok(SoulCoreCraftRequest {
            command: CommandRef::new(GameSessionId::decode(&id)?, CommandId::new(1)?),
            source: [2; 16],
            target: [3; 16],
            intent: vec![1],
            ground: None,
        })
    }

    #[test]
    fn core_selection_is_repeatable_and_separates_decisions()
    -> Result<(), Box<dyn std::error::Error>> {
        let request = request()?;
        assert_eq!(
            command_draw(&request, b"species"),
            command_draw(&request, b"species")
        );
        assert_ne!(
            command_draw(&request, b"species"),
            command_draw(&request, b"ominous")
        );
        let mut other = request.clone();
        other.target = [4; 16];
        assert_ne!(
            command_draw(&request, b"species"),
            command_draw(&other, b"species")
        );
        Ok(())
    }

    #[test]
    fn one_stack_fusion_combines_both_debits_and_ominous_keeps_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut request = request()?;
        let fusion = ConversionPlan {
            output_item_id: 49163,
            consume_source: 1,
            consume_target: 1,
        };
        assert_eq!(consumption(request.source, &request, fusion), 1);
        request.target = request.source;
        assert_eq!(consumption(request.source, &request, fusion), 2);
        request.target = [3; 16];
        let ominous = ConversionPlan {
            output_item_id: 49163,
            consume_source: 1,
            consume_target: 0,
        };
        assert_eq!(consumption(request.source, &request, ominous), 1);
        assert_eq!(consumption(request.target, &request, ominous), 0);
        Ok(())
    }

    #[test]
    fn output_catalogue_membership_does_not_authorize_non_core_name_inputs() {
        assert!(!is_fusible_soul_core(SOUL_PRISM_SOURCE_ID));
        assert!(!is_fusible_soul_core(49_038)); // Crystal's exact name is "white lion".
        assert!(is_fusible_soul_core(49_163));
        assert_eq!(conversion_kind(49_163, SOUL_PRISM_SOURCE_ID), None);
        assert_eq!(conversion_kind(49_038, 49_163), None);
        assert_eq!(
            conversion_kind(SOUL_PRISM_SOURCE_ID, 49_163),
            Some(ConversionKind::Prism)
        );
    }

    #[test]
    fn native_entry_catalogue_preserves_registration_lookup_and_compatibility_boundaries()
    -> Result<(), Box<dyn std::error::Error>> {
        let profiles: CreatureProfilesDocument = serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/creatures/definitions/spell-native-profiles.json"
        )))?;
        // The real content has two identically displayed Harpies. Source registration,
        // rather than first-record order or display-name ambiguity, resolves the core.
        let harpies = profiles
            .records
            .iter()
            .filter(|record| match &record.profile.data {
                ProjectV2AuthoringProfileData::Creature(profile) => profile
                    .details
                    .as_ref()
                    .is_some_and(|details| details.display_name == "Night Harpy"),
                _ => false,
            })
            .count();
        assert_eq!(harpies, 2);
        let name = crate::domain::soulpit::soulpit_entry_monster_name(52_813)
            .ok_or("missing Night Harpy core")?;
        assert_eq!(
            monster_record_for_name(&profiles, &name).map(|(key, _)| key),
            Some("oteryn:creature.night_harpy")
        );
        assert_eq!(
            monster_record_for_name(&profiles, "rat").map(|(key, _)| key),
            Some("oteryn:creature.rat")
        );
        // Cake Golem is a source conversion target/candidate, while entry still needs
        // the native Creature owner. Metadata capture cannot authorize a new actor.
        assert!(monster_record_for_name(&profiles, "Cake Golem").is_none());
        assert_eq!(
            monster_for_core(47_691).map(|monster| monster.stars),
            Some(2)
        );
        assert_eq!(
            monster_for_core(47_984).map(|monster| monster.stars),
            Some(4)
        );
        Ok(())
    }
}
