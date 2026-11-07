//! Trusted accepted Bazaar ledger facts for Drome consumers, not S2S admission.
//! All normal 0994/0995 SQL triggers remain enabled: the same-transaction
//! transition proves the exact lifecycle/revision update and emits real audit.
use super::TestResult;
use sha2::{Digest, Sha256};

#[derive(serde::Serialize)]
struct AcceptedReceipt<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'static str,
    command: &'a str,
    operation_id: String,
    outcome: &'static str,
    result_code: &'static str,
    result_data: Option<serde_json::Value>,
    recovery_generation: &'static str,
    decided_at: String,
}

pub(super) async fn transition(
    pool: &sqlx::PgPool,
    character: &[u8; 16],
    lifecycle: i16,
    decided_at: i64,
) -> TestResult {
    let mut tx = pool.begin().await?;
    let (account, world, old_lifecycle, revision): (sqlx::types::Uuid, sqlx::types::Uuid, i16, String) = sqlx::query_as("SELECT account_id,world_id,lifecycle,character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_one(&mut *tx).await.map_err(|error| format!("accepted auction root lookup: {error:?}"))?;
    assert_ne!(old_lifecycle, lifecycle);
    let revision = revision.parse::<u64>()?;
    let next = revision.checked_add(1).ok_or("Bazaar revision overflow")?;
    let operation: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
        .fetch_one(&mut *tx)
        .await?;
    let command = if lifecycle == 2 {
        "LockBazaarCharacter"
    } else {
        "UnlockBazaarCharacter"
    };
    let character = sqlx::types::Uuid::from_bytes(*character);
    let mut intent_value = serde_json::json!({
        "contract_version":2,"command":command,"operation_id":operation.to_string(),
        "variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY",
        "issuer_decision_id":operation.to_string(),"source_revision":next.to_string(),
        "listing_id":operation.to_string(),
        "issued_at_source":(decided_at-1).to_string(),"expires_at_source":(decided_at+120).to_string(),
        "audience":"OTERYN_GAME_BAZAAR_AUTHORITY",
        "character_id":character.to_string(),"seller_account_id":account.to_string(),"world_id":world.to_string(),
    });
    use oteryn_gameplay_native_admission_source::bazaar;
    let intent = if lifecycle == 2 {
        intent_value["expected_character_revision"] = serde_json::json!(revision.to_string());
        intent_value["eligibility_revision"] = serde_json::json!("1");
        intent_value["snapshot_schema_version"] = serde_json::json!(1);
        let typed: bazaar::LockBazaarCharacter = serde_json::from_value(intent_value)?;
        serde_json::to_vec(&typed)?
    } else {
        intent_value["lock_id"] = serde_json::json!(operation.to_string());
        intent_value["expected_lock_revision"] = serde_json::json!("1");
        let typed: bazaar::UnlockBazaarCharacter = serde_json::from_value(intent_value)?;
        serde_json::to_vec(&typed)?
    };
    assert!(bazaar::decode_intent(&intent, command, &operation.to_string()).is_ok());
    // The consumer uses accepted lifecycle history; no invented snapshot or
    // signed capture payload is supplied. Receipt fields follow bazaar/mod.rs:107-119.
    let receipt = serde_json::to_vec(&AcceptedReceipt {
        contract_version: 2,
        operation: "PublishCharacterCommandReceiptV1",
        source_authority: "oteryn:character-authority:primary",
        command,
        operation_id: operation.to_string(),
        outcome: "COMMITTED",
        result_code: "CHAR_CMD_OK",
        result_data: None,
        recovery_generation: "1",
        decided_at: decided_at.to_string(),
    })?;
    // Exact BazaarAuthorityDecisionV1 digest/generation payload: bazaar/mod.rs:524-536.
    let mut payload = vec![0x0a, 32];
    payload.extend_from_slice(&Sha256::digest(&intent));
    payload.extend_from_slice(&[0x12, 32]);
    payload.extend_from_slice(&Sha256::digest(&receipt));
    payload.extend_from_slice(&[0x18, 1]);
    sqlx::query("INSERT INTO game_bazaar_decisions(issuer,operation_id,command,intent,receipt,result_code,recovery_generation,decided_at,event_id,transaction_id,world_id,payload) VALUES('OTERYN_PLATFORM_BAZAAR_AUTHORITY',$1,$2,$3,$4,'CHAR_CMD_OK',1,$5,game_character_uuid_v7(),game_character_uuid_v7(),$6,$7)")
        .bind(operation).bind(command).bind(intent).bind(receipt).bind(decided_at).bind(world).bind(payload).execute(&mut *tx).await.map_err(|error| format!("accepted auction decision: {error:?}"))?;
    sqlx::query("INSERT INTO game_bazaar_character_transitions(operation_id,character_id,old_account,new_account,old_lifecycle,new_lifecycle,old_revision,new_revision,issuer) VALUES($1,$2,$3,$3,$4,$5,$6::text::numeric,$7::text::numeric,'OTERYN_PLATFORM_BAZAAR_AUTHORITY')")
        .bind(operation).bind(character).bind(account).bind(old_lifecycle).bind(lifecycle).bind(revision.to_string()).bind(next.to_string()).execute(&mut *tx).await.map_err(|error| format!("accepted auction transition: {error:?}"))?;
    sqlx::query("UPDATE game_character_roots SET lifecycle=$2,character_revision=$3::text::numeric WHERE character_id=$1")
        .bind(character).bind(lifecycle).bind(next.to_string()).execute(&mut *tx).await.map_err(|error| format!("accepted auction lifecycle root update: {error:?}"))?;
    tx.commit()
        .await
        .map_err(|error| format!("accepted auction commit: {error:?}"))?;
    Ok(())
}
