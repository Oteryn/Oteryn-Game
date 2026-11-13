//! Game-owned Bazaar command decisions. One semantic pass owns the effect,
//! immutable receipt, audit payload and pending publication. No network in a DB transaction.
mod audit;
pub use audit::AuditDelivery;
mod characters;
mod gold;
mod items;
mod preload;
mod publication;
pub(super) mod recovery;
mod transfer;
pub use transfer::{TransferIntent, decode_transfer};

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::native_admission_source::{
    bazaar as wire,
    character_commands::{Decision, Input, source_authority_valid, uuid_v7},
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
pub const TRANSFER_COMMAND: &str = "TransferCharacterOwnership";
pub const CHARACTER_ISSUER: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";
pub const OK: &str = "CHAR_CMD_OK";
/// Bounded authority snapshot shared with the production node adapter.
pub const TRANSPORT_BATCH_LIMIT: u16 = 24;

/// Candidate policy defaults selected by the owner on 2026-11-06. Missing
/// provisioned identities, keys or admitted Content never grant authority.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub obligations_revision: Option<u64>,
    pub cooldown_seconds: Option<u64>,
    pub snapshot_survives_transfer: bool,
    pub escrow_account_id: Option<String>,
    pub escrow_capacity: Option<u16>,
    pub inbox_capacity: Option<u64>,
    pub item_classes: Vec<String>,
    pub content_artifact_file: Option<String>,
    pub content_artifact_sha256: Option<String>,
    #[serde(skip)]
    preloaded_artifact: Option<std::sync::Arc<items::PreparedArtifact>>,
    pub gold_fee_bps: Option<u16>,
    pub reserve_incoming_headroom: bool,
    pub common_restore_scope: bool,
    pub signing_key_file: Option<String>,
    pub signing_kid: Option<String>,
    pub signing_trust_generation: Option<u64>,
    pub snapshot_source_authority: Option<String>,
    pub signing_public_keys: std::collections::BTreeMap<String, String>,
    pub revoked_signing_kids: Vec<String>,
    pub proof_retention_seconds: Option<u64>,
    pub v2_receipt_bytes: Option<usize>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            obligations_revision: Some(1),
            cooldown_seconds: Some(2_592_000),
            snapshot_survives_transfer: true,
            escrow_account_id: None,
            escrow_capacity: Some(64),
            inbox_capacity: Some(10_000),
            item_classes: vec![
                "nonstackable".into(),
                "stackable".into(),
                "empty_container".into(),
            ],
            content_artifact_file: None,
            content_artifact_sha256: None,
            preloaded_artifact: None,
            gold_fee_bps: Some(250),
            reserve_incoming_headroom: true,
            common_restore_scope: true,
            signing_key_file: None,
            signing_kid: None,
            signing_trust_generation: Some(1),
            snapshot_source_authority: Some("oteryn:game:primary".into()),
            signing_public_keys: std::collections::BTreeMap::new(),
            revoked_signing_kids: Vec::new(),
            proof_retention_seconds: Some(7_776_000),
            v2_receipt_bytes: Some(270336),
        }
    }
}

pub(super) struct Request {
    issuer: String,
    source: String,
    command: String,
    operation: String,
    raw: Vec<u8>,
    generation: u64,
    technical: Option<&'static str>,
    content: Option<preload::PreparedContent>,
}

