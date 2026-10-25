//! SUPPLY-STASH-0: Character/World value conversions under the native item fence.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::assert_current_equipment_in_transaction;
use super::item_loop_move::{ItemLoopContent, carried_weight};
use super::item_mint::{TypedDefinitionRef, uuid_text};
use super::item_transfer::{
    CurrentCharacterItemFence, read_derived_transfer_capacity_centi_oz, scope_of, stack_maximum,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::premium::PremiumGate;
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::ReferenceItemField::Known;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_protocol_oteryn::stash::*;
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};

pub(crate) mod audit;

#[derive(Debug, Clone)]
pub struct StashRequest {
    pub command: CommandRef,
    pub intent: StashIntent,
    /// Resolved by this connection's inventory view; never supplied as a wire identity.
    pub source: Option<[u8; 16]>,
    /// Current page membership. A durable replay precedes ephemeral view resolution.
    pub row_open: bool,
    pub access_refusal: Option<StashOutcome>,
    pub premium: PremiumGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StashReceipt {
    pub stowed_quantity: u32,
    pub transaction: [u8; 16],
    pub outcome: StashOutcome,
}

struct Plan {
    definition: TypedDefinitionRef,
    entry: String,
    amount: u32,
    before: i64,
    previous: Option<String>,
    parent: Vec<u8>,
    lines: Vec<Value>,
}

fn stowable(native: &NativeGameplayState, definition: &TypedDefinitionRef) -> bool {
    definition.family=="Item" && native.item_policy(&definition.production_key,&definition.revision_ref)
        .is_some_and(|policy| {
            let semantics=&policy.record().semantics;
            matches!(&semantics.trade_restrictions,Known(trade) if trade.marketable==Known(true))
                && !matches!(&semantics.container,Known(_))
        })
}

fn definition(row: &sqlx::postgres::PgRow) -> Result<TypedDefinitionRef, DurabilityError> {
    Ok(TypedDefinitionRef {
        family: "Item".into(),
        production_key: row.try_get("definition_key")?,
        revision_ref: row.try_get("definition_revision")?,
    })
}

impl DurabilityRoot {
    /// One bounded page, including incompatible revisions so stored value never disappears.
    pub async fn read_stash_page(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
        page: u32,
        native: &NativeGameplayState,
    ) -> Result<(Vec<StashRow>, bool), DurabilityError> {
        if page > MAX_STASH_PAGE {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let mut context = native;
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,native|Box::pin(async move {
            let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            let rows=sqlx::query("SELECT uuid_send(b.last_entry_id) AS handle,b.definition_key,b.definition_revision,b.quantity FROM game_character_stash_balances b JOIN game_character_roots r USING(character_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.quantity>0 AND r.lifecycle=1 ORDER BY b.definition_key COLLATE \"C\",b.definition_revision COLLATE \"C\" LIMIT 33 OFFSET $2")
                .bind(character.as_bytes().as_slice()).bind(i64::from(page)*32).fetch_all(&mut *tx).await?;
            let more=rows.len()>32;
            let rows=rows.into_iter().take(32).map(|row| {
                let definition=definition(&row)?;
                Ok(StashRow {handle:row.try_get::<Vec<u8>,_>("handle")?.try_into().map_err(|_|DurabilityError::InvalidStoredState)?,
                    migration_required:!stowable(native,&definition), definition_key:definition.production_key,definition_revision:definition.revision_ref,
                    quantity:u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_|DurabilityError::InvalidStoredState)?})
            }).collect::<Result<Vec<_>,DurabilityError>>()?;
            super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok((rows,more))
        })).await
    }

