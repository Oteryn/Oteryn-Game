//! Command-retained banker and native item-use conversion of carried coins only.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::begin_type2_transaction;
use super::item_loop_move::{ItemLoopContent, carried_weight};
use super::item_mint::TypedDefinitionRef;
use super::character_progression::CurrentCharacterGameplayFence;
use super::item_timed_state::{
    self, TimedCheckpointRequest, TimedDefinitionFacts, TimedExpiryRequest, TimedWriteError,
};
use super::item_transfer::{CurrentCharacterItemFence, read_derived_transfer_capacity_centi_oz};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::currency::Coin;
use crate::domain::{
    CharacterRevision,
    timed_item::{ExpireReason, TimedValues},
};
use crate::foundation::CommandRef;
use oteryn_gameplay_economy::coin_exchange::{CoinExchange, ExchangeRefusal, ExchangeUse, qualify, qualify_item_use};
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct CoinExchangeRequest {
    pub exchange: CoinExchange,
    /// Number of conversions (output platinum/crystal when upgrading, input coins otherwise).
    pub amount: u64,
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub npc_key: String,
    pub catalogue_revision: u64,
    pub conversation_revision: u64,
}

pub struct CoinExchangeReceipt {
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub line: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinConverter {
    Gold([u8; 16]),
    Magic([u8; 16]),
}

impl CoinConverter {
    fn id(self) -> [u8; 16] { match self { Self::Gold(id) | Self::Magic(id) => id } }
    fn code(self) -> i16 { match self { Self::Gold(_) => 1, Self::Magic(_) => 2 } }
}

fn code(exchange: CoinExchange) -> i16 {
    match exchange {
        CoinExchange::GoldToPlatinum => 1,
        CoinExchange::PlatinumToGold => 2,
        CoinExchange::PlatinumToCrystal => 3,
        CoinExchange::CrystalToPlatinum => 4,
    }
}
fn from_code(value: i16) -> Result<CoinExchange, DurabilityError> {
    match value {
        1 => Ok(CoinExchange::GoldToPlatinum),
        2 => Ok(CoinExchange::PlatinumToGold),
        3 => Ok(CoinExchange::PlatinumToCrystal),
        4 => Ok(CoinExchange::CrystalToPlatinum),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

impl DurabilityRoot {
    /// Accept a banker request or one exact item-use source, or resume its retained command.
    /// Acceptance commits before execution; the source binding separates item USE from NPC replay.
    #[allow(clippy::too_many_arguments)]
    pub async fn exchange_carried_coins<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: Option<CoinExchangeRequest>,
        source: Option<[u8; 16]>,
        content: Option<&C>,
        coins: Option<[TypedDefinitionRef; 3]>,
        converter: Option<(CoinConverter, TimedDefinitionFacts)>,
    ) -> Result<Option<CoinExchangeReceipt>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(DurabilityError::InvalidStoredState);
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = content;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, content| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock(&mut tx, command).await?;
            let existing = read(&mut tx, command).await?;
            if let Some(row) = existing.as_ref() {
                let stored: Option<Vec<u8>> = row.try_get("item_source")?;
                if stored.as_deref() != source.as_ref().map(|id| id.as_slice()) {
                    // NPC resume must never execute or acknowledge an item-use receipt.
                    if source.is_none() { tx.commit(deadline).await?; return Ok(None); }
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            if existing.is_none() {
                let Some(request) = request.as_ref() else { tx.commit(deadline).await?; return Ok(None); };
                let content = content.ok_or(DurabilityError::Unavailable)?;
                if request.npc_generation == 0 || request.catalogue_revision == 0 || request.conversation_revision == 0 || request.npc_key.is_empty() || request.npc_key.len() > 512 {
                    return Err(DurabilityError::InvalidStoredState);
                }
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await.map_err(|_| DurabilityError::Unavailable)?;
                sqlx::query("INSERT INTO game_coin_exchange_receipts(transaction_id,game_session_id,command_id,character_id,world_id,content_digest,exchange,amount,npc_identity,npc_generation,npc_key,catalogue_revision,conversation_revision,item_source,converter_source,converter_kind) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7::text::numeric,$8,$9::text::numeric,$10,$11::text::numeric,$12::text::numeric,CASE WHEN $13::bytea IS NULL THEN NULL ELSE encode($13,'hex')::uuid END,CASE WHEN $14::bytea IS NULL THEN NULL ELSE encode($14,'hex')::uuid END,$15)")
                    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(content.loop_content_digest().as_slice()).bind(code(request.exchange)).bind(request.amount.to_string()).bind(request.npc_identity.as_slice()).bind(request.npc_generation.to_string()).bind(&request.npc_key).bind(request.catalogue_revision.to_string()).bind(request.conversation_revision.to_string()).bind(source.as_ref().map(|id| id.as_slice())).bind(converter.as_ref().map(|(tool, _)| tool.id()).as_ref().map(|id| id.as_slice())).bind(converter.as_ref().map(|(tool, _)| tool.code())).execute(&mut **tx).await?;
            }
            // The immutable accepted request survives a timeout or process loss after this commit.
            tx.commit(deadline).await?;
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock(&mut tx, command).await?;
            let row = read(&mut tx, command).await?.ok_or(DurabilityError::InvalidStoredState)?;
            let character: Vec<u8> = row.try_get("character")?;
            if character != fence.character_id.as_bytes() { return Err(DurabilityError::InvalidStoredState); }
            let exchange = from_code(row.try_get("exchange")?)?;
            let amount: u64 = row.try_get::<String,_>("amount")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            let identity: Vec<u8> = row.try_get("npc_identity")?;
            let generation: u64 = row.try_get::<String,_>("npc_generation")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            if let Some(request) = request.as_ref()
                && (request.exchange != exchange || request.amount != amount || request.npc_identity.as_slice() != identity || request.npc_generation != generation || request.npc_key != row.try_get::<String,_>("npc_key")? || request.catalogue_revision.to_string() != row.try_get::<String,_>("catalogue_revision")? || request.conversation_revision.to_string() != row.try_get::<String,_>("conversation_revision")?) {
                return Err(DurabilityError::InvalidStoredState);
            }
            let mut result: i16 = row.try_get("result")?;
            let stored_converter: Option<Vec<u8>> = row.try_get("converter_source")?;
            if stored_converter.as_deref() != converter.as_ref().map(|(tool, _)| tool.id()).as_ref().map(|id| id.as_slice())
                || row.try_get::<Option<i16>,_>("converter_kind")? != converter.as_ref().map(|(tool, _)| tool.code()) { return Err(DurabilityError::InvalidStoredState); }
            if result == 0 {
                let content = content.ok_or(DurabilityError::Unavailable)?;
                let coins = coins.as_ref().ok_or(DurabilityError::Unavailable)?;
                if row.try_get::<Vec<u8>,_>("content_digest")? != content.loop_content_digest() {
                    return Err(DurabilityError::Unavailable);
                }
                assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content.loop_content_digest()).await.map_err(|_| DurabilityError::Unavailable)?;
                let transaction: String = row.try_get("transaction_id")?;
                // An unusable converter refuses before any coin is planned or locked.
                let charge = match converter.as_ref() {
                    Some((tool, facts)) => match qualify_converter(&mut tx, fence, *tool, facts, exchange).await? {
                        Some(charge) => Some(charge),
                        None => {
                            sqlx::query("UPDATE game_coin_exchange_receipts SET result=2,inputs='[]',outputs='[]' WHERE transaction_id=$1::uuid")
                                .bind(&transaction).execute(&mut **tx).await?;
                            tx.commit(deadline).await?;
                            return Ok(Some(CoinExchangeReceipt { npc_identity: identity.try_into().map_err(|_| DurabilityError::InvalidStoredState)?, npc_generation: generation, line: exchange.insufficient_coins().to_owned(), changed: false }));
                        }
                    },
                    None => None,
                };
                let (outcome, inputs, outputs) = plan(&mut tx, fence, exchange, amount, source, content, coins).await?;
                result = match outcome { Ok(()) => 1, Err(ExchangeRefusal::InvalidAmount) => 2, Err(ExchangeRefusal::InsufficientCoins) => 3, Err(ExchangeRefusal::NoRoom) => 4 };
                sqlx::query("UPDATE game_coin_exchange_receipts SET result=$2,inputs=$3,outputs=$4 WHERE transaction_id=$1::uuid")
                    .bind(&transaction).bind(result).bind(json!(inputs)).bind(json!(outputs)).execute(&mut **tx).await?;
                if result == 1 {
                    if let (Some((tool, facts)), Some(charge)) = (converter.as_ref(), charge) {
                        spend_converter(&mut tx, fence, *tool, facts, charge).await?;
                    }
                    apply(&mut tx, fence, &transaction, &inputs, &outputs).await?;
                }
            }
            let line = match result {
                1 => "Here you are.",
                2 | 3 => exchange.insufficient_coins(),
                4 => "There is not enough room in your backpack.",
                _ => return Err(DurabilityError::InvalidStoredState),
            }.to_owned();
            tx.commit(deadline).await?;
            Ok(Some(CoinExchangeReceipt { npc_identity: identity.try_into().map_err(|_| DurabilityError::InvalidStoredState)?, npc_generation: generation, line, changed: result == 1 }))
        })).await
    }
}