#[derive(Serialize)]
struct Receipt<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    command: &'a str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'a str,
    result_data: Option<&'a RawValue>,
    recovery_generation: String,
    decided_at: String,
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "existing authority context and typed read outcome"
    )]
    pub async fn decide_bazaar_input(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        command: &str,
        operation: &str,
        input: Input<'_>,
    ) -> Decision {
        let (raw, technical) = match input {
            Input::Intent(raw) => (raw.to_vec(), None),
            Input::ReadConflict => (
                super::character_command::read_conflict_binding(command),
                Some("CHAR_CMD_OPERATION_CONFLICT"),
            ),
            Input::Unsupported => (
                super::character_command::unsupported_binding(command),
                Some("CHAR_CMD_UNSUPPORTED"),
            ),
            Input::Oversized => (
                super::character_command::oversized_binding(command),
                Some("CHAR_CMD_INTENT_INVALID"),
            ),
        };
        self.decide_authority_command(
            authority,
            proof,
            source,
            command,
            operation,
            &raw,
            wire::ISSUER,
            technical,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "existing authority context plus immutable command binding"
    )]
    pub async fn decide_bazaar_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        command: &str,
        operation: &str,
        raw: &[u8],
    ) -> Decision {
        self.decide_authority_command(
            authority,
            proof,
            source,
            command,
            operation,
            raw,
            wire::ISSUER,
            None,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "existing authority context plus immutable command binding"
    )]
    pub(super) async fn decide_authority_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        command: &str,
        operation: &str,
        raw: &[u8],
        issuer: &str,
        technical: Option<&'static str>,
    ) -> Decision {
        if !crate::native_admission_source::character_commands::command_valid(command)
            || !uuid_v7(operation)
            || !source_authority_valid(source)
            || raw.is_empty()
            || raw.len() > 4096
        {
            return Decision::Transient;
        }
        let Ok(recovery) = authority.record_for(self) else {
            return Decision::Transient;
        };
        let content = if matches!(
            command,
            "LockBazaarCharacter" | "EscrowBazaarItem" | "DeliverBazaarItem"
        ) {
            self.prepare_bazaar_content(authority, proof).await
        } else {
            None
        };
        let request = Request {
            issuer: issuer.to_owned(),
            source: source.to_owned(),
            command: command.to_owned(),
            operation: operation.to_owned(),
            raw: raw.to_vec(),
            generation: recovery.recovery_generation,
            technical,
            content,
        };
        let proof = proof.clone();
        let Ok(pass) = self.try_issue_semantic_pass() else {
            return Decision::Transient;
        };
        let result = pass
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    if !prove_current_incarnation(&mut tx, &proof).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let result = decide(&mut tx, &request).await?;
                    if result.is_some() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await;
        match result {
            Ok(Some(result)) => result,
            _ => Decision::Transient,
        }
    }
}

