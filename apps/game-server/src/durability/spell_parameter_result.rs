//! Candidate private result outbox on the actual command/Character source owner.
//! Historical bytes can be replayed, but cannot reopen an editor or pay a cast.
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError, check_transaction};
use crate::durability::item_mint::TypedDefinitionRef;
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_protocol_oteryn::actor_spell_v2::{
    ParameterSpellCastIntent, ParameterSpellCastResult, decode_parameter_spell_cast_result,
    encode_parameter_spell_cast_intent, encode_parameter_spell_cast_result,
};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone)]
pub(crate) struct ParameterResultRecord {
    session: crate::foundation::GameSessionId,
    command: u64,
    intent: Vec<u8>,
    result: Vec<u8>,
    cost_transaction: Option<[u8; 16]>,
    physical_transaction: String,
    historical: bool,
}
impl ParameterResultRecord {
    pub(crate) async fn verify_staged_row(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), super::DurabilityError> {
        if self.historical {
            return Err(super::DurabilityError::InvalidStoredState);
        }
        let found: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_parameter_results WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0) AND intent=$3 AND result=$4 AND created_xact_id=pg_current_xact_id() AND created_xact_id::text=$5 AND cost_transaction_id IS NOT DISTINCT FROM CASE WHEN $6::bytea IS NULL THEN NULL ELSE encode($6,'hex')::uuid END)")
            .bind(self.session.as_bytes().as_slice()).bind(self.command.to_string()).bind(&self.intent).bind(&self.result).bind(&self.physical_transaction).bind(self.cost_transaction.map(|v|v.to_vec())).fetch_one(&mut **tx).await?;
        if !found {
            return Err(super::DurabilityError::InvalidStoredState);
        }
        Ok(())
    }
    pub(crate) fn result(&self) -> Result<ParameterSpellCastResult, SpellItemError> {
        decode_parameter_spell_cast_result(&self.result)
            .map_err(|_| SpellItemError::Rejected("stored parameter result codec"))
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.result
    }
    pub(crate) fn cost_transaction(&self) -> Option<[u8; 16]> {
        self.cost_transaction
    }
    pub(crate) fn physical_transaction(&self) -> &str {
        &self.physical_transaction
    }
    pub(crate) fn historical(&self) -> bool {
        self.historical
    }
}
fn bad<T>() -> Result<T, SpellItemError> {
    Err(SpellItemError::Rejected(
        "parameter result binding mismatch",
    ))
}
/// Re-read by exact normalized parameter intent after independently checking the
/// current sealed owner. The physical XID status distinguishes history from an
/// INSERT in this reader transaction; neither observation grants runtime authority.
pub(crate) async fn read_parameter_result_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    spell: &TypedDefinitionRef,
    intent: &ParameterSpellCastIntent,
) -> Result<Option<ParameterResultRecord>, SpellItemError> {
    check_transaction(tx, authority).await?;
    let intent_bytes = encode_parameter_spell_cast_intent(intent)
        .map_err(|_| SpellItemError::Rejected("parameter intent codec"))?;
    let row=sqlx::query("SELECT uuid_send(character_id) AS character,uuid_send(world_id) AS world,scope_generation::text,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,uuid_send(cost_transaction_id) AS cost,created_xact_id::text AS physical,created_xact_id=pg_current_xact_id() AS same,pg_xact_status(created_xact_id) AS status FROM game_spell_parameter_results WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0) FOR SHARE")
        .bind(authority.game_session_id().as_bytes().as_slice())
        .bind(authority.command().command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    let stored_intent: Vec<u8> = row.try_get("intent")?;
    let result: Vec<u8> = row.try_get("result")?;
    let generation: String = row.try_get("scope_generation")?;
    let historical = !row.try_get::<bool, _>("same")?;
    if row.try_get::<Vec<u8>, _>("character")? != authority.character_id_bytes()
        || row.try_get::<Vec<u8>, _>("world")? != *authority.runtime_scope().world_id().as_bytes()
        || generation.parse::<u64>().ok() != Some(authority.scope_generation())
        || row.try_get::<Vec<u8>, _>("catalog_digest")? != authority.compatible_content_digest()
        || row.try_get::<String, _>("spell_key")? != spell.production_key
        || row.try_get::<String, _>("spell_revision")? != spell.revision_ref
        || stored_intent != intent_bytes
        || row.try_get::<Vec<u8>, _>("intent_digest")? != Sha256::digest(&stored_intent).as_slice()
        || row.try_get::<Vec<u8>, _>("result_digest")? != Sha256::digest(&result).as_slice()
        || historical && row.try_get::<Option<String>, _>("status")?.as_deref() != Some("committed")
    {
        return bad();
    }
    let decoded = decode_parameter_spell_cast_result(&result)
        .map_err(|_| SpellItemError::Rejected("stored parameter result codec"))?;
    if encode_parameter_spell_cast_result(&decoded).ok().as_deref() != Some(result.as_slice()) {
        return bad();
    }
    let cost: Option<Vec<u8>> = row.try_get("cost")?;
    let cost_transaction = cost
        .map(|b| {
            b.try_into()
                .map_err(|_| SpellItemError::Rejected("stored parameter cost UUID"))
        })
        .transpose()?;
    Ok(Some(ParameterResultRecord {
        session: authority.game_session_id(),
        command: authority.command().command_id().get(),
        intent: stored_intent,
        result,
        cost_transaction,
        physical_transaction: row.try_get("physical")?,
        historical,
    }))
}
/// This is called only after the source callback and common caster preflight.
/// The receipt/editor foreign keys and deferred guards bind all successful
/// output to the actual cost source in this physical transaction.
pub(crate) async fn write_parameter_result_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    spell: &TypedDefinitionRef,
    intent: &ParameterSpellCastIntent,
    result: &ParameterSpellCastResult,
    cost_transaction: Option<[u8; 16]>,
) -> Result<ParameterResultRecord, SpellItemError> {
    check_transaction(tx, authority).await?;
    if spell.family != "Spell"
        || result.disposition == SpellCastDisposition::Cast && cost_transaction.is_none()
    {
        return bad();
    }
    let intent_bytes = encode_parameter_spell_cast_intent(intent)
        .map_err(|_| SpellItemError::Rejected("parameter intent codec"))?;
    let result_bytes = encode_parameter_spell_cast_result(result)
        .map_err(|_| SpellItemError::Rejected("parameter result codec"))?;
    if let Some(existing) =
        read_parameter_result_in_transaction(tx, authority, spell, intent).await?
    {
        if existing.result != result_bytes || existing.cost_transaction != cost_transaction {
            return bad();
        }
        return Ok(existing);
    }
    let editor = result.editor.as_ref().map(|e| e.editor_id.to_vec());
    let cost = cost_transaction.map(|v| v.to_vec());
    sqlx::query("INSERT INTO game_spell_parameter_results(game_session_id,command_id,character_id,world_id,scope_generation,catalog_digest,spell_key,spell_revision,intent,intent_digest,result,result_digest,cast_succeeded,cost_transaction_id,editor_id) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric(20,0),$6,$7,$8,$9,sha256($9),$10,sha256($10),$11,CASE WHEN $12::bytea IS NULL THEN NULL ELSE encode($12,'hex')::uuid END,CASE WHEN $13::bytea IS NULL THEN NULL ELSE encode($13,'hex')::uuid END)")
        .bind(authority.game_session_id().as_bytes().as_slice()).bind(authority.command().command_id().get().to_string())
        .bind(authority.character_id_bytes().as_slice()).bind(authority.runtime_scope().world_id().as_bytes().as_slice())
        .bind(authority.scope_generation().to_string()).bind(authority.compatible_content_digest().as_slice())
        .bind(&spell.production_key).bind(&spell.revision_ref).bind(&intent_bytes).bind(&result_bytes)
        .bind(result.disposition==SpellCastDisposition::Cast).bind(cost).bind(editor).execute(&mut **tx).await?;
    read_parameter_result_in_transaction(tx, authority, spell, intent)
        .await?
        .ok_or(SpellItemError::Rejected("parameter result INSERT lost"))
}