impl DurabilityRoot {
    /// MAGICCONV: toggle a carried magic gold converter between its two admitted forms.
    /// The charge row moves unchanged with the definition under one cause-9 timed write.
    /// `transaction_id` is derived from the command, so a retry replays the committed toggle.
    /// Returns whether the converter is now active, or `None` when it is not togglable.
    #[allow(clippy::too_many_arguments)]
    pub async fn toggle_magic_converter(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        transaction_id: [u8; 16],
        converter: [u8; 16],
        inactive: &TimedDefinitionFacts,
        active: &TimedDefinitionFacts,
    ) -> Result<Option<bool>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let full = inactive.full.charges().filter(|_| {
            inactive.definition.production_key == MAGIC_CONVERTER_INACTIVE
                && active.definition.production_key == MAGIC_CONVERTER_ACTIVE
                && inactive.definition.family == "Item"
                && active.definition.family == "Item"
                && inactive.full == active.full
                && inactive.full.remaining_ms().is_none()
                && !inactive.lit_continuous
                && !active.lit_continuous
        });
        let Some(full) = full else { return Ok(None) };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let (inactive, active) = (inactive.definition.clone(), active.definition.clone());
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            // The live session-generation fence takes the Character root lock before the item.
            if super::character_progression::assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            let replay = sqlx::query("SELECT uuid_send(item_instance_id) AS item,definition_after_production_key FROM game_item_timed_state_writes WHERE transaction_id=encode($1,'hex')::uuid AND cause=9")
                .bind(transaction_id.as_slice()).fetch_optional(&mut **tx).await?;
            if let Some(replay) = replay {
                if replay.try_get::<Vec<u8>, _>("item")? != converter {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let now = replay.try_get::<String, _>("definition_after_production_key")? == MAGIC_CONVERTER_ACTIVE;
                tx.commit(deadline).await?;
                return Ok(Some(now));
            }
            let row = sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=encode($3,'hex')::uuid) FOR UPDATE OF i")
                .bind(converter.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let Some(row) = row else { tx.commit(deadline).await?; return Ok(None) };
            let stored = TypedDefinitionRef {
                family: row.try_get("definition_family")?,
                production_key: row.try_get("definition_production_key")?,
                revision_ref: row.try_get("definition_revision_ref")?,
            };
            let (before, after) = if stored == inactive { (&inactive, &active) } else if stored == active { (&active, &inactive) } else { tx.commit(deadline).await?; return Ok(None) };
            let state = item_timed_state::load_timed_state(&mut tx, &converter).await.map_err(timed_error)?;
            let (revision, charges) = match state {
                None => (0, full),
                Some(state) => match (state.deadline_at, state.values.and_then(|values| values.charges())) {
                    (None, Some(charges)) if charges >= 1 && charges <= full => (state.revision, charges),
                    _ => { tx.commit(deadline).await?; return Ok(None) }
                },
            };
            sqlx::query("INSERT INTO game_item_timed_state_writes(item_instance_id,expected_revision,cause,transaction_id,holder_character_id,definition_before_family,definition_before_production_key,definition_before_revision_ref,definition_after_family,definition_after_production_key,definition_after_revision_ref,full_charges,full_remaining_ms,lit_continuous,charges_before,remaining_ms_before,deadline_before,charges_after,remaining_ms_after,deadline_after) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),9,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10,$11,NULL,false,$12,NULL,NULL,$12,NULL,NULL)")
                .bind(converter.as_slice()).bind(revision.to_string()).bind(transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
                .bind(&before.family).bind(&before.production_key).bind(&before.revision_ref)
                .bind(&after.family).bind(&after.production_key).bind(&after.revision_ref)
                .bind(i32::try_from(full).map_err(|_| DurabilityError::InvalidStoredState)?).bind(i32::try_from(charges).map_err(|_| DurabilityError::InvalidStoredState)?)
                .execute(&mut **tx).await?;
            sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,definition_revision_ref=$3,last_transaction_id=encode($4,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(converter.as_slice()).bind(&after.production_key).bind(&after.revision_ref).bind(transaction_id.as_slice()).execute(&mut **tx).await?;
            let written = if revision == 0 {
                sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,remaining_ms,deadline_at,state_revision) VALUES(encode($1,'hex')::uuid,$2,NULL,NULL,1)")
                    .bind(converter.as_slice()).bind(i32::try_from(charges).map_err(|_| DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?
            } else {
                sqlx::query("UPDATE game_item_timed_states SET state_revision=state_revision+1 WHERE item_instance_id=encode($1,'hex')::uuid AND state_revision=$2::text::numeric(20,0)")
                    .bind(converter.as_slice()).bind(revision.to_string()).execute(&mut **tx).await?
            };
            if written.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
            tx.commit(deadline).await?;
            Ok(Some(after.production_key == MAGIC_CONVERTER_ACTIVE))
        })).await
    }
}