async fn decide(tx: &mut Tx<'_>, r: &Request) -> Result<Option<Decision>, DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-operation:'||replace($1,'-',''),0))").bind(&r.operation).execute(&mut **tx).await?;
    if let Some(row)=sqlx::query("SELECT intent,receipt,result_code FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid")
        .bind(&r.issuer).bind(&r.operation).fetch_optional(&mut **tx).await? {
        return Ok(Some(if row.try_get::<Vec<u8>,_>("intent")? != r.raw {Decision::Conflict} else { Decision::Publish {receipt:row.try_get("receipt")?,result_code:row.try_get("result_code")?}}));
    }
    if r.issuer==CHARACTER_ISSUER && sqlx::query("SELECT 1 FROM game_character_command_receipts WHERE operation_id=$1::uuid UNION ALL SELECT 1 FROM game_character_operation_receipts WHERE operation_id=$1::uuid UNION ALL SELECT 1 FROM game_character_service_receipts WHERE operation_id=$1::uuid LIMIT 1")
        .bind(&r.operation).fetch_optional(&mut **tx).await?.is_some(){return Ok(Some(Decision::Conflict));}
    if r.issuer == CHARACTER_ISSUER
        && super::character_command::account_deletion::operation_exists(
            tx,
            &super::character_authority::uuid_text(&r.operation)?,
        )
        .await?
    {
        return Ok(Some(Decision::Conflict));
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    if r.issuer == CHARACTER_ISSUER {
        let enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM game_bazaar_configuration WHERE singleton FOR SHARE",
        )
        .fetch_one(&mut **tx)
        .await?;
        if !enabled {
            return Ok(None);
        }
        return transfer::decide(tx, r, now).await;
    }
    let configuration = sqlx::query(
        "SELECT enabled,policy::text FROM game_bazaar_configuration WHERE singleton FOR SHARE",
    )
    .fetch_one(&mut **tx)
    .await?;
    // Rollback stops new work while existing immutable receipts remain replayable.
    if !configuration.try_get::<bool, _>("enabled")? {
        return Ok(None);
    }
    if let Some(code) = r.technical {
        return terminal(tx, r, code, None, now, None).await.map(Some);
    }
    let mut policy: Policy = serde_json::from_str(configuration.try_get("policy")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if let Some(content) = &r.content {
        content.bind(&mut policy);
    }
    if let Some(code) = classify_header(&r.raw, &r.command) {
        return terminal(tx, r, code, None, now, None).await.map(Some);
    }
    let intent = match wire::decode_intent(&r.raw, &r.command, &r.operation) {
        Ok(intent) => intent,
        Err(_) => {
            return terminal(tx, r, "CHAR_CMD_INTENT_INVALID", None, now, None)
                .await
                .map(Some);
        }
    };
    if !intent.time_valid(u64::try_from(now).map_err(|_| DurabilityError::Unavailable)?) {
        let value: serde_json::Value =
            serde_json::from_slice(&r.raw).map_err(|_| DurabilityError::InvalidStoredState)?;
        let expires = value["expires_at_source"]
            .as_str()
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        return terminal(
            tx,
            r,
            if expires <= u64::try_from(now).map_err(|_| DurabilityError::Unavailable)? {
                "CHAR_CMD_INTENT_EXPIRED"
            } else {
                "CHAR_CMD_INTENT_INVALID"
            },
            None,
            now,
            None,
        )
        .await
        .map(Some);
    }
    if !policy.common_restore_scope
        || policy.proof_retention_seconds.is_none_or(|n| n == 0)
        || policy.v2_receipt_bytes != Some(270336)
    {
        return Ok(None);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&r.raw).map_err(|_| DurabilityError::InvalidStoredState)?;
    let seller = field(&value, "seller_account_id")?;
    let world = field(&value, "world_id")?;
    // Shared Content admission precedes portfolio/root locks, matching the
    // existing Content->Character order while allowing concurrent readers.
    sqlx::query("LOCK TABLE game_content_activations IN SHARE MODE")
        .execute(&mut **tx)
        .await?;
    let mut accounts = vec![seller.to_owned()];
    if matches!(
        r.command.as_str(),
        "LockBazaarCharacter" | "BindBazaarCharacterTransfer" | "UnlockBazaarCharacter"
    ) {
        if let Some(escrow) = policy.escrow_account_id.as_ref().filter(|id| uuid_v7(id)) {
            accounts.push(escrow.clone());
        }
        if r.command == "BindBazaarCharacterTransfer"
            && value["funding"]["currency"] == "GOLD"
            && let Some(hold) = value["funding"]["reservation_id"].as_str()
            && let Some(buyer) = sqlx::query_scalar::<_, String>(
                "SELECT buyer_account_id::text FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid",
            )
            .bind(hold)
            .fetch_optional(&mut **tx)
            .await?
        {
            accounts.push(buyer);
        }
    }
    if let Some(buyer) = value
        .get("buyer_account_id")
        .and_then(serde_json::Value::as_str)
    {
        accounts.push(buyer.to_owned());
    }
    accounts.sort();
    accounts.dedup();
    for account in &accounts {
        sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING").bind(account).execute(&mut **tx).await?;
        sqlx::query(
            "SELECT 1 FROM game_character_account_guards WHERE account_id=$1::uuid FOR NO KEY UPDATE",
        )
        .bind(account)
        .fetch_one(&mut **tx)
        .await?;
    }
    super::db::lock_admission_relations(tx).await?;
    let mut characters = Vec::new();
    for name in ["character_id", "seller_character_id", "buyer_character_id"] {
        if let Some(character) = value.get(name).and_then(serde_json::Value::as_str) {
            characters.push(character.to_owned());
        }
    }
    characters.sort();
    characters.dedup();
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=ANY($1::text[]::uuid[]) ORDER BY character_id FOR UPDATE").bind(&characters).fetch_all(&mut **tx).await?;
    let security_account = match r.command.as_str() {
        "LockBazaarCharacter" | "EscrowBazaarItem" => Some(seller),
        "HoldBazaarGold" | "DeliverBazaarItem" => Some(field(&value, "buyer_account_id")?),
        _ => None,
    };
    if let Some(account) = security_account {
        use super::character_authority::{Security, account_security, uuid_text};
        match account_security(tx, &uuid_text(account)?, now).await? {
            Security::Allowed => {}
            Security::Unknown => return Ok(None),
            Security::Refused => {
                return terminal(
                    tx,
                    r,
                    "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE",
                    None,
                    now,
                    Some(world),
                )
                .await
                .map(Some);
            }
        }
    }
    if super::character_authority::current_interpretation(tx)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    if sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=$1::uuid AND scope_kind=1 AND state=1 LIMIT 1 FOR SHARE").bind(world).fetch_optional(&mut **tx).await?.is_none(){return Ok(None);}
    sqlx::query("SAVEPOINT bazaar_effect")
        .execute(&mut **tx)
        .await?;
    let effect = match intent {
        wire::Intent::LockBazaarCharacter(_)
        | wire::Intent::BindBazaarCharacterTransfer(_)
        | wire::Intent::UnlockBazaarCharacter(_) => {
            characters::apply(tx, r, &policy, &value, now).await?
        }
        wire::Intent::EscrowBazaarItem(_)
        | wire::Intent::DeliverBazaarItem(_)
        | wire::Intent::ReturnBazaarItem(_) => items::apply(tx, r, &policy, &value, now).await?,
        wire::Intent::HoldBazaarGold(_)
        | wire::Intent::ReleaseBazaarGold(_)
        | wire::Intent::PayoutBazaarGold(_) => gold::apply(tx, r, &policy, &value, now).await?,
    };
    match effect {
        Effect::Pending => Ok(None),
        Effect::Rejected(code) => {
            sqlx::query("ROLLBACK TO SAVEPOINT bazaar_effect")
                .execute(&mut **tx)
                .await?;
            terminal(tx, r, code, None, now, Some(world))
                .await
                .map(Some)
        }
        Effect::Committed(data) => {
            let data =
                RawValue::from_string(data).map_err(|_| DurabilityError::InvalidStoredState)?;
            let decision = terminal(tx, r, OK, Some(&data), now, Some(world)).await?;
            if let Decision::Publish { receipt, .. } = &decision {
                let digest = Sha256::digest(receipt);
                if r.command == "EscrowBazaarItem" {
                    sqlx::query("UPDATE game_bazaar_escrows SET escrow_receipt_digest=$2 WHERE listing_id=$1::uuid AND last_operation_id=$3::uuid").bind(field(&value,"listing_id")?).bind(digest.as_slice()).bind(&r.operation).execute(&mut **tx).await?;
                }
                if r.command == "DeliverBazaarItem" {
                    gold::delivery_committed(
                        tx,
                        r,
                        &value["funding"],
                        field(&value, "buyer_account_id")?,
                        field(&value, "sale_id")?,
                        &r.operation,
                        digest.as_slice(),
                    )
                    .await?;
                }
            }
            publication::listing_changed(tx, r, &value, now).await?;
            Ok(Some(decision))
        }
    }
}

