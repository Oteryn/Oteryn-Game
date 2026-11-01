//! Atelier BURN/MINT lines use the ordinary ItemInstance and main backpack owners.
use super::super::item_loop_move::carried_weight;
use super::super::item_mint::TypedDefinitionRef;
use super::super::item_mint_audit::{self as audit, Type2Transaction};
use super::*;
use prost::Message;
use serde_json::{Value, json};

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemAtelierV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub proof: Vec<u8>,
}
/// Only these Character-owned actions may create Atelier item lines.
enum GemAtelierCause {
    Reveal,
    Dismantle,
    GradeUp,
}
impl GemAtelierCause {
    fn from_mutation(mutation: &AtelierMutation) -> std::result::Result<Self, DurabilityError> {
        match mutation {
            AtelierMutation::Reveal { .. } => Ok(Self::Reveal),
            AtelierMutation::Dismantle { .. } => Ok(Self::Dismantle),
            AtelierMutation::GradeUp { .. } => Ok(Self::GradeUp),
            _ => Err(DurabilityError::InvalidStoredState),
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Self::Reveal => "REVEAL",
            Self::Dismantle => "DISMANTLE",
            Self::GradeUp => "GRADE_UP",
        }
    }
}

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &AtelierRequest<'_>,
    resources: &AtelierResources<'_>,
    identity: &actions::TransactionIdentity,
    burn: Option<(&str, u32)>,
    mint: Option<(&str, u32)>,
) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
    if burn.is_none() && mint.is_none() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let cause = GemAtelierCause::from_mutation(&request.mutation)?;
    let character = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let Some(bag)=sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF b,i")
        .bind(character).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await? else { return Ok(None); };
    let parent: String = bag.try_get("item_instance_id")?;
    let bag_definition = TypedDefinitionRef {
        family: bag.try_get("definition_family")?,
        production_key: bag.try_get("definition_production_key")?,
        revision_ref: bag.try_get("definition_revision_ref")?,
    };
    let Some(bag_facts) = resources.content.loop_facts(&bag_definition) else {
        return Ok(None);
    };
    let capacity = bag_facts.container_capacity.unwrap_or(0).min(20);
    let mut inputs = Vec::new();
    let mut burned_weight = 0_u64;
    if let Some((key, mut needed)) = burn {
        // B3 display order; all selected leaves are locked before the first write.
        let rows=sqlx::query("SELECT to_jsonb(i) AS before,i.item_instance_id::text,i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,e.placement_ordinal::text FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=$2::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$4 AND i.quantity BETWEEN 1 AND 100 AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id=i.item_instance_id) ORDER BY e.placement_ordinal DESC LIMIT 20 FOR UPDATE OF e,i")
            .bind(character).bind(&parent).bind(world.as_bytes().as_slice()).bind(key).fetch_all(&mut ***tx).await?;
        for row in rows {
            if needed == 0 {
                break;
            }
            let definition = TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            };
            let Some(facts) = resources.content.loop_facts(&definition) else {
                return Ok(None);
            };
            let Some(weight) = facts.weight else {
                return Ok(None);
            };
            if super::super::item_transfer::stack_maximum(facts.stack).map(|(n, _)| n) != Ok(100) {
                return Ok(None);
            }
            let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let count = needed.min(quantity);
            needed -= count;
            burned_weight += u64::from(weight) * u64::from(count);
            inputs.push(json!({"item":row.try_get::<String,_>("item_instance_id")?,"ordinal":row.try_get::<String,_>("placement_ordinal")?,"before":row.try_get::<Value,_>("before")?,"count":count}));
        }
        if needed != 0 {
            return Ok(None);
        }
    }
    let transaction = &identity.transaction;
    let event = &identity.event;
    let now = identity.now;
    let mut outputs = Vec::new();
    if let Some((key, count)) = mint {
        if resources.fragment.family != "Item"
            || resources.fragment.production_key != key
            // Crystal 201189f player_wheel.cpp:1308-1318 permits ten regular fragments;
            // the progression owner and deferred SQL guard enforce the quality-specific range.
            || !(1..=10).contains(&count)
        {
            return Ok(None);
        }
        let Some(facts) = resources.content.loop_facts(&resources.fragment) else {
            return Ok(None);
        };
        let Some(weight) = facts.weight else {
            return Ok(None);
        };
        if super::super::item_transfer::stack_maximum(facts.stack).map(|(n, _)| n) != Ok(100) {
            return Ok(None);
        }
        let occupied:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=$1::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
        let receiver=sqlx::query("SELECT to_jsonb(i) AS before,i.item_instance_id::text,e.placement_ordinal::text FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.parent_item_instance_id=$2::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$4 AND i.definition_revision_ref=$5 AND i.quantity BETWEEN 1 AND 100-$6 AND NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_texts t WHERE t.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries n WHERE n.parent_item_instance_id=i.item_instance_id) ORDER BY e.placement_ordinal DESC LIMIT 1 FOR UPDATE OF e,i")
            .bind(character).bind(&parent).bind(world.as_bytes().as_slice()).bind(key).bind(&resources.fragment.revision_ref).bind(i64::from(count)).fetch_optional(&mut ***tx).await?;
        if receiver.is_none() && occupied >= i64::from(capacity) {
            return Ok(None);
        }
        let carried = match carried_weight(
            tx,
            fence.character_id,
            *world.as_bytes(),
            resources.content,
        )
        .await
        {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let limit = super::super::item_transfer::read_derived_transfer_capacity_centi_oz(
            tx,
            fence.character_id,
            resources.content,
        )
        .await?;
        if carried
            .checked_sub(burned_weight)
            .and_then(|v| v.checked_add(u64::from(weight) * u64::from(count)))
            .is_none_or(|v| v > limit)
        {
            return Ok(None);
        }
        if let Some(row) = receiver {
            outputs.push(json!({"item":row.try_get::<String,_>("item_instance_id")?,"ordinal":row.try_get::<String,_>("placement_ordinal")?,"key":key,"revision":resources.fragment.revision_ref,"count":count,"before":row.try_get::<Value,_>("before")?}));
        } else {
            let (item,ordinal):(String,String)=sqlx::query_as("SELECT game_character_uuid_v7()::text,(coalesce(max(placement_ordinal),0)+1)::text FROM game_item_container_entries WHERE parent_item_instance_id=$1::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
            outputs.push(json!({"item":item,"ordinal":ordinal,"key":key,"revision":resources.fragment.revision_ref,"count":count}));
        }
    }
    if !identity.paid {
        audit::TransactionResourceUsage {
            touched_item_instances: (inputs.len() + outputs.len()) as u64,
            location_custody_lines: (inputs.len() + outputs.len()) as u64,
            value_lines: 0,
            transform_lines: 0,
            container_expansion: 0,
            participants: (inputs.len() + outputs.len()) as u64,
            effect_work_units: ((inputs.len() + outputs.len()) * 2) as u64,
            events: 1,
        }
        .check()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    // SQL retains the full locked snapshot. Audit carries the plain leaf's bounded semantic
    // state, so twenty fragment inputs still fit the existing fee envelope ceiling.
    let audit_inputs:Vec<Value>=inputs.iter().map(|line| {
        let before=&line["before"];
        let quantity=before["quantity"].as_u64().ok_or(DurabilityError::InvalidStoredState)?;
        let count=line["count"].as_u64().ok_or(DurabilityError::InvalidStoredState)?;
        let after=quantity.checked_sub(count).ok_or(DurabilityError::InvalidStoredState)?;
        Ok(json!({"item":line["item"],"ordinal":line["ordinal"],"count":line["count"],
            "before":{"family":before["definition_family"],"key":before["definition_production_key"],"revision":before["definition_revision_ref"],"quantity":before["quantity"],"state_revision":before["state_revision"]},
            "quantity_after":after}))
    }).collect::<std::result::Result<_,DurabilityError>>()?;
    let proof = json!({"character":fence.character_id.as_bytes(),"occurrence":request.occurrence,"mutation":request.mutation,"character_lease_generation":fence.character_lease_generation,"scope_ownership_generation":fence.scope_ownership_generation.get(),"content_digest":resources.content.loop_content_digest(),"inputs":audit_inputs,"outputs":outputs,"parent":parent});
    let proof = serde_json::to_vec(&proof).map_err(|_| DurabilityError::InvalidStoredState)?;
    let tuple = tx.tuple();
    let payload = audit::encode_bounded(
        &audit::OneItemTransactionV1 {
            interpretation_revision: audit::INTERPRETATION_REVISION,
            operation: Some(audit::OneItemOperationV1::Atelier(OneItemAtelierV1 {
                proof: proof.clone(),
            })),
        },
        audit::RL07_PAYLOAD_BYTES_MAX,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let envelope = audit::EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: uuid_text(event)?.to_vec(),
        event_type_id: audit::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: audit::DURABLE_AUDIT,
        privacy_class: audit::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: now,
        world_id: Some(world.as_bytes().to_vec()),
        channel_id: match fence.runtime_scope {
            crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => {
                Some(channel_id.as_bytes().to_vec())
            }
            crate::foundation::RuntimeScopeRefV1::Instance { .. } => None,
        },
        game_session_id: Some(fence.game_session_id.as_bytes().to_vec()),
        connection_generation: Some(fence.connection_generation.get()),
        transaction_event: Some(audit::TransactionEventRefV1 {
            transaction_id: uuid_text(transaction)?.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(audit::CausationRefV1 {
            cause: Some(audit::CausationV1::OperationId(request.occurrence.to_vec())),
        }),
        server_build_id: super::super::character_authority::SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..Default::default()
    };
    let wire = audit::encode_bounded(&envelope, audit::RL07_ENVELOPE_BYTES_MAX)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    audit::decode_common_envelope(&wire).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_atelier_item_records(transaction_id,event_id,character_id,world_id,occurrence,cause,parent_item_instance_id,inputs,outputs,occurred_at,envelope_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7::uuid,$8,$9,$10,sha256($11))")
        .bind(transaction).bind(event).bind(character).bind(world.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(cause.as_str()).bind(&parent).bind(json!(inputs)).bind(json!(outputs)).bind(now).bind(if identity.paid {None} else {Some(wire.as_slice())}).execute(&mut ***tx).await?;
    for input in &inputs {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-($1->>'count')::bigint,lifecycle=CASE WHEN quantity=($1->>'count')::bigint THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid").bind(input).bind(transaction).execute(&mut ***tx).await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid AND ($1->'before'->>'quantity')::bigint=($1->>'count')::bigint").bind(input).execute(&mut ***tx).await?;
    }
    for output in &outputs {
        if output.get("before").is_some() {
            sqlx::query("UPDATE game_item_instances SET quantity=quantity+($1->>'count')::bigint,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid").bind(output).bind(transaction).execute(&mut ***tx).await?;
            continue;
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)").bind(output).bind(world.as_bytes().as_slice()).bind(transaction).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::uuid,($1->>'ordinal')::numeric,$5::uuid)").bind(output).bind(world.as_bytes().as_slice()).bind(character).bind(&parent).bind(transaction).execute(&mut ***tx).await?;
    }
    if !identity.paid {
        let anchor = inputs
            .first()
            .or(outputs.first())
            .and_then(|v| v["item"].as_str())
            .ok_or(DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)").bind(event).bind(transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(anchor).bind(now).bind(audit::AUDIT_RETENTION_P90D_MS).bind(wire).execute(&mut ***tx).await?;
    }
    Ok(Some(proof))
}