pub const MAGIC_CONVERTER_INACTIVE: &str = "oteryn:item.tibia.i28525";
pub const MAGIC_CONVERTER_ACTIVE: &str = "oteryn:item.tibia.i28526";

async fn lock(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    command: CommandRef,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:coin-exchange:'||encode($1,'hex')||':'||$2,0))")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut ***tx).await?;
    Ok(())
}
async fn read(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    command: CommandRef,
) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query("SELECT transaction_id::text,uuid_send(character_id) AS character,content_digest,exchange,amount::text,npc_identity,npc_generation::text,npc_key,catalogue_revision::text,conversation_revision::text,result,uuid_send(item_source) AS item_source,uuid_send(converter_source) AS converter_source,converter_kind FROM game_coin_exchange_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric FOR UPDATE")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut ***tx).await?)
}

/// The converter's live charge row, locked for this execution transaction.
struct ConverterCharge {
    revision: u64,
    charges: u32,
}

fn converter_keys(tool: CoinConverter) -> &'static [&'static str] {
    match tool {
        CoinConverter::Gold(_) => &["oteryn:item.tibia.i23722", "oteryn:item.tibia.i25719"],
        CoinConverter::Magic(_) => &[MAGIC_CONVERTER_ACTIVE],
    }
}

/// `None` refuses the conversion: the tool is not a live, carried, charged converter of
/// the admitted definition. Custody follows the timed burn: a direct backpack entry.
async fn qualify_converter(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    tool: CoinConverter,
    facts: &TimedDefinitionFacts,
    exchange: CoinExchange,
) -> Result<Option<ConverterCharge>, DurabilityError> {
    if matches!(tool, CoinConverter::Magic(_))
        && !matches!(exchange, CoinExchange::GoldToPlatinum | CoinExchange::PlatinumToCrystal)
    {
        return Ok(None);
    }
    let definition = &facts.definition;
    if definition.family != "Item"
        || !converter_keys(tool).contains(&definition.production_key.as_str())
        || facts.full.remaining_ms().is_some()
        || facts.full.charges().is_none()
        || facts.lit_continuous
    {
        return Ok(None);
    }
    let row = sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=encode($3,'hex')::uuid) FOR UPDATE OF i")
        .bind(tool.id().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else { return Ok(None) };
    let stored = TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    };
    if &stored != definition {
        return Ok(None);
    }
    let charge = match item_timed_state::load_timed_state(tx, &tool.id()).await.map_err(timed_error)? {
        None => facts.full.charges().map(|charges| ConverterCharge { revision: 0, charges }),
        Some(state) if state.deadline_at.is_none() => state
            .values
            .and_then(|values| values.charges())
            .map(|charges| ConverterCharge { revision: state.revision, charges }),
        Some(_) => None,
    };
    Ok(charge.filter(|charge| charge.charges >= 1))
}

