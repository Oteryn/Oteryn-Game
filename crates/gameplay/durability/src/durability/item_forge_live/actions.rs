//! Rule plans and one atomic native item/dust/fee composition.
use super::*;
use crate::durability::item_fee_burn::{
    FeeBurnCause, FeeBurnError, FeeBurnRequest, FeeChangeFacts,
};
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_items::forge::{EXALTED_CORE_KEY, FORGE_SLIVER_KEY};

pub(super) struct Effects {
    pub reply: Value,
    pub inputs: Vec<Value>,
    pub outputs: Vec<Value>,
    pub stash: Option<Value>,
    pub dust: u32,
    pub gold: u64,
    pub dust_before: ForgeDustBalance,
}

fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::QueryHistory => "history",
        Operation::Fusion => "fusion",
        Operation::ConvergenceFusion => "convergence_fusion",
        Operation::Transfer => "transfer",
        Operation::ConvergenceTransfer => "convergence_transfer",
        Operation::DustToSlivers => "dust_to_slivers",
        Operation::SliversToCores => "slivers_to_cores",
        Operation::DustLimit => "dust_limit",
    }
}

pub(super) fn refusal(request: Request, reason: &str) -> Value {
    json!({"operation":operation_name(request.operation),"succeeded":false,"applied":false,
        "reason":reason,"bonus":0,"result":null})
}