    /// Probe before resolving handles, then plan and commit one exact conversion or refusal.
    /// The CommandRef and original canonical request bind the immutable receipt across restart.
    #[allow(clippy::too_many_arguments)]
    pub async fn stash_conversion<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: StashRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
    ) -> Result<Option<StashReceipt>, DurabilityError> {
        let intent =
            encode_stash_intent(request.intent).map_err(|_| DurabilityError::InvalidStoredState)?;
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.action == StashAction::Browse
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,context|Box::pin(async move {
            let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:stash:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut *tx).await?;
            let previous=sqlx::query("SELECT uuid_send(r.transaction_id) AS transaction,uuid_send(r.character_id) AS character,r.intent,r.outcome,CASE WHEN r.action=2 THEN COALESCE(e.amount,0) ELSE 0 END AS stowed FROM game_character_stash_operations r LEFT JOIN game_character_stash_entries e ON e.transaction_id=r.transaction_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            if let Some(previous)=previous {
                let same=previous.try_get::<Vec<u8>,_>("intent")?==intent && previous.try_get::<Vec<u8>,_>("character")?==fence.character_id.as_bytes();
                let outcome=if same {outcome_from_code(previous.try_get("outcome")?)?} else {StashOutcome::Rejected};
                let transaction=previous.try_get::<Vec<u8>,_>("transaction")?.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
                super::db::commit_semantic_transaction(tx,deadline).await?;
                return Ok(Some(StashReceipt {transaction,outcome,stowed_quantity:if same {u32::try_from(previous.try_get::<i64,_>("stowed")?).map_err(|_|DurabilityError::InvalidStoredState)?}else{0}}));
            }
            if probe {super::db::commit_semantic_transaction(tx,deadline).await?;return Ok(None);}
            assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(request.command),context.1.loop_content_digest()).await.map_err(|_|DurabilityError::Unavailable)?;
            if context.1.loop_content_digest()!=context.0.source_digest() {return Err(DurabilityError::Unavailable);}
            let planned=plan(&mut tx,fence,&request,context.0,context.1).await?;
            let (transaction,event,occurred):(String,String,i64)=sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            let outcome=match &planned {Ok(_)=>StashOutcome::Committed,Err(outcome)=>*outcome};
            let action=if matches!(request.intent.action,StashAction::Stow(_)) {2_i16}else{3};
            let stowed_quantity=if action==2 {planned.as_ref().map(|p|p.amount).unwrap_or(0)}else{0};
            let envelope=match &planned {Ok(plan)=>Some(audit::encode(fence,&request,plan,&transaction,&event,occurred)?),Err(_)=>None};
            let (world,_)=scope_of(&fence).map_err(|_|DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_character_stash_operations(transaction_id,game_session_id,command_id,character_id,world_id,intent,action,outcome,event_id,occurred_at,envelope_sha256) VALUES($1::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,$8,$9::uuid,$10,sha256($11))")
                .bind(&transaction).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&intent).bind(action).bind(outcome as i16).bind(envelope.as_ref().map(|_|event.as_str())).bind(occurred).bind(envelope.as_deref()).execute(&mut *tx).await?;
            if let Ok(plan)=planned {
                apply(&mut tx,fence,&plan,action,&transaction).await?;
                let envelope=envelope.ok_or(DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_character_stash_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,world_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,9,1,'ECONOMY_LEDGER_RETENTION_V1',encode($3,'hex')::uuid,$4,$4+2592000000,$5,sha256($5),1)")
                    .bind(&event).bind(&transaction).bind(world.as_bytes().as_slice()).bind(occurred).bind(&envelope).execute(&mut *tx).await?;
            }
            super::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(Some(StashReceipt {transaction:uuid_text(&transaction)?,outcome,stowed_quantity}))
        })).await
    }
}

fn outcome_from_code(code: i16) -> Result<StashOutcome, DurabilityError> {
    Ok(match code {
        2 => StashOutcome::Committed,
        3 => StashOutcome::NoLocker,
        4 => StashOutcome::Stale,
        5 => StashOutcome::NoRoom,
        6 => StashOutcome::NoBackpack,
        7 => StashOutcome::Overweight,
        8 => StashOutcome::Unsupported,
        9 => StashOutcome::Rejected,
        10 => StashOutcome::PremiumRequired,
        11 => StashOutcome::AmountLimit,
        12 => StashOutcome::Insufficient,
        _ => return Err(DurabilityError::InvalidStoredState),
    })
}