/// Spend one charge under its own timed write; the last charge burns the converter.
/// Its TransactionId is distinct from the receipt's, whose coin lines it never joins.
async fn spend_converter(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    tool: CoinConverter,
    facts: &TimedDefinitionFacts,
    charge: ConverterCharge,
) -> Result<(), DurabilityError> {
    let ids = sqlx::query("SELECT uuid_send(game_character_uuid_v7()) AS tx,uuid_send(game_character_uuid_v7()) AS event,floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now")
        .fetch_one(&mut ***tx).await?;
    let id = |column: &str| -> Result<[u8; 16], DurabilityError> {
        ids.try_get::<Vec<u8>, _>(column)?.try_into().map_err(|_| DurabilityError::InvalidStoredState)
    };
    let transaction_id = id("tx")?;
    if charge.charges > 1 {
        let checkpoint = TimedCheckpointRequest {
            item_instance_id: tool.id(),
            expected_revision: charge.revision,
            transaction_id,
            definition: facts.clone(),
            values: TimedValues::new(Some(charge.charges - 1), None).map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        item_timed_state::validate_checkpoint(&checkpoint).map_err(timed_error)?;
        item_timed_state::write_checkpoint(tx, fence.character_id, &checkpoint).await.map_err(timed_error)?;
        return Ok(());
    }
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let gameplay = CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(revision.parse().map_err(|_| DurabilityError::InvalidStoredState)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    let expiry = TimedExpiryRequest {
        item_instance_id: tool.id(),
        expected_revision: charge.revision,
        transaction_id,
        event_id: id("event")?,
        reason: ExpireReason::ChargesExhausted,
        definition: facts.clone(),
        target: None,
        occurred_at_unix_ms: ids.try_get("now")?,
        server_build_id: super::character_authority::SERVER_BUILD_ID.to_owned(),
    };
    item_timed_state::validate_expiry(&expiry).map_err(timed_error)?;
    let tuple = tx.tuple();
    item_timed_state::write_expiry(tx, &gameplay, &expiry, tuple).await.map_err(timed_error)?;
    Ok(())
}

fn timed_error(error: TimedWriteError) -> DurabilityError {
    match error {
        TimedWriteError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}

type Plan = (Result<(), ExchangeRefusal>, Vec<Value>, Vec<Value>);
#[allow(clippy::too_many_arguments)]
async fn plan<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    exchange: CoinExchange,
    amount: u64,
    source: Option<[u8; 16]>,
    content: &C,
    coins: &[TypedDefinitionRef; 3],
) -> Result<Plan, DurabilityError> {
    let refuse = |reason| Ok((Err(reason), Vec::new(), Vec::new()));
    if amount > u64::from(u32::MAX) {
        return refuse(ExchangeRefusal::InvalidAmount);
    }
    let initial = match qualify(exchange, amount, u64::MAX, usize::MAX, true) {
        Ok(plan) => plan,
        Err(reason) => return refuse(reason),
    };
    // The main backpack and its nested tree are the only roots. Depot and bank are never read.
    let rows = sqlx::query("WITH RECURSIVE tree AS (SELECT b.item_instance_id,NULL::uuid AS parent,0::numeric AS ordinal,0 AS depth FROM game_item_container_slots b WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid UNION ALL SELECT e.item_instance_id,e.parent_item_instance_id,e.placement_ordinal,t.depth+1 FROM tree t JOIN (SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries) e ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8) SELECT to_jsonb(i) AS before,i.item_instance_id::text AS item,t.parent::text,t.ordinal::text,t.depth,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,NOT EXISTS(SELECT 1 FROM game_item_keys k WHERE k.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_timed_states s WHERE s.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_texts s WHERE s.item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_imbuements s WHERE s.item_instance_id=i.item_instance_id) AS plain FROM tree t JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid ORDER BY t.depth,t.ordinal DESC,i.item_instance_id LIMIT 502 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
    if rows.is_empty() {
        return refuse(ExchangeRefusal::InsufficientCoins);
    }
    if rows.len() > 501 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut inputs = Vec::new();
    let mut needed = initial.input.1;
    let mut available = 0_u64;
    let mut weights = (0_u64, 0_u64);
    let mut bags = Vec::new();
    for row in &rows {
        let definition = TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        };
        let Some(facts) = content.loop_facts(&definition) else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let item: String = row.try_get("item")?;
        let depth: i32 = row.try_get("depth")?;
        if let Some(capacity) = facts.container_capacity
            && depth < 8
        {
            bags.push((
                item.clone(),
                depth,
                usize::try_from(capacity.min(20))
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ));
        }
        let coin = Coin::from_production_key(&definition.production_key);
        if depth == 0 || definition.family != "Item" || !row.try_get::<bool, _>("plain")? {
            continue;
        }
        let count = u64::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let item_id = super::character_progression::uuid_text(&item)?;
        let selected = source.is_none_or(|id| item_id == id);
        if coin == Some(initial.input.0) && selected {
            if source.is_some() {
                let selected = qualify_item_use(initial.input.0, count, ExchangeUse::Coin, usize::MAX, true);
                if amount != 1 || !selected.is_ok_and(|p| p.exchange == initial) {
                    return refuse(ExchangeRefusal::InvalidAmount);
                }
            }
            available = available
                .checked_add(count)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let consumed = needed.min(count);
            if consumed > 0 {
                needed -= consumed;
                weights.0 +=
                    consumed * u64::from(facts.weight.ok_or(DurabilityError::InvalidStoredState)?);
                inputs.push(json!({"item":item,"parent":row.try_get::<String,_>("parent")?,"ordinal":row.try_get::<String,_>("ordinal")?,"nested":depth>1,"count":consumed,"before":row.try_get::<Value,_>("before")?}));
            }
        }
    }
    if available < initial.input.1 {
        return refuse(ExchangeRefusal::InsufficientCoins);
    }
    // Resolve the output coin even if the purse holds none of that denomination yet.
    let definition = coins
        .iter()
        .find(|d| d.family == "Item" && d.production_key == initial.output.0.production_key())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let facts = content
        .loop_facts(definition)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let maximum = super::item_transfer::stack_maximum(facts.stack)
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .0;
    let mut remaining = initial.output.1;
    let mut outputs = Vec::new();
    for row in &rows {
        if remaining == 0 {
            break;
        }
        if row.try_get::<i32, _>("depth")? == 0
            || !row.try_get::<bool, _>("plain")?
            || row.try_get::<String, _>("definition_production_key")? != definition.production_key
            || row.try_get::<String, _>("definition_revision_ref")? != definition.revision_ref
        {
            continue;
        }
        let quantity = u64::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let count = remaining.min(u64::from(maximum).saturating_sub(quantity));
        if count > 0 {
            remaining -= count;
            outputs.push(json!({"item":row.try_get::<String,_>("item")?,"parent":row.try_get::<String,_>("parent")?,"ordinal":row.try_get::<String,_>("ordinal")?,"nested":row.try_get::<i32,_>("depth")?>1,"key":definition.production_key,"revision":definition.revision_ref,"count":count,"before":row.try_get::<Value,_>("before")?}));
        }
    }
    let freed = inputs
        .iter()
        .filter(|line| line["count"] == line["before"]["quantity"])
        .count();
    let mut created = 0;
    for (parent, depth, capacity) in bags {
        let occupied = rows
            .iter()
            .filter(|row| {
                row.try_get::<Option<String>, _>("parent")
                    .ok()
                    .flatten()
                    .as_ref()
                    == Some(&parent)
            })
            .count();
        let released = inputs
            .iter()
            .filter(|line| line["parent"] == parent && line["count"] == line["before"]["quantity"])
            .count();
        let mut ordinal = rows
            .iter()
            .filter(|row| {
                row.try_get::<Option<String>, _>("parent")
                    .ok()
                    .flatten()
                    .as_ref()
                    == Some(&parent)
            })
            .map(|row| {
                row.try_get::<String, _>("ordinal")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(u64::MAX)
            })
            .max()
            .unwrap_or(0);
        for _ in 0..capacity.saturating_sub(occupied - released) {
            if remaining == 0 {
                break;
            }
            if rows.len() - 1 - freed + created >= 500 {
                return refuse(ExchangeRefusal::NoRoom);
            }
            ordinal = ordinal
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut ***tx)
                .await?;
            let count = remaining.min(u64::from(maximum));
            remaining -= count;
            created += 1;
            outputs.push(json!({"item":item,"parent":parent,"ordinal":ordinal.to_string(),"nested":depth>0,"key":definition.production_key,"revision":definition.revision_ref,"count":count}));
        }
    }
    if remaining != 0 {
        return refuse(ExchangeRefusal::NoRoom);
    }
    weights.1 = initial
        .output
        .1
        .checked_mul(u64::from(
            facts.weight.ok_or(DurabilityError::InvalidStoredState)?,
        ))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let carried = carried_weight(
        tx,
        fence.character_id,
        *fence.runtime_scope.world_id().as_bytes(),
        content,
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
    if carried
        .checked_sub(weights.0)
        .and_then(|n| n.checked_add(weights.1))
        .is_none_or(|n| n > capacity)
    {
        return refuse(ExchangeRefusal::NoRoom);
    }
    // The exact post-consumption capacity and all receiver room have been materialized above.
    qualify(exchange, amount, available, created, !outputs.is_empty())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok((Ok(()), inputs, outputs))
}

async fn apply(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    transaction: &str,
    inputs: &[Value],
    outputs: &[Value],
) -> Result<(), DurabilityError> {
    for line in inputs {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-($1->>'count')::bigint,lifecycle=CASE WHEN quantity=($1->>'count')::bigint THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid")
            .bind(line).bind(transaction).execute(&mut ***tx).await?;
        if line["count"] == line["before"]["quantity"] {
            let query = if line["nested"] == true {
                "DELETE FROM game_item_nested_entries WHERE item_instance_id=($1->>'item')::uuid"
            } else {
                "DELETE FROM game_item_container_entries WHERE item_instance_id=($1->>'item')::uuid"
            };
            sqlx::query(query).bind(line).execute(&mut ***tx).await?;
        }
    }
    for line in outputs {
        if line.get("before").is_some() {
            sqlx::query("UPDATE game_item_instances SET quantity=quantity+($1->>'count')::bigint,last_transaction_id=$2::uuid WHERE item_instance_id=($1->>'item')::uuid").bind(line).bind(transaction).execute(&mut ***tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)").bind(line).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(transaction).execute(&mut ***tx).await?;
            let query = if line["nested"] == true {
                "INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,($1->>'parent')::uuid,($1->>'ordinal')::numeric,$3::uuid)"
            } else {
                "INSERT INTO game_item_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id,character_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,($1->>'parent')::uuid,($1->>'ordinal')::numeric,$3::uuid,encode($4,'hex')::uuid)"
            };
            let mut query = sqlx::query(query)
                .bind(line)
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(transaction);
            if line["nested"] != true {
                query = query.bind(fence.character_id.as_bytes().as_slice());
            }
            query.execute(&mut ***tx).await?;
        }
    }
    Ok(())
}