#[allow(clippy::too_many_lines)]
pub(super) async fn plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    request: Request,
    reserved: &storage::Reservation,
    native: &NativeGameplayState,
    inventory: &inventory::Inventory,
) -> NativeResult<std::result::Result<Effects, &'static str>> {
    let before = lock_limit_balance(tx, fence.character_id)
        .await?
        .map_or(ForgeDustBalance::default(), |(balance, _)| balance);
    let Some(core) = inventory::current_definition(native, EXALTED_CORE_KEY) else {
        return Ok(Err("UNSUPPORTED_ITEM"));
    };
    let Some(sliver) = inventory::current_definition(native, FORGE_SLIVER_KEY) else {
        return Ok(Err("UNSUPPORTED_ITEM"));
    };
    let cores = inventory::material_count(tx, fence, inventory, &core).await?;
    let slivers = inventory::material_count(tx, fence, inventory, &sliver).await?;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut stash = None;
    let (
        success,
        bonus,
        gold,
        dust,
        spent_cores,
        spent_slivers,
        new_slivers,
        new_cores,
        first,
        second,
    ) = match request.operation {
        Operation::Fusion
        | Operation::ConvergenceFusion
        | Operation::Transfer
        | Operation::ConvergenceTransfer => {
            let Some(resolved) = reserved.resolved_items else {
                return Ok(Err("INVALID_ITEM_HANDLE"));
            };
            let Some(target) = inventory
                .items
                .iter()
                .find(|item| item.id == resolved.first)
            else {
                return Ok(Err("WRONG_ITEMS"));
            };
            let Some(source) = inventory
                .items
                .iter()
                .find(|item| item.id == resolved.second)
            else {
                return Ok(Err("WRONG_ITEMS"));
            };
            let (Some(a), Some(b)) = (target.forge(native), source.forge(native)) else {
                return Ok(Err("WRONG_ITEMS"));
            };
            let operation = match request.operation {
                Operation::Fusion => ForgeOperation::Fusion {
                    boost: request.boost,
                    protect: request.protect,
                },
                Operation::ConvergenceFusion => ForgeOperation::ConvergenceFusion,
                Operation::Transfer => ForgeOperation::Transfer,
                _ => ForgeOperation::ConvergenceTransfer,
            };
            let quote = match ForgeQuote::try_from(ForgeRequest {
                operation,
                target: a,
                source: b,
            }) {
                Ok(quote) => quote,
                Err(_) => return Ok(Err("WRONG_ITEMS")),
            };
            // PR2785 Q3: native tier changes preserve identity and create no chest,
            // so the donor chest-only free-slot check does not apply to this pair.
            // The established fee owner performs the authoritative carried-coin/bank quote.
            // This rule projection validates dust, cores and the outcome before that debit.
            let plan = match ForgePlan::try_from(ForgeAttempt {
                quote,
                resources: ForgeResources {
                    gold: u64::MAX,
                    dust: before,
                    cores,
                    slivers,
                },
                success_roll: u8::try_from(reserved.rolls[0])
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                protection_roll: u8::try_from(reserved.rolls[1])
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                bonus_roll: reserved.rolls[2],
            }) {
                Ok(plan) => plan,
                Err(ForgeRuleError::InsufficientDust) => return Ok(Err("INSUFFICIENT_DUST")),
                Err(ForgeRuleError::InsufficientCores) => return Ok(Err("INSUFFICIENT_CORES")),
                Err(_) => return Ok(Err("WRONG_ITEMS")),
            };
            inputs.push(target.line(1, plan.target_tier));
            inputs.push(source.line(
                u32::from(plan.source_tier.is_some()),
                plan.source_tier.unwrap_or(ItemTier::ZERO),
            ));
            if plan.cost.cores > 0 {
                stash = inventory::consume_material(
                    tx,
                    fence,
                    inventory,
                    &core,
                    u32::from(plan.cost.cores),
                    &mut inputs,
                )
                .await?;
            }
            (
                plan.success,
                plan.bonus,
                plan.cost.gold,
                plan.cost.dust,
                plan.cost.cores,
                0,
                0,
                0,
                Some(plan.target_tier.get()),
                plan.source_tier.map(ItemTier::get),
            )
        }
        Operation::DustToSlivers | Operation::SliversToCores => {
            let resources = crate::domain::forge::ForgeDustResources {
                dust: u64::from(before.balance()),
                slivers,
                cores,
            };
            let converted = if request.operation == Operation::DustToSlivers {
                resources.dust_to_slivers()
            } else {
                resources.slivers_to_core()
            };
            let after = match converted {
                Ok(after) => after,
                Err(crate::domain::forge::ForgeDustError::InsufficientDust) => {
                    return Ok(Err("INSUFFICIENT_DUST"));
                }
                Err(crate::domain::forge::ForgeDustError::InsufficientSlivers) => {
                    return Ok(Err("INSUFFICIENT_SLIVERS"));
                }
                Err(_) => return Ok(Err("INVALID_INPUT")),
            };
            let dust = u32::try_from(resources.dust - after.dust)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let slivers_spent = u32::try_from(resources.slivers.saturating_sub(after.slivers))
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let new_slivers = u32::try_from(after.slivers.saturating_sub(resources.slivers))
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let new_cores = u32::try_from(after.cores - resources.cores)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if slivers_spent > 0 {
                stash = inventory::consume_material(
                    tx,
                    fence,
                    inventory,
                    &sliver,
                    slivers_spent,
                    &mut inputs,
                )
                .await?;
            }
            outputs = match inventory::produce(
                tx,
                fence,
                native,
                inventory,
                if new_slivers > 0 { &sliver } else { &core },
                new_slivers + new_cores,
                &mut inputs,
            )
            .await?
            {
                Ok(outputs) => outputs,
                Err(reason) => return Ok(Err(reason)),
            };
            (
                true,
                0,
                0,
                dust,
                0,
                slivers_spent,
                new_slivers,
                new_cores,
                None,
                None,
            )
        }
        _ => return Ok(Err("INVALID_INPUT")),
    };
    let dust_after = if dust == 0 {
        before
    } else {
        before
            .spend(dust)
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .after
    };
    let reply = json!({"operation":operation_name(request.operation),"succeeded":success,"applied":true,"reason":null,"bonus":bonus,
        "result":{"first_item":request.first_item.map(|handle|handle.get()),"first_tier":first,
            "second_item":request.second_item.map(|handle|handle.get()),"second_tier":second,
            "dust":dust_after.balance(),"dust_limit":dust_after.limit().get(),"gold_spent":gold,"cores_spent":spent_cores,
            "slivers_spent":spent_slivers,"slivers_created":new_slivers,"cores_created":new_cores}});
    Ok(Ok(Effects {
        reply,
        inputs,
        outputs,
        stash,
        dust,
        gold,
        dust_before: before,
    }))
}

