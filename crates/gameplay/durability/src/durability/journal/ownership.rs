//! Registered expected bindings never substitute for current Game authority.
use super::*;
/// Called only after independently checking current owner, World, Character
/// revision and recovery generation. Validate every remaining receipt member.
pub(super) async fn approved_cut(
    tx: &mut Transaction<'_, Postgres>,
    s: &ReadGameJournalCursorV1,
) -> Result<bool> {
    let deleted: bool =
        sqlx::query_scalar("SELECT game_journal_deleted($1::text::uuid,$2::text::uuid)")
            .bind(&s.account_id)
            .bind(&s.character_id)
            .fetch_one(&mut **tx)
            .await?;
    if deleted {
        return Ok(false);
    }
    let Some(character) = &s.character_id else {
        return Ok(true);
    };
    let epoch: Option<String> = sqlx::query_scalar("SELECT game_journal_projection_epoch()")
        .fetch_one(&mut **tx)
        .await?;
    if epoch.as_ref() != s.projection_epoch.as_ref() {
        return Ok(false);
    }
    let receipt=sqlx::query("SELECT d.receipt,d.receipt_digest FROM game_bazaar_character_transitions t JOIN game_bazaar_decisions d USING(issuer,operation_id) WHERE t.character_id=$1::text::uuid AND d.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND d.command='TransferCharacterOwnership' AND d.result_code='CHAR_CMD_OK' ORDER BY t.new_revision DESC LIMIT 1")
  .bind(character).fetch_optional(&mut **tx).await?;
    if let Some(row) = receipt {
        let bytes: Vec<u8> = row.try_get("receipt")?;
        let digest: Vec<u8> = row.try_get("receipt_digest")?;
        if Sha256::digest(&bytes).as_slice() != digest {
            return Err(DurabilityError::InvalidStoredState);
        }
        let body: serde_json::Value = serde_json::from_slice(&bytes).map_err(invalid)?;
        return Ok(body["contract_version"] == 1
            && body["operation"] == "PublishCharacterCommandReceiptV1"
            && body["command"] == "TransferCharacterOwnership"
            && body["outcome"] == "COMMITTED"
            && body["result_code"] == "CHAR_CMD_OK"
            && body["character_id"].as_str() == Some(character.as_str())
            && body["to_account_id"].as_str() == Some(s.account_id.as_str())
            && body["character_revision"].as_str() == s.character_revision.as_deref()
            && body["projection_epoch"].as_str() == s.projection_epoch.as_deref()
            && body["from_projection_revision"].as_str() == s.from_projection_revision.as_deref()
            && body["to_projection_revision"].as_str() == s.to_projection_revision.as_deref()
            && body["recovery_generation"].as_str() == Some(s.recovery_generation.as_str()));
    }
    // Initial ownership has no prior-owner receipt. A protected acceptance record
    // is necessary, and the live account projection must not precede the accepted
    // initial projection; the immutable approval alone grants no authority.
    let accepted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_journal_items_baselines WHERE stream_key=$1)",
    )
    .bind(s.encode().map_err(invalid)?)
    .fetch_one(&mut **tx)
    .await?;
    if !accepted {
        return Ok(false);
    }
    let projection: Option<String> =
        sqlx::query_scalar("SELECT game_journal_account_projection($1::text::uuid)")
            .bind(&s.account_id)
            .fetch_one(&mut **tx)
            .await?;
    let expected = s
        .to_projection_revision
        .as_deref()
        .and_then(oteryn_gameplay_native_admission_source::character_commands::counter);
    let observed = projection
        .as_deref()
        .and_then(oteryn_gameplay_native_admission_source::character_commands::counter);
    Ok(s.from_projection_revision == s.to_projection_revision
        && matches!((expected,observed),(Some(a),Some(b)) if b>=a))
}
