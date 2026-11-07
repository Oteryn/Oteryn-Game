//! Atelier BURN/MINT lines use the existing carried custody and Supply Stash owners.
use super::super::item_loop_move::carried_weight;
use super::super::item_mint::TypedDefinitionRef;
use super::super::item_mint_audit::{self as audit, Type2Transaction};
use super::*;
use prost::Message;
use serde_json::{Value, json};

#[path = "materials.rs"]
mod materials;
#[path = "outputs.rs"]
mod outputs;
use super::super::item_fee_burn_audit::atelier_cause as atelier_audit;

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
    let bag=sqlx::query("SELECT i.item_instance_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF b,i")
        .bind(character).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let parent: Option<String> = bag
        .as_ref()
        .map(|row| row.try_get("item_instance_id"))
        .transpose()?;
    let planned = if let Some((key, needed)) = burn {
        let Some(plan) = materials::plan(tx, fence, request, resources, key, needed).await? else {
            return Ok(None);
        };
        plan
    } else {
        materials::MaterialPlan {
            inputs: Vec::new(),
            stash: None,
            equipment: None,
            burned_weight: 0,
        }
    };
    let materials::MaterialPlan {
        inputs,
        stash,
        equipment,
        burned_weight,
    } = planned;
    let transaction = &identity.transaction;
    let event = &identity.event;
    let now = identity.now;
    let outputs = if let Some((key, count)) = mint {
        let Some(outputs) = outputs::plan(tx, fence, resources, key, count, burned_weight).await?
        else {
            return Ok(None);
        };
        outputs
    } else {
        Vec::new()
    };
    if !identity.paid {
        atelier_audit::check_usage(&audit::TransactionResourceUsage {
            touched_item_instances: (inputs.len() + outputs.len()) as u64,
            location_custody_lines: (inputs.len() + outputs.len()) as u64,
            value_lines: 0,
            transform_lines: 0,
            container_expansion: 0,
            participants: (inputs.len() + outputs.len()) as u64,
            effect_work_units: ((inputs.len() + outputs.len()) * 2) as u64,
            events: 1,
        })
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    // SQL retains the full locked snapshot. Audit carries the plain leaf's bounded semantic
    // state; paid proofs use the fee envelope instead of the one-item mint ceiling.
    let audit_inputs:Vec<Value>=inputs.iter().map(|line| {
        let before=&line["before"];
        let quantity=before["quantity"].as_u64().ok_or(DurabilityError::InvalidStoredState)?;
        let count=line["count"].as_u64().ok_or(DurabilityError::InvalidStoredState)?;
        let after=quantity.checked_sub(count).ok_or(DurabilityError::InvalidStoredState)?;
        Ok(json!({"item":line["item"],"ordinal":line["ordinal"],"parent":line["parent"],"location":line["location"],"slot":line["slot"],"count":line["count"],
            "before":{"family":before["definition_family"],"key":before["definition_production_key"],"revision":before["definition_revision_ref"],"quantity":before["quantity"],"state_revision":before["state_revision"]},
            "quantity_after":after}))
    }).collect::<std::result::Result<_,DurabilityError>>()?;
    let proof = json!({"character":fence.character_id.as_bytes(),"occurrence":request.occurrence,"mutation":request.mutation,"character_lease_generation":fence.character_lease_generation,"scope_ownership_generation":fence.scope_ownership_generation.get(),"content_digest":resources.content.loop_content_digest(),"inputs":audit_inputs,"outputs":outputs,"parent":parent,"stash":stash});
    let proof = serde_json::to_vec(&proof).map_err(|_| DurabilityError::InvalidStoredState)?;
    let tuple = tx.tuple();
    let wire = if identity.paid {
        None
    } else {
        let payload = audit::encode_bounded(
            &audit::OneItemTransactionV1 {
                interpretation_revision: audit::INTERPRETATION_REVISION,
                operation: Some(audit::OneItemOperationV1::Atelier(OneItemAtelierV1 {
                    proof: proof.clone(),
                })),
            },
            atelier_audit::PAYLOAD_BYTES_MAX,
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
        let wire = audit::encode_bounded(&envelope, atelier_audit::ENVELOPE_BYTES_MAX)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        audit::decode_common_envelope_within(&wire, atelier_audit::ENVELOPE_BYTES_MAX)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Some(wire)
    };
    sqlx::query("INSERT INTO game_atelier_item_records(transaction_id,event_id,character_id,world_id,occurrence,cause,parent_item_instance_id,inputs,outputs,occurred_at,envelope_sha256,stash_input,equipment_before) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7::uuid,$8,$9,$10,sha256($11),$12,$13)")
        .bind(transaction).bind(event).bind(character).bind(world.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(cause.as_str()).bind(&parent).bind(json!(inputs)).bind(json!(outputs)).bind(now).bind(wire.as_deref()).bind(stash.as_ref()).bind(equipment.as_ref()).execute(&mut ***tx).await?;
    for input in &inputs {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-($1->>'count')::bigint,lifecycle=CASE WHEN quantity=($1->>'count')::bigint THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid").bind(input).bind(transaction).execute(&mut ***tx).await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid AND ($1->'before'->>'quantity')::bigint=($1->>'count')::bigint").bind(input).execute(&mut ***tx).await?;
        sqlx::query("DELETE FROM game_item_nested_entries WHERE item_instance_id=($1->>'item')::uuid AND ($1->'before'->>'quantity')::bigint=($1->>'count')::bigint").bind(input).execute(&mut ***tx).await?;
        sqlx::query("DELETE FROM game_character_equipment_slots WHERE item_instance_id=($1->>'item')::uuid AND ($1->'before'->>'quantity')::bigint=($1->>'count')::bigint").bind(input).execute(&mut ***tx).await?;
    }
    if let Some(before) = &equipment {
        sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$2::uuid WHERE character_id=encode($1,'hex')::uuid AND revision=($3->>'revision')::numeric")
            .bind(character).bind(transaction).bind(before).execute(&mut ***tx).await?;
    }
    if let Some(stash) = &stash {
        materials::apply_stash(tx, fence, transaction, stash).await?;
    }
    for output in &outputs {
        if output.get("before").is_some() {
            sqlx::query("UPDATE game_item_instances SET quantity=quantity+($1->>'count')::bigint,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid").bind(output).bind(transaction).execute(&mut ***tx).await?;
            continue;
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)").bind(output).bind(world.as_bytes().as_slice()).bind(transaction).execute(&mut ***tx).await?;
        if output["location"] == "main" {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,($1->>'parent')::uuid,($1->>'ordinal')::numeric,$4::uuid)").bind(output).bind(world.as_bytes().as_slice()).bind(character).bind(transaction).execute(&mut ***tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,($1->>'parent')::uuid,($1->>'ordinal')::numeric,$3::uuid)").bind(output).bind(world.as_bytes().as_slice()).bind(transaction).execute(&mut ***tx).await?;
        }
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