pub(super) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    request: Request,
    reserved: &storage::Reservation,
    reply: &Value,
    effects: Option<&Effects>,
) -> NativeResult<()> {
    let audit_proof = effects
        .map(|effects| proof(fence, reserved, request, effects))
        .transpose()?;
    sqlx::query("INSERT INTO game_forge_operations(character_id,occurrence_id,transaction_id,event_id,game_session_id,command_id,operation,reply,applied,inputs,outputs,stash,dust_spent,gold_spent,dust_before,dust_limit,occurred_at,audit_proof) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(reserved.transaction.as_slice()).bind(reserved.event.as_slice())
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(request.operation as i16).bind(reply).bind(effects.is_some())
        .bind(json!(effects.map_or(&[][..],|e|e.inputs.as_slice()))).bind(json!(effects.map_or(&[][..],|e|e.outputs.as_slice())))
        .bind(effects.and_then(|e|e.stash.as_ref())).bind(i64::from(effects.map_or(0,|e|e.dust)))
        .bind(i64::try_from(effects.map_or(0,|e|e.gold)).map_err(|_|DurabilityError::InvalidStoredState)?)
        .bind(effects.map(|e|i64::from(e.dust_before.balance()))).bind(effects.map(|e|i16::try_from(e.dust_before.limit().get()).unwrap_or(i16::MAX)))
        .bind(reserved.occurred_at).bind(audit_proof).execute(&mut **tx).await?;
    Ok(())
}

pub(super) fn proof(
    fence: CurrentCharacterItemFence,
    reserved: &storage::Reservation,
    request: Request,
    effects: &Effects,
) -> NativeResult<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let hash = |value: &Value| {
        serde_json::to_vec(value)
            .map(|bytes| Sha256::digest(bytes).to_vec())
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let value = audit::Proof {
        occurrence: reserved.occurrence.to_vec(),
        character: fence.character_id.as_bytes().to_vec(),
        world: fence.runtime_scope.world_id().as_bytes().to_vec(),
        operation: request.operation as u8,
        content: reserved.content.to_vec(),
        ruleset: reserved.ruleset.clone(),
        simulation_revision: reserved.simulation_revision,
        rolls: reserved.rolls,
        reply: effects.reply.clone(),
        inputs_hash: hash(&json!(effects.inputs))?,
        outputs_hash: hash(&json!(effects.outputs))?,
        stash_hash: hash(&json!(effects.stash))?,
        inputs: effects.inputs.len() as u64,
        outputs: effects.outputs.len() as u64,
        retired: effects
            .inputs
            .iter()
            .filter(|line| line["quantity_after"] == 0)
            .count() as u64,
        tier_changes: effects
            .inputs
            .iter()
            .filter(|line| line["tier_before"] != line["tier_after"])
            .count() as u64,
        dust_spent: effects.dust,
        gold_spent: effects.gold,
        stash_spent: effects.stash.is_some(),
    };
    let proof = serde_json::to_vec(&value).map_err(|_| DurabilityError::InvalidStoredState)?;
    audit::decode(&proof).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(proof)
}

// Keep the independently resolved fence/content and the immutable plan explicit
// at this single physical-transaction composition boundary.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    request: Request,
    reserved: &storage::Reservation,
    native: &NativeGameplayState,
    inventory: &inventory::Inventory,
    effects: &Effects,
) -> NativeResult<std::result::Result<(), &'static str>> {
    for line in &effects.inputs {
        sqlx::query("UPDATE game_item_instances SET quantity=($1->>'quantity_after')::bigint,lifecycle=CASE WHEN ($1->>'quantity_after')::bigint=0 THEN 2 ELSE 1 END,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=($1->>'item')::uuid")
            .bind(line).bind(reserved.transaction.as_slice()).execute(&mut ***tx).await?;
        if line["quantity_after"] == 0 {
            let query = if line["location"]["kind"] == 1 {
                "DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid"
            } else {
                "DELETE FROM game_item_nested_entries WHERE item_instance_id=($1->>'item')::uuid"
            };
            sqlx::query(query).bind(line).execute(&mut ***tx).await?;
        }
        if line["tier_before"] != line["tier_after"] {
            if line["tier_after"] == 0 {
                sqlx::query(
                    "DELETE FROM game_item_tiers WHERE item_instance_id=($1->>'item')::uuid",
                )
                .bind(line)
                .execute(&mut ***tx)
                .await?;
            } else if line["tier_before"] == 0 {
                sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(($1->>'item')::uuid,($1->>'tier_after')::smallint)").bind(line).execute(&mut ***tx).await?;
            } else {
                sqlx::query("UPDATE game_item_tiers SET tier=($1->>'tier_after')::smallint WHERE item_instance_id=($1->>'item')::uuid").bind(line).execute(&mut ***tx).await?;
            }
        }
    }
    for output in &effects.outputs {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,encode($3,'hex')::uuid)")
            .bind(output).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(reserved.transaction.as_slice()).execute(&mut ***tx).await?;
        if output["location"]["kind"] == 1 {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,($1->'location'->>'parent')::uuid,($1->'location'->>'ordinal')::numeric,encode($4,'hex')::uuid)")
                .bind(output).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(reserved.transaction.as_slice()).execute(&mut ***tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,($1->'location'->>'parent')::uuid,($1->'location'->>'ordinal')::numeric,encode($3,'hex')::uuid)")
                .bind(output).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(reserved.transaction.as_slice()).execute(&mut ***tx).await?;
        }
    }
    let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let gameplay = super::super::super::character_progression::CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: crate::domain::CharacterRevision::new(
            revision
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    if effects.dust > 0 {
        let outcome = write_entry(
            tx,
            &gameplay,
            Write {
                kind: if request.operation == Operation::DustToSlivers {
                    ForgeDustEntryKind::Convert
                } else {
                    ForgeDustEntryKind::Spend
                },
                cause: "forge",
                occurrence: request.occurrence,
                amount: effects.dust,
                ids: ForgeDustWriteIds {
                    entry_id: reserved.entry,
                    transaction_id: reserved.transaction,
                },
            },
        )
        .await
        .map_err(|error| match error {
            ForgeDustWriteError::Unavailable(error) => error,
            _ => DurabilityError::InvalidStoredState,
        })?;
        if !matches!(outcome, ForgeDustOutcome::Written(_)) {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let proof = proof(fence, reserved, request, effects)?;
    if effects.gold > 0 {
        let Some(platinum) = inventory::current_definition(native, "oteryn:item.tibia.i3035")
        else {
            return Ok(Err("UNSUPPORTED_ITEM"));
        };
        let Some(gold) = inventory::current_definition(native, "oteryn:item.tibia.i3031") else {
            return Ok(Err("UNSUPPORTED_ITEM"));
        };
        let Some(crystal) = inventory::current_definition(native, "oteryn:item.tibia.i3043") else {
            return Ok(Err("UNSUPPORTED_ITEM"));
        };
        let ids: (String, String) =
            sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text")
                .fetch_one(&mut ***tx)
                .await?;
        let fee = FeeBurnRequest {
            cause: FeeBurnCause::Forge {
                occurrence: request.occurrence,
                proof,
            },
            fee_gold_units: effects.gold,
            transaction_id: reserved.transaction,
            event_id: reserved.event,
            occurred_at_unix_ms: reserved.occurred_at,
            server_build_id: SERVER_BUILD_ID.into(),
            change: FeeChangeFacts {
                item_instance_ids: [uuid_text(&ids.0)?, uuid_text(&ids.1)?],
                platinum,
                gold,
                crystal,
                backpack: inventory.bag_facts.clone(),
            },
        };
        match super::super::super::item_fee_burn::burn_fee_in_transaction(tx, &gameplay, &fee).await
        {
            Ok(_) => (),
            Err(FeeBurnError::InsufficientFunds) => return Ok(Err("INSUFFICIENT_GOLD")),
            Err(FeeBurnError::ChangeDoesNotFit | FeeBurnError::CapacityExceeded) => {
                return Ok(Err("NO_ROOM"));
            }
            Err(FeeBurnError::Unavailable(error)) => return Err(error),
            Err(_) => return Ok(Err("INVALID_INPUT")),
        }
    } else {
        let envelope = audit::encode(fence, command, reserved, proof, tx.tuple().tuple())?;
        let audit_item = effects
            .inputs
            .first()
            .or_else(|| effects.outputs.first())
            .and_then(|line| line["item"].as_str());
        let tuple = tx.tuple();
        sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)")
            .bind(reserved.event.as_slice()).bind(reserved.transaction.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(audit_item)
            .bind(reserved.occurred_at).bind(mint_audit::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut ***tx).await?;
    }
    let envelope_hash:Vec<u8>=sqlx::query_scalar("SELECT envelope_sha256 FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid AND transaction_id=encode($2,'hex')::uuid").bind(reserved.event.as_slice()).bind(reserved.transaction.as_slice()).fetch_one(&mut ***tx).await?;
    if let Some(stash) = &effects.stash {
        apply_stash(tx, fence, command, reserved, stash, &envelope_hash).await?;
    }
    sqlx::query("UPDATE game_forge_operations SET envelope_sha256=$3 WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.occurrence.as_slice()).bind(envelope_hash).execute(&mut ***tx).await?;
    storage::append_history(tx, reserved, &effects.reply).await?;
    Ok(Ok(()))
}

async fn apply_stash(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    reserved: &storage::Reservation,
    stash: &Value,
    envelope_hash: &[u8],
) -> NativeResult<()> {
    sqlx::query("INSERT INTO game_character_stash_operations(transaction_id,game_session_id,command_id,character_id,world_id,intent,action,outcome,event_id,occurred_at,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,sha256($6),4,2,encode($7,'hex')::uuid,$8,$9)")
        .bind(reserved.transaction.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&reserved.intent).bind(reserved.event.as_slice()).bind(reserved.occurred_at).bind(envelope_hash).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_character_stash_entries(entry_id,transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after,previous_entry_id) VALUES(($1->>'entry')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$1->>'key',$1->>'revision',($1->>'amount')::bigint,($1->>'before')::bigint,($1->>'after')::bigint,($1->>'previous')::uuid)")
        .bind(stash).bind(reserved.transaction.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut ***tx).await?;
    sqlx::query("UPDATE game_character_stash_balances SET quantity=($1->>'after')::bigint,last_entry_id=($1->>'entry')::uuid WHERE character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid AND definition_key=$1->>'key' AND definition_revision=$1->>'revision'")
        .bind(stash).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut ***tx).await?;
    Ok(())
}
