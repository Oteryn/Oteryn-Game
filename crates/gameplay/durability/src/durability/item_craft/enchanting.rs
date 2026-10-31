//! I19 extends the existing craft receipt and fenced consume/mint owner.
use super::super::character_last_position::{
    LastPositionKey, LastPositionRecord, LastPositionWrite,
};
use super::*;
use oteryn_gameplay_items::item_craft::enchanting as rules;

#[derive(Debug, Clone)]
pub struct EnchantingCraft {
    pub target: Option<[u8; 16]>,
    pub cost: Option<super::super::spell_items_abi::CastCostBinding>,
    pub save: Option<(LastPositionKey, LastPositionRecord)>,
}

fn definition(before: &Value) -> Option<TypedDefinitionRef> {
    Some(TypedDefinitionRef {
        family: before["definition_family"].as_str()?.into(),
        production_key: before["definition_production_key"].as_str()?.into(),
        revision_ref: before["definition_revision_ref"].as_str()?.into(),
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "the existing craft transaction owns both input debits, output, resource save and audit"
)]
pub(super) async fn write<C: ItemLoopContent>(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    content: &C,
) -> Result<UseDisposition, DurabilityError> {
    let Some(enchanting) = &request.enchanting else {
        return Ok(UseDisposition::Rejected);
    };
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut inputs = vec![request.source.to_vec()];
    if let Some(target) = enchanting.target {
        if target == request.source {
            return Ok(UseDisposition::NothingToUse);
        }
        inputs.push(target.to_vec());
    }
    let rows = sqlx::query("SELECT to_jsonb(i) AS before_item,i.item_instance_id::text AS id,uuid_send(i.item_instance_id) AS bytes,uuid_send(e.parent_item_instance_id) AS parent,e.placement_ordinal::text AS ordinal,e.parent_item_instance_id::text AS parent_text FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE uuid_send(i.item_instance_id)=ANY($1) AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND e.world_id=i.world_id AND b.world_id=i.world_id ORDER BY i.item_instance_id FOR UPDATE OF i")
        .bind(&inputs).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
    if rows.len() != inputs.len() {
        return Ok(UseDisposition::StaleState);
    }
    let source = rows
        .iter()
        .find(|r| r.get::<Vec<u8>, _>("bytes") == request.source)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let before: Value = source.try_get("before_item")?;
    let Some(source_definition) = definition(&before) else {
        return Ok(UseDisposition::Rejected);
    };
    let parent: Vec<u8> = source.try_get("parent")?;
    let target = enchanting
        .target
        .and_then(|target| rows.iter().find(|r| r.get::<Vec<u8>, _>("bytes") == target));
    let target_before = target
        .map(|r| r.try_get::<Value, _>("before_item"))
        .transpose()?;
    let Some(station) = request.station.as_ref() else {
        return Ok(UseDisposition::NothingToUse);
    };
    if target_before
        .as_ref()
        .is_some_and(|v| definition(v).as_ref() != Some(station))
    {
        return Ok(UseDisposition::StaleState);
    }
    let ids = rules::source_id(&source_definition.production_key)
        .zip(rules::source_id(&station.production_key));
    let count = before["quantity"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok());
    let target_count = target_before.as_ref().map_or(Some(1), |v| {
        v["quantity"].as_u64().and_then(|n| u32::try_from(n).ok())
    });
    let Some(rule) = ids
        .zip(count.zip(target_count))
        .and_then(|((s, t), (n, m))| rules::rule(s, t, n, m))
    else {
        return Ok(UseDisposition::NothingToUse);
    };
    if (rule.mana > 0) != enchanting.target.is_none() {
        return Ok(UseDisposition::NothingToUse);
    }
    let Some(output) = rules::output(native, rule) else {
        return Ok(UseDisposition::NotPossible);
    };
    for typed in [&source_definition, station] {
        if native
            .item_policy(&typed.production_key, &typed.revision_ref)
            .is_none()
        {
            return Ok(UseDisposition::StaleState);
        }
    }
    let cost = if rule.mana > 0 {
        let Some(cost) = enchanting.cost.as_ref() else {
            return Ok(UseDisposition::Rejected);
        };
        if cost.mana_before.checked_sub(rule.mana) != Some(cost.mana_after)
            || cost.soul_before.checked_sub(rule.soul) != Some(cost.soul_after)
            || enchanting.save.is_none()
        {
            return Ok(UseDisposition::Rejected);
        }
        let bytes = super::super::spell_item_transaction::encode_cast_cost(cost)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        serde_json::from_slice::<Value>(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?
    } else {
        if enchanting.cost.is_some() || enchanting.save.is_some() {
            return Ok(UseDisposition::Rejected);
        }
        Value::Null
    };
    let mut debits = vec![
        json!({"before":before,"count":rule.source_debit,"parent":source.try_get::<String,_>("parent_text")?,"ordinal":source.try_get::<String,_>("ordinal")?}),
    ];
    if let (Some(target), Some(target_before)) = (target, target_before) {
        debits.push(json!({"before":target_before,"count":rule.count,"parent":target.try_get::<String,_>("parent_text")?,"ordinal":target.try_get::<String,_>("ordinal")?}));
    }
    let consumed_weight = debits.iter().try_fold(0_u64, |sum, d| {
        let facts = content.loop_facts(&definition(&d["before"])?)?;
        sum.checked_add(u64::from(facts.weight?) * d["count"].as_u64()?)
    });
    let output_facts = content.loop_facts(&output);
    let Some((consumed_weight, output_weight)) = consumed_weight.zip(
        output_facts
            .as_ref()
            .and_then(|f| f.weight)
            .map(|w| u64::from(w) * u64::from(rule.count)),
    ) else {
        return Ok(UseDisposition::NotPossible);
    };
    if output_facts
        .and_then(|f| stack_maximum(f.stack).ok())
        .is_none_or(|(max, _)| rule.count > max)
    {
        return Ok(UseDisposition::NotPossible);
    }
    let carried = carried_weight(tx, fence.character_id, *world.as_bytes(), content)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(after_weight) = carried
        .checked_sub(consumed_weight)
        .and_then(|w| w.checked_add(output_weight))
    else {
        return Ok(UseDisposition::NotPossible);
    };
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id).await?;
    if after_weight > capacity {
        return Ok(UseDisposition::NotPossible);
    }
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let released = debits
        .iter()
        .filter(|d| d["before"]["quantity"] == d["count"] && d["parent"] == debits[0]["parent"])
        .count();
    let bag = sqlx::query_scalar::<_,Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE").bind(&parent).fetch_one(&mut ***tx).await?;
    let slots = definition(&bag)
        .and_then(|d| content.loop_facts(&d))
        .and_then(|f| f.container_capacity)
        .unwrap_or(0)
        .min(20);
    if occupied + 1 - i64::try_from(released).map_err(|_| DurabilityError::InvalidStoredState)?
        > i64::from(slots)
    {
        return Ok(UseDisposition::NoRoom);
    }
    let highest: String = sqlx::query_scalar("SELECT coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let ordinal = highest
        .parse::<u64>()
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (item,transaction,event,occurred):(String,String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let outputs = vec![
        json!({"item":item,"key":output.production_key,"revision":output.revision_ref,"count":rule.count,"ordinal":ordinal.to_string(),"charges":rule.charges}),
    ];
    let recipe = if rule.mana > 0 {
        "oteryn:enchanting.shrine"
    } else {
        "oteryn:enchanting.weapon"
    };
    let proof = json!({"debits":debits,"cost":cost,
        "shrine":request.station.as_ref().and_then(|s|rules::source_id(&s.production_key)),
        "save_sequence":enchanting.save.as_ref().map(|(key,_)|key.write_sequence),
        "lease_generation":fence.character_lease_generation});
    let envelope = audit::encode(
        fence,
        request,
        recipe,
        native.source_digest(),
        &proof,
        &outputs,
        &transaction,
        &event,
        occurred,
        tx.tuple().tuple(),
    )?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,enchanting) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15),$16)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(recipe).bind(request.source.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).bind(&proof).execute(&mut ***tx).await?;
    for debit in &debits {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=($1->>'item_instance_id')::uuid")
            .bind(&debit["before"]).bind(debit["count"].as_i64().ok_or(DurabilityError::InvalidStoredState)?).bind(&transaction).execute(&mut ***tx).await?;
        if debit["before"]["quantity"] == debit["count"] {
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item_instance_id')::uuid").bind(&debit["before"]).execute(&mut ***tx).await?;
        }
    }
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,$6::uuid)")
        .bind(&item).bind(world.as_bytes().as_slice()).bind(&output.production_key).bind(&output.revision_ref).bind(i64::from(rule.count)).bind(&transaction).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::uuid)")
        .bind(&item).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(ordinal.to_string()).bind(&transaction).execute(&mut ***tx).await?;
    if let Some(charges) = rule.charges {
        sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES($1::uuid,$2,1)").bind(&item).bind(i32::from(charges)).execute(&mut ***tx).await?;
    }
    if let Some((key, record)) = &enchanting.save {
        if record.vitals.map(|v| v.mana()) != enchanting.cost.as_ref().map(|c| c.mana_after)
            || record.rest.map(|r| r[0]) != enchanting.cost.as_ref().map(|c| c.soul_after)
        {
            return Ok(UseDisposition::Rejected);
        }
        if super::super::character_last_position::upsert_last_position(
            tx,
            fence.character_id.as_bytes(),
            *key,
            record,
        )
        .await?
            != LastPositionWrite::Written
        {
            return Ok(UseDisposition::StaleState);
        }
    }
    let tuple = tx.tuple().tuple();
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(request.source.as_slice()).bind(occurred).bind(super::super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    Ok(UseDisposition::Committed)
}