async fn plan<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: &StashRequest,
    native: &NativeGameplayState,
    content: &C,
) -> Result<Result<Plan, StashOutcome>, DurabilityError> {
    macro_rules! refuse {
        ($outcome:ident) => {
            return Ok(Err(StashOutcome::$outcome))
        };
    }
    if let Some(refusal) = request.access_refusal {
        return Ok(Err(refusal));
    }
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let bag=sqlx::query("SELECT uuid_send(s.item_instance_id) AS parent,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND i.lifecycle=1")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(bag) = bag else { refuse!(NoBackpack) };
    let parent: Vec<u8> = bag.try_get("parent")?;
    // The root fence serializes the carried graph; item locks are always UUID ordered.
    sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY item_instance_id FOR UPDATE")
        .bind(&parent).bind(request.source.unwrap_or([0;16]).as_slice()).fetch_all(&mut **tx).await?;
    let stow = matches!(request.intent.action, StashAction::Stow(_));
    let (definition, amount, source) = match request.intent.action {
        StashAction::Stow(_) => {
            if !request.row_open {
                refuse!(Stale);
            }
            if !request.premium.premium_current() {
                refuse!(PremiumRequired);
            }
            let Some(source) = request.source else {
                refuse!(Stale)
            };
            let source=sqlx::query("SELECT i.definition_family,i.definition_production_key AS definition_key,i.definition_revision_ref AS definition_revision,i.quantity,to_jsonb(i) AS before_item,e.placement_ordinal::text AS ordinal,i.item_instance_id::text AS item, NOT EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_tiers WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=i.item_instance_id) AS plain FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=encode($3,'hex')::uuid AND i.world_id=encode($4,'hex')::uuid AND e.world_id=i.world_id AND i.lifecycle=1")
                .bind(source.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let Some(source) = source else { refuse!(Stale) };
            if !source.try_get::<bool, _>("plain")?
                || source.try_get::<String, _>("definition_family")? != "Item"
            {
                refuse!(Unsupported);
            }
            let def = definition(&source)?;
            let quantity = u32::try_from(source.try_get::<i64, _>("quantity")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            (def, quantity, Some(source))
        }
        StashAction::Withdraw { row, amount } => {
            if !request.row_open {
                refuse!(Stale);
            }
            let row=sqlx::query("SELECT definition_key,definition_revision FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND last_entry_id=encode($3,'hex')::uuid AND quantity>0")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(row.as_slice()).fetch_optional(&mut **tx).await?;
            let Some(row) = row else { refuse!(Stale) };
            (definition(&row)?, amount, None)
        }
        StashAction::Browse => refuse!(Rejected),
    };
    if !stowable(native, &definition) {
        refuse!(Unsupported);
    }
    let Some(facts) = content.loop_facts(&definition) else {
        refuse!(Unsupported)
    };
    if facts.definition != definition {
        refuse!(Unsupported);
    }
    let maximum = stack_maximum(facts.stack).map(|(m, _)| m).unwrap_or(0);
    if maximum == 0 || maximum > 100 || facts.container_capacity.is_some() {
        refuse!(Unsupported);
    }
    if (stow && amount > maximum) || (!stow && amount > MAX_STASH_OUTPUTS * maximum) {
        refuse!(AmountLimit);
    }
    let outputs = if stow { 0 } else { amount.div_ceil(maximum) };
    if !stow {
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
        let occupied:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut **tx).await?;
        let tree:i64=sqlx::query_scalar("WITH RECURSIVE tree(item_instance_id,depth) AS (SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8) SELECT count(*) FROM (SELECT item_instance_id FROM tree LIMIT 501) bounded").bind(&parent).fetch_one(&mut **tx).await?;
        if occupied + i64::from(outputs) > i64::from(capacity) || tree + i64::from(outputs) > 500 {
            refuse!(NoRoom);
        }
        let Some(weight) = facts.weight else {
            refuse!(Unsupported)
        };
        let carried = match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await
        {
            Ok(weight) => weight,
            Err(super::item_loop_move::LoopError::Refused(_)) => refuse!(Unsupported),
            Err(super::item_loop_move::LoopError::Database(error)) => return Err(error.into()),
            Err(super::item_loop_move::LoopError::Durability(error)) => return Err(error),
        };
        let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id).await?;
        if carried
            .checked_add(u64::from(weight) * u64::from(amount))
            .is_none_or(|total| total > capacity)
        {
            refuse!(Overweight);
        }
    }
    // The balance is last in the lock order, after root, items and backpack slot.
    sqlx::query("SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let balance=sqlx::query("SELECT quantity,last_entry_id::text AS previous FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 AND definition_revision=$4 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).fetch_optional(&mut **tx).await?;
    let (before, previous) = if let Some(balance) = balance {
        (
            balance.try_get::<i64, _>("quantity")?,
            Some(balance.try_get::<String, _>("previous")?),
        )
    } else {
        (0, None)
    };
    if !stow && i64::from(amount) > before {
        refuse!(Insufficient);
    }
    if stow {
        let after = before + i64::from(amount);
        if after > i64::from(MAX_STASH_UNITS) {
            refuse!(AmountLimit);
        }
        let used:i64=sqlx::query_scalar("SELECT COALESCE(sum((quantity+99)/100),0)::bigint FROM game_character_stash_balances WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND quantity>0")
            .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        let depot:i64=sqlx::query_scalar("SELECT COALESCE(max(n),0)::bigint FROM (SELECT sum(committed) AS n FROM game_character_depot_box_counters WHERE character_id=encode($1,'hex')::uuid GROUP BY town_id) towns")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if depot + used - (before + 99) / 100 + (after + 99) / 100 > 15_000 {
            refuse!(NoRoom);
        }
    }
    let mut lines = Vec::new();
    if let Some(source) = source {
        lines.push(json!({"item":source.try_get::<String,_>("item")?,"count":amount,"ordinal":source.try_get::<String,_>("ordinal")?,"before":source.try_get::<Value,_>("before_item")?}));
    } else {
        let highest:String=sqlx::query_scalar("SELECT coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut **tx).await?;
        let highest = highest
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        for i in 0..outputs {
            let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut **tx)
                .await?;
            let ordinal = highest
                .checked_add(u64::from(i) + 1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            lines.push(json!({"item":item,"count":(amount-i*maximum).min(maximum),"ordinal":ordinal.to_string()}));
        }
    }
    let entry = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(Ok(Plan {
        definition,
        entry,
        amount,
        before,
        previous,
        parent,
        lines,
    }))
}

async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    plan: &Plan,
    action: i16,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = &plan.entry;
    let after = plan.before
        + if action == 2 {
            i64::from(plan.amount)
        } else {
            -i64::from(plan.amount)
        };
    sqlx::query("INSERT INTO game_character_stash_entries(entry_id,transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after,previous_entry_id) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10::uuid)")
        .bind(entry).bind(transaction).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&plan.definition.production_key).bind(&plan.definition.revision_ref).bind(i64::from(plan.amount)).bind(plan.before).bind(after).bind(plan.previous.as_deref()).execute(&mut **tx).await?;
    for (position, line) in plan.lines.iter().enumerate() {
        sqlx::query("INSERT INTO game_character_stash_item_lines(transaction_id,ordinal,item_instance_id,definition_key,definition_revision,quantity,direction,parent_item_instance_id,placement_ordinal,before_item) VALUES($1::uuid,$2,($3->>'item')::uuid,$4,$5,($3->>'count')::bigint,$6,encode($7,'hex')::uuid,($3->>'ordinal')::numeric,$8)")
            .bind(transaction).bind(i16::try_from(position+1).map_err(|_|DurabilityError::InvalidStoredState)?).bind(line).bind(&plan.definition.production_key).bind(&plan.definition.revision_ref).bind(if action==2 {1_i16}else{2}).bind(&plan.parent).bind(line.get("before")).execute(&mut **tx).await?;
    }
    if plan.previous.is_none() {
        sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&plan.definition.production_key).bind(&plan.definition.revision_ref).bind(after).bind(entry).execute(&mut **tx).await?;
    } else {
        sqlx::query("UPDATE game_character_stash_balances SET quantity=$5,last_entry_id=$6::uuid WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 AND definition_revision=$4")
            .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&plan.definition.production_key).bind(&plan.definition.revision_ref).bind(after).bind(entry).execute(&mut **tx).await?;
    }
    for line in &plan.lines {
        if action == 2 {
            sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid").bind(line).bind(transaction).execute(&mut **tx).await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid").bind(line).execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,($1->>'count')::bigint,1,$5::uuid)")
                .bind(line).bind(world.as_bytes().as_slice()).bind(&plan.definition.production_key).bind(&plan.definition.revision_ref).bind(transaction).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,($1->>'ordinal')::numeric,$5::uuid)")
                .bind(line).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&plan.parent).bind(transaction).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
