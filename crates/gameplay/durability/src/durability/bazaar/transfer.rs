use super::super::character_authority::{Security, account_security, current_interpretation};
use super::super::db::lock_admission_relations;
use super::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransferIntent {
    contract_version: u8,
    command: String,
    variant: String,
    issuer_authority: String,
    issuer_decision_id: wire::Id,
    source_revision: wire::Counter,
    operation_id: wire::Id,
    pub character_id: wire::Id,
    pub expected_from_account_id: wire::Id,
    pub to_account_id: wire::Id,
    purpose: wire::TransferPurpose,
    issued_at_source: wire::Counter,
    expires_at_source: wire::Counter,
    audience: String,
}

pub fn decode_transfer(raw: &[u8], operation: &str) -> Option<TransferIntent> {
    if raw.is_empty() || raw.len() > 4096 {
        return None;
    }
    let wire: TransferIntent = serde_json::from_slice(raw).ok()?;
    if raw.contains(&b'\\')
        || serde_json::to_vec(&wire).ok()? != raw
        || wire.contract_version != 1
        || wire.command != TRANSFER_COMMAND
        || wire.variant != "PLATFORM_BAZAAR_TRANSFER"
        || wire.issuer_authority != CHARACTER_ISSUER
        || wire.audience != "OTERYN_GAME_CHARACTER_AUTHORITY"
        || wire.operation_id.as_str() != operation
        || wire.expires_at_source.get() <= wire.issued_at_source.get()
        || wire.expires_at_source.get() - wire.issued_at_source.get() > 300
    {
        return None;
    }
    Some(wire)
}

#[derive(Serialize)]
struct Rejected<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    command: &'static str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'a str,
    recovery_generation: String,
    decided_at: String,
}
#[derive(Serialize)]
struct Committed<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    command: &'static str,
    operation_id: &'a str,
    outcome: &'static str,
    result_code: &'static str,
    character_id: &'a str,
    from_account_id: &'a str,
    to_account_id: &'a str,
    character_revision: String,
    projection_epoch: String,
    from_projection_revision: String,
    to_projection_revision: String,
    recovery_generation: String,
    decided_at: String,
}

async fn rejected(
    tx: &mut Tx<'_>,
    r: &Request,
    code: &str,
    now: i64,
) -> Result<Option<Decision>, DurabilityError> {
    let receipt = serde_json::to_vec(&Rejected {
        contract_version: 1,
        operation: "PublishCharacterCommandReceiptV1",
        source_authority: &r.source,
        command: TRANSFER_COMMAND,
        operation_id: &r.operation,
        outcome: "REJECTED",
        result_code: code,
        recovery_generation: r.generation.to_string(),
        decided_at: now.to_string(),
    })
    .map_err(|_| DurabilityError::Unavailable)?;
    store(tx, r, code, receipt, now, None).await.map(Some)
}