pub(super) async fn terminal(
    tx: &mut Tx<'_>,
    r: &Request,
    code: &str,
    data: Option<&RawValue>,
    now: i64,
    world: Option<&str>,
) -> Result<Decision, DurabilityError> {
    let receipt = serde_json::to_vec(&Receipt {
        contract_version: 2,
        operation: "PublishCharacterCommandReceiptV1",
        source_authority: &r.source,
        command: &r.command,
        operation_id: &r.operation,
        outcome: if code == OK { "COMMITTED" } else { "REJECTED" },
        result_code: code,
        result_data: data,
        recovery_generation: r.generation.to_string(),
        decided_at: now.to_string(),
    })
    .map_err(|_| DurabilityError::Unavailable)?;
    store(tx, r, code, receipt, now, world).await
}

pub(super) async fn store(
    tx: &mut Tx<'_>,
    r: &Request,
    code: &str,
    receipt: Vec<u8>,
    now: i64,
    world: Option<&str>,
) -> Result<Decision, DurabilityError> {
    if receipt.len() > 270336 || (code != OK && receipt.len() > 1024) {
        return Err(DurabilityError::Unavailable);
    }
    // Registered BazaarAuthorityDecisionV1: two exact SHA256 bytes and the
    // current nonrollback generation, not the potentially large receipt body.
    let mut payload = vec![0x0a, 32];
    payload.extend_from_slice(&Sha256::digest(&r.raw));
    payload.extend_from_slice(&[0x12, 32]);
    payload.extend_from_slice(&Sha256::digest(&receipt));
    payload.push(0x18);
    let mut generation = r.generation;
    while generation >= 128 {
        payload.push((generation as u8 & 127) | 128);
        generation >>= 7;
    }
    payload.push(generation as u8);
    sqlx::query("INSERT INTO game_bazaar_decisions(issuer,operation_id,command,intent,receipt,result_code,recovery_generation,decided_at,event_id,transaction_id,world_id,payload,server_build_id) VALUES($1,$2::uuid,$3,$4,$5,$6,$7::numeric,$8,game_character_uuid_v7(),COALESCE((SELECT transaction_id FROM game_bazaar_item_transitions WHERE issuer=$1 AND operation_id=$2::uuid),game_character_uuid_v7()),$9::uuid,$10,$11)")
        .bind(&r.issuer).bind(&r.operation).bind(&r.command).bind(&r.raw).bind(&receipt).bind(code).bind(r.generation.to_string()).bind(now).bind(world).bind(payload).bind(super::character_authority::SERVER_BUILD_ID).execute(&mut **tx).await?;
    Ok(Decision::Publish {
        receipt,
        result_code: code.to_owned(),
    })
}