pub(super) async fn decide(
    tx: &mut Tx<'_>,
    r: &Request,
    now: i64,
) -> Result<Option<Decision>, DurabilityError> {
    if let Some(code) = r.technical {
        return rejected(tx, r, code, now).await;
    }
    let head: serde_json::Value = match serde_json::from_slice(&r.raw) {
        Ok(head) => head,
        Err(_) => return rejected(tx, r, "CHAR_CMD_INTENT_INVALID", now).await,
    };
    match head.get("command").and_then(serde_json::Value::as_str) {
        Some(TRANSFER_COMMAND) => {}
        Some(_) => return rejected(tx, r, "CHAR_CMD_OPERATION_CONFLICT", now).await,
        None => return rejected(tx, r, "CHAR_CMD_INTENT_INVALID", now).await,
    }
    if head.get("contract_version") != Some(&serde_json::Value::from(1))
        || head.get("variant").and_then(serde_json::Value::as_str)
            != Some("PLATFORM_BAZAAR_TRANSFER")
    {
        return rejected(tx, r, "CHAR_CMD_UNSUPPORTED", now).await;
    }
    let Some(intent) = decode_transfer(&r.raw, &r.operation) else {
        return rejected(tx, r, "CHAR_CMD_INTENT_INVALID", now).await;
    };
    if intent.expires_at_source.get() <= now as u64 {
        return rejected(tx, r, "CHAR_CMD_INTENT_EXPIRED", now).await;
    }
    if intent.issued_at_source.get() > (now as u64).saturating_add(5) {
        return rejected(tx, r, "CHAR_CMD_INTENT_INVALID", now).await;
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-transfer-revision:'||$1,0))").bind(intent.source_revision.get().to_string()).execute(&mut **tx).await?;
    if sqlx::query("SELECT 1 FROM game_bazaar_decisions WHERE issuer=$1 AND command='TransferCharacterOwnership' AND source_revision=$2::numeric AND result_code<>'CHAR_CMD_OPERATION_CONFLICT' LIMIT 1").bind(CHARACTER_ISSUER).bind(intent.source_revision.get().to_string()).fetch_optional(&mut **tx).await?.is_some(){return rejected(tx,r,"CHAR_CMD_OPERATION_CONFLICT",now).await;}
    let from = intent.expected_from_account_id.as_str();
    let to = intent.to_account_id.as_str();
    let character = intent.character_id.as_str();
    // Only immutable identity metadata is read before portfolio serialization.
    // It is re-read under the Character root below; no row-level hold is taken
    // ahead of the bank owner's balance -> hold order.
    let metadata=sqlx::query("SELECT seller_account_id::text,transfer_buyer::text FROM game_bazaar_locks WHERE character_id=$1::uuid AND state='locked'")
        .bind(character).fetch_optional(&mut **tx).await?;
    let mut accounts = vec![from.to_owned(), to.to_owned()];
    if let Some(metadata) = &metadata {
        accounts.push(metadata.try_get::<String, _>("seller_account_id")?);
        if let Some(buyer) = metadata.try_get::<Option<String>, _>("transfer_buyer")? {
            accounts.push(buyer);
        }
    }
    accounts.sort();
    accounts.dedup();
    for account in &accounts {
        sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING").bind(account).execute(&mut **tx).await?;
        sqlx::query(
            "SELECT 1 FROM game_character_account_guards WHERE account_id=$1::uuid FOR UPDATE",
        )
        .bind(account)
        .fetch_one(&mut **tx)
        .await?;
    }
    lock_admission_relations(tx).await?;
    let Some(root)=sqlx::query("SELECT account_id::text,world_id::text,character_revision::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(character).fetch_optional(&mut **tx).await? else{return rejected(tx,r,"CHAR_CMD_CHARACTER_NOT_FOUND",now).await;};
    if root.try_get::<String, _>("account_id")? != from || from == to {
        return rejected(tx, r, "CHAR_CMD_OWNER_MISMATCH", now).await;
    }
    if !offline(tx, character).await? {
        return rejected(tx, r, "CHAR_CMD_CHARACTER_IN_SESSION", now).await;
    }
    let lifecycle: i16 = root.try_get("lifecycle")?;
    let world: String = root.try_get("world_id")?;
    let revision: u64 = root
        .try_get::<String, _>("character_revision")?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let lock=sqlx::query("SELECT lock_id::text,revision::text,listing_id::text,world_id::text,seller_account_id::text,signed_snapshot,transfer_operation_id::text,transfer_digest,transfer_purpose,transfer_deadline,transfer_sale_id::text,transfer_funding,transfer_buyer::text FROM game_bazaar_locks WHERE character_id=$1::uuid AND state='locked' FOR UPDATE").bind(character).fetch_optional(&mut **tx).await?;
    if lifecycle != 1 && lifecycle != 2 {
        return rejected(tx, r, "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE", now).await;
    }
    if lifecycle == 1 && lock.is_some() {
        return Ok(None);
    }
    let mut settlement: Option<(serde_json::Value, String, String)> = None;
    let mut escrow_capacity = None;
    if lifecycle == 2 {
        let Some(lock) = &lock else {
            return rejected(tx, r, "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE", now).await;
        };
        let purpose =
            serde_json::to_string(&intent.purpose).map_err(|_| DurabilityError::Unavailable)?;
        if lock
            .try_get::<Option<String>, _>("transfer_operation_id")?
            .as_deref()
            != Some(&r.operation)
            || lock
                .try_get::<Option<Vec<u8>>, _>("transfer_digest")?
                .as_deref()
                != Some(Sha256::digest(&r.raw).as_slice())
            || lock
                .try_get::<Option<String>, _>("transfer_purpose")?
                .as_deref()
                != Some(purpose.trim_matches('"'))
            || lock
                .try_get::<Option<i64>, _>("transfer_deadline")?
                .is_none_or(|deadline| deadline <= now)
        {
            return rejected(tx, r, "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE", now).await;
        }
        let configuration: String =
            sqlx::query_scalar("SELECT policy FROM game_character_service_bazaar_policy()")
                .fetch_one(&mut **tx)
                .await?;
        let policy: Policy = serde_json::from_str(&configuration)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let returning = intent.purpose == wire::TransferPurpose::BazaarEscrowReturn;
        if !policy.common_restore_scope
            || policy
                .proof_retention_seconds
                .is_none_or(|seconds| seconds == 0)
            || policy.v2_receipt_bytes != Some(270336)
            || policy
                .escrow_account_id
                .as_deref()
                .filter(|account| uuid_v7(account))
                .is_none()
            || (!returning
                && (policy.obligations_revision != Some(1)
                    || policy.cooldown_seconds.is_none()
                    || !policy.snapshot_survives_transfer))
        {
            return Ok(None);
        }
        // A safe return is authorized by Game's retained custody binding and
        // current original-owner guards. It does not certify an auction snapshot.
        let attestation = if returning {
            None
        } else {
            let signed: Vec<u8> = lock.try_get("signed_snapshot")?;
            if !super::characters::signed_snapshot_trusted(tx, &policy, &signed).await? {
                return Ok(None);
            }
            Some(
                serde_json::from_slice::<serde_json::Value>(&signed)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
        };
        if let Some(attestation) = &attestation {
            let snapshot = &attestation["snapshot"];
            if snapshot["projection_epoch"].as_str() != Some(r.generation.to_string().as_str()) {
                return Ok(None);
            }
            if snapshot["character_id"].as_str() != Some(character)
                || snapshot["listing_id"].as_str()
                    != Some(lock.try_get::<String, _>("listing_id")?.as_str())
                || snapshot["lock_id"].as_str()
                    != Some(lock.try_get::<String, _>("lock_id")?.as_str())
                || snapshot["world_id"].as_str() != Some(world.as_str())
                || snapshot["source_authority"].as_str()
                    != policy.snapshot_source_authority.as_deref()
                || snapshot["eligibility_revision"].as_str() != Some("1")
            {
                return rejected(tx, r, "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE", now).await;
            }
        }
        let seller: String = lock.try_get("seller_account_id")?;
        let valid = match intent.purpose {
            wire::TransferPurpose::BazaarListingEscrow => {
                from == seller && policy.escrow_account_id.as_deref() == Some(to)
            }
            wire::TransferPurpose::BazaarSettlement => {
                policy.escrow_account_id.as_deref() == Some(from) && to != seller
            }
            wire::TransferPurpose::BazaarEscrowReturn => {
                policy.escrow_account_id.as_deref() == Some(from) && to == seller
            }
        };
        if !valid {
            return rejected(tx, r, "CHAR_CMD_OWNER_MISMATCH", now).await;
        }
        if !accounts.contains(&seller)
            || lock.try_get::<String, _>("world_id")? != world
            || attestation.as_ref().is_some_and(|a| {
                a["snapshot"]["seller_account_id"].as_str() != Some(seller.as_str())
            })
        {
            return Ok(None);
        }
        if !returning && let Some(code) = super::characters::obligation(tx, character).await? {
            // v1 keeps its accepted result vocabulary. The v2 obligation
            // refinement cannot change an existing ownership receipt shape.
            let code = if code == "CHAR_CMD_BAZAAR_OBLIGATION" {
                "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"
            } else {
                code
            };
            return rejected(tx, r, code, now).await;
        }
        if intent.purpose == wire::TransferPurpose::BazaarListingEscrow {
            escrow_capacity = policy
                .escrow_capacity
                .filter(|capacity| *capacity > 0 && *capacity <= 64);
            if escrow_capacity.is_none() {
                return Ok(None);
            }
        }
        if intent.purpose == wire::TransferPurpose::BazaarSettlement {
            let Some(sale) = lock.try_get::<Option<String>, _>("transfer_sale_id")? else {
                return Ok(None);
            };
            let Some(raw_funding) = lock.try_get::<Option<Vec<u8>>, _>("transfer_funding")? else {
                return Ok(None);
            };
            let funding: serde_json::Value = serde_json::from_slice(&raw_funding)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if funding["currency"] == "GOLD" {
                if policy.gold_fee_bps != Some(250) || !policy.reserve_incoming_headroom {
                    return Ok(None);
                }
                if lock
                    .try_get::<Option<String>, _>("transfer_buyer")?
                    .as_deref()
                    != Some(to)
                {
                    return rejected(tx, r, "CHAR_CMD_OWNER_MISMATCH", now).await;
                }
                match funded_settlement(
                    tx,
                    r,
                    &funding,
                    to,
                    &seller,
                    &world,
                    &lock.try_get::<String, _>("listing_id")?,
                    &sale,
                )
                .await?
                {
                    Some(true) => {}
                    Some(false) => {
                        return rejected(tx, r, "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE", now).await;
                    }
                    None => return Ok(None),
                }
            } else if funding["currency"] != "COINS" || !funding["hold_revision"].is_null() {
                return rejected(tx, r, "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE", now).await;
            }
            settlement = Some((funding, to.to_owned(), sale));
        }
    }
    if intent.purpose != wire::TransferPurpose::BazaarEscrowReturn {
        match account_security(tx, &super::super::character_authority::uuid_text(to)?, now).await? {
            Security::Allowed => {}
            Security::Refused => {
                return rejected(tx, r, "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE", now).await;
            }
            Security::Unknown => return Ok(None),
        }
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_live_roots WHERE account_id=$1::uuid",
    )
    .bind(to)
    .fetch_one(&mut **tx)
    .await?;
    if count >= escrow_capacity.map(i64::from).unwrap_or(64) {
        return rejected(tx, r, "CHAR_CMD_ACCOUNT_CHARACTER_LIMIT", now).await;
    }
    // Housing remains with its owner. Until the owning domain exposes a matching
    // prepare-consumption API, no house-carrying Character may cross accounts.
    if sqlx::query_scalar::<_, bool>("SELECT game_character_service_has_house($1::uuid)")
        .bind(character)
        .fetch_one(&mut **tx)
        .await?
    {
        return rejected(tx, r, "CHAR_CMD_HOUSING_NOT_PREPARED", now).await;
    }
    if current_interpretation(tx).await?.is_none() {
        return Ok(None);
    }
    if sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=$1::uuid AND scope_kind=1 AND state=1 LIMIT 1 FOR SHARE").bind(&world).fetch_optional(&mut **tx).await?.is_none(){return Ok(None);}
    let next = revision
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    sqlx::query("INSERT INTO game_bazaar_character_transitions(operation_id,character_id,old_account,new_account,old_lifecycle,new_lifecycle,old_revision,new_revision,issuer) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5,$5,$6::numeric,$7::numeric,$8)").bind(&r.operation).bind(character).bind(from).bind(to).bind(lifecycle).bind(revision.to_string()).bind(next.to_string()).bind(&r.issuer).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_character_roots SET account_id=$2::uuid,character_revision=$3::numeric WHERE character_id=$1::uuid").bind(character).bind(to).bind(next.to_string()).execute(&mut **tx).await?;
    // Retained old owner evidence cannot admit or authorize a stale generation.
    sqlx::query(
        "DELETE FROM game_durability_admission_character_guards WHERE character_id=$1::uuid",
    )
    .bind(character)
    .execute(&mut **tx)
    .await?;
    if let Some(lock) = &lock {
        sqlx::query("UPDATE game_bazaar_locks SET revision=revision+1,transfer_operation_id=NULL,transfer_digest=NULL,transfer_purpose=NULL,transfer_deadline=NULL,terminal_owner=$2::uuid,last_operation_id=$3::uuid WHERE lock_id=$1::uuid AND state='locked'").bind(lock.try_get::<String, _>("lock_id")?).bind(to).bind(&r.operation).execute(&mut **tx).await?;
    }
    let projection=sqlx::query("SELECT e.projection_epoch::text,(SELECT projection_revision::text FROM game_character_account_projections WHERE account_id=$1::uuid) AS from_revision,(SELECT projection_revision::text FROM game_character_account_projections WHERE account_id=$2::uuid) AS to_revision FROM game_character_account_projection_epoch e WHERE epoch_scope=1").bind(from).bind(to).fetch_one(&mut **tx).await?;
    let receipt = serde_json::to_vec(&Committed {
        contract_version: 1,
        operation: "PublishCharacterCommandReceiptV1",
        source_authority: &r.source,
        command: TRANSFER_COMMAND,
        operation_id: &r.operation,
        outcome: "COMMITTED",
        result_code: OK,
        character_id: character,
        from_account_id: from,
        to_account_id: to,
        character_revision: next.to_string(),
        projection_epoch: projection.try_get("projection_epoch")?,
        from_projection_revision: projection.try_get("from_revision")?,
        to_projection_revision: projection.try_get("to_revision")?,
        recovery_generation: r.generation.to_string(),
        decided_at: now.to_string(),
    })
    .map_err(|_| DurabilityError::Unavailable)?;
    let decision = store(tx, r, OK, receipt, now, Some(&world)).await?;
    if let (Some((funding, buyer, sale)), Decision::Publish { receipt, .. }) =
        (&settlement, &decision)
    {
        super::gold::delivery_committed(
            tx,
            r,
            funding,
            buyer,
            sale,
            &r.operation,
            &Sha256::digest(receipt),
        )
        .await?;
    }
    if lock.is_some() {
        let value: serde_json::Value =
            serde_json::from_slice(&r.raw).map_err(|_| DurabilityError::InvalidStoredState)?;
        super::publication::listing_changed(tx, r, &value, now).await?;
    }
    Ok(Some(decision))
}

pub(super) async fn offline(tx: &mut Tx<'_>, character: &str) -> Result<bool, DurabilityError> {
    Ok(sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND session_state IN(1,2) UNION ALL SELECT 1 FROM game_durability_admission_character_guards WHERE character_id=$1::uuid AND holder_game_session_id IS NOT NULL LIMIT 1").bind(character).fetch_optional(&mut **tx).await?.is_none())
}

#[allow(
    clippy::too_many_arguments,
    reason = "full immutable funded Character settlement binding"
)]
async fn funded_settlement(
    tx: &mut Tx<'_>,
    r: &Request,
    funding: &serde_json::Value,
    buyer: &str,
    seller: &str,
    world: &str,
    listing: &str,
    sale: &str,
) -> Result<Option<bool>, DurabilityError> {
    // Use the single native balance authority and the same deterministic order
    // as Hold/Release/Payout. This path validates an existing disposition; it
    // never creates or renews a hold from an unread v1 ownership intent.
    for account in if buyer < seller {
        [buyer, seller]
    } else {
        [seller, buyer]
    } {
        if sqlx::query("SELECT 1 FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid FOR UPDATE").bind(account).bind(world).fetch_optional(&mut **tx).await?.is_none(){return Ok(None);}
    }
    let Some(hold)=sqlx::query("SELECT buyer_account_id::text,seller_account_id::text,world_id::text,listing_id::text,revision::text,state,amount,incoming_net,sale_id::text,gross_amount,delivery_operation_id::text FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid FOR UPDATE").bind(field(funding,"reservation_id")?).fetch_optional(&mut **tx).await? else{return Ok(None);};
    let gross = number(funding, "gross_amount")?;
    let fee = u128::from(gross)
        .checked_mul(250)
        .ok_or(DurabilityError::Unavailable)?
        .div_ceil(10000);
    let Some(net) = u128::from(gross)
        .checked_sub(fee)
        .and_then(|net| u64::try_from(net).ok())
        .filter(|net| *net > 0)
    else {
        return Ok(Some(false));
    };
    if hold.try_get::<String, _>("buyer_account_id")? != buyer
        || hold.try_get::<String, _>("seller_account_id")? != seller
        || hold.try_get::<String, _>("world_id")? != world
        || hold.try_get::<String, _>("listing_id")? != listing
        || hold.try_get::<String, _>("revision")? != field(funding, "hold_revision")?
        || hold.try_get::<String, _>("state")? != "DISPOSITION_PENDING"
        || hold.try_get::<Option<String>, _>("sale_id")?.as_deref() != Some(sale)
        || hold
            .try_get::<Option<String>, _>("delivery_operation_id")?
            .as_deref()
            != Some(r.operation.as_str())
        || hold.try_get::<Option<i64>, _>("gross_amount")? != i64::try_from(gross).ok()
        || u64::try_from(hold.try_get::<i64, _>("amount")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?
            < gross
        || u64::try_from(hold.try_get::<i64, _>("incoming_net")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?
            < net
    {
        return Ok(Some(false));
    }
    let solvent:bool=sqlx::query_scalar("SELECT (SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$3::uuid) BETWEEN (SELECT coalesce(sum(amount),0) FROM game_bazaar_gold_holds WHERE buyer_account_id=$1::uuid AND world_id=$3::uuid) AND 999999999999 AND (SELECT balance FROM game_account_bank_balances WHERE account_id=$2::uuid AND world_id=$3::uuid)+(SELECT coalesce(sum(incoming_net),0) FROM game_bazaar_gold_holds WHERE seller_account_id=$2::uuid AND world_id=$3::uuid)<=999999999999").bind(buyer).bind(seller).bind(world).fetch_one(&mut **tx).await?;
    Ok(Some(solvent))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v1_transfer_fixture_preserves_exact_unexpired_or_expired_intent_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/bazaar/decisions.json"
        )))?;
        let vector = fixtures["vectors"]
            .as_array()
            .ok_or("vectors")?
            .iter()
            .find(|v| v["name"] == "character_settlement_intent")
            .ok_or("v1 transfer")?;
        let raw = vector["wire"].as_str().ok_or("wire")?;
        let value: serde_json::Value = serde_json::from_str(raw)?;
        let operation = value["operation_id"].as_str().ok_or("operation")?;
        let intent = decode_transfer(raw.as_bytes(), operation).ok_or("decoded v1")?;
        assert_eq!(serde_json::to_vec(&intent)?, raw.as_bytes());
        assert!(decode_transfer(raw.as_bytes(), "0192a000-0000-7000-8000-000000000099").is_none());
        for invalid in [
            format!("{raw}\n"),
            raw.replace("\"contract_version\":1", "\"contract_version\":2"),
            raw.replace("PLATFORM_BAZAAR_TRANSFER", "PLATFORM_USER_CREATE"),
            raw.replace("BAZAAR_SETTLEMENT", "TRANSFER"),
            raw.replace("1790000300", "1790000301"),
            raw.replace("1790000300", "1790000000"),
            raw.replace("\"source_revision\":\"19\"", "\"source_revision\":\"019\""),
            raw.replace("}", ",\"extra\":1}"),
            raw.replace("\"purpose\":", "\"purpo\\u0073e\":"),
        ] {
            assert!(decode_transfer(invalid.as_bytes(), operation).is_none());
        }
        Ok(())
    }
}