/// Existing command consumer delegates transfers here without changing v1 bytes.
pub(super) async fn transfer_input(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    source: &str,
    operation: &str,
    input: Input<'_>,
) -> Decision {
    let (raw, technical) = match input {
        Input::Intent(raw) => (raw.to_vec(), None),
        Input::ReadConflict => (
            super::character_command::read_conflict_binding(TRANSFER_COMMAND),
            Some("CHAR_CMD_OPERATION_CONFLICT"),
        ),
        Input::Unsupported => (
            super::character_command::unsupported_binding(TRANSFER_COMMAND),
            Some("CHAR_CMD_UNSUPPORTED"),
        ),
        Input::Oversized => (
            super::character_command::oversized_binding(TRANSFER_COMMAND),
            Some("CHAR_CMD_INTENT_INVALID"),
        ),
    };
    root.decide_authority_command(
        authority,
        proof,
        source,
        TRANSFER_COMMAND,
        operation,
        &raw,
        CHARACTER_ISSUER,
        technical,
    )
    .await
}

pub(super) enum Effect {
    Committed(String),
    Rejected(&'static str),
    Pending,
}

pub(super) fn field<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a str, DurabilityError> {
    v.get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or(DurabilityError::InvalidStoredState)
}
pub(super) fn number(v: &serde_json::Value, key: &str) -> Result<u64, DurabilityError> {
    field(v, key)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
pub(super) async fn id(tx: &mut Tx<'_>) -> Result<String, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?)
}
pub(super) fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Deserialize)]
struct Header {
    contract_version: Option<serde_json::Value>,
    command: Option<serde_json::Value>,
    variant: Option<serde_json::Value>,
}
fn classify_header(raw: &[u8], command: &str) -> Option<&'static str> {
    let Ok(header) = serde_json::from_slice::<Header>(raw) else {
        return Some("CHAR_CMD_INTENT_INVALID");
    };
    match header.command {
        Some(serde_json::Value::String(found)) if found == command => {}
        Some(serde_json::Value::String(_)) => return Some("CHAR_CMD_OPERATION_CONFLICT"),
        _ => return Some("CHAR_CMD_INTENT_INVALID"),
    }
    if header.contract_version != Some(serde_json::Value::from(2))
        || header.variant != Some(serde_json::Value::String("PLATFORM_BAZAAR".into()))
        || !matches!(
            command,
            "LockBazaarCharacter"
                | "BindBazaarCharacterTransfer"
                | "UnlockBazaarCharacter"
                | "EscrowBazaarItem"
                | "DeliverBazaarItem"
                | "ReturnBazaarItem"
                | "HoldBazaarGold"
                | "ReleaseBazaarGold"
                | "PayoutBazaarGold"
        )
    {
        return Some("CHAR_CMD_UNSUPPORTED");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bazaar_header_preserves_conflict_unsupported_invalid_precedence() {
        let raw =
            br#"{"contract_version":2,"command":"EscrowBazaarItem","variant":"PLATFORM_BAZAAR"}"#;
        assert_eq!(classify_header(raw, "EscrowBazaarItem"), None);
        assert_eq!(
            classify_header(raw, "ReturnBazaarItem"),
            Some("CHAR_CMD_OPERATION_CONFLICT")
        );
        assert_eq!(classify_header(br#"{"contract_version":1,"command":"EscrowBazaarItem","variant":"PLATFORM_BAZAAR"}"#,"EscrowBazaarItem"),Some("CHAR_CMD_UNSUPPORTED"));
        assert_eq!(
            classify_header(
                br#"{"command":"EscrowBazaarItem","command":"EscrowBazaarItem"}"#,
                "EscrowBazaarItem"
            ),
            Some("CHAR_CMD_INTENT_INVALID")
        );
    }
}
