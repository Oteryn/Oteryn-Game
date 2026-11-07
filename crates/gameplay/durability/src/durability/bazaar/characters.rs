//! Bazaar Character custody and immutable signed capture. Unknown product policy
//! and unsupported derived facts stay PENDING; no field is filled from a caller.
use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use oteryn_gameplay_domain::domain::character_build::BuildFormula;
use serde_json::Value;

const STALE: &str = "CHAR_CMD_BAZAAR_STALE_REVISION";
const OBLIGATION: &str = "CHAR_CMD_BAZAAR_OBLIGATION";

pub(super) async fn apply(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    now: i64,
) -> Result<Effect, DurabilityError> {
    match r.command.as_str() {
        "LockBazaarCharacter" => lock(tx, r, p, v, now).await,
        "BindBazaarCharacterTransfer" => bind(tx, r, p, v, now).await,
        "UnlockBazaarCharacter" => unlock(tx, r, v).await,
        _ => Ok(Effect::Rejected("CHAR_CMD_UNSUPPORTED")),
    }
}

async fn lock(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    now: i64,
) -> Result<Effect, DurabilityError> {
    if p.obligations_revision != Some(1)
        || p.cooldown_seconds.is_none()
        || !p.snapshot_survives_transfer
        || p.escrow_account_id.is_none()
        || p.escrow_capacity.is_none()
        || !p.common_restore_scope
    {
        return Ok(Effect::Pending);
    }
    let character = field(v, "character_id")?;
    let seller = field(v, "seller_account_id")?;
    let world = field(v, "world_id")?;
    let listing = field(v, "listing_id")?;
    let Some(root)=sqlx::query("SELECT account_id::text,world_id::text,character_revision::text,lifecycle,name,content_revision FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE")
        .bind(character).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_FOUND"));};
    if root.try_get::<String, _>("account_id")? != seller
        || root.try_get::<String, _>("world_id")? != world
    {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    let revision = root
        .try_get::<String, _>("character_revision")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if revision != number(v, "expected_character_revision")?
        || p.obligations_revision != Some(number(v, "eligibility_revision")?)
    {
        return Ok(Effect::Rejected(STALE));
    }
    if root.try_get::<i16, _>("lifecycle")? != 1 {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));
    }
    if !super::transfer::offline(tx, character).await? {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_IN_SESSION"));
    }
    if sqlx::query("SELECT 1 FROM game_bazaar_locks WHERE (character_id=$1::uuid AND state IN('locked','INVALID')) OR listing_id=$2::uuid LIMIT 1 FOR UPDATE")
        .bind(character).bind(listing).fetch_optional(&mut **tx).await?.is_some(){return Ok(Effect::Rejected(OBLIGATION));}
    match super::super::character_authority::account_security(
        tx,
        &super::super::character_authority::uuid_text(seller)?,
        now,
    )
    .await?
    {
        super::super::character_authority::Security::Allowed => {}
        super::super::character_authority::Security::Refused => {
            return Ok(Effect::Rejected("CHAR_CMD_ACCOUNT_NOT_ELIGIBLE"));
        }
        super::super::character_authority::Security::Unknown => return Ok(Effect::Pending),
    }
    if let Some(code) = obligation(tx, character).await? {
        return Ok(Effect::Rejected(code));
    }
    let last_transfer:Option<i64>=sqlx::query_scalar("SELECT max(d.decided_at) FROM game_bazaar_character_transitions t JOIN game_bazaar_decisions d USING(issuer,operation_id) WHERE t.character_id=$1::uuid AND d.command='TransferCharacterOwnership' AND d.result_code='CHAR_CMD_OK' AND convert_from(d.intent,'UTF8')::jsonb->>'purpose'='BAZAAR_SETTLEMENT'")
        .bind(character).fetch_one(&mut **tx).await?;
    let cooldown = p
        .cooldown_seconds
        .ok_or(DurabilityError::InvalidStoredState)?;
    if last_transfer.is_some_and(|last| {
        u64::try_from(now.saturating_sub(last)).map_or(true, |age| age < cooldown)
    }) {
        return Ok(Effect::Rejected(OBLIGATION));
    }
    let escrow = p
        .escrow_account_id
        .as_deref()
        .ok_or(DurabilityError::InvalidStoredState)?;
    if !uuid_v7(escrow) || escrow == seller {
        return Ok(Effect::Pending);
    }
    let capacity = p
        .escrow_capacity
        .map(u64::from)
        .filter(|n| *n > 0 && *n <= 64);
    let Some(capacity) = capacity else {
        return Ok(Effect::Pending);
    };
    let escrow_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_roots WHERE account_id=$1::uuid")
            .bind(escrow)
            .fetch_one(&mut **tx)
            .await?;
    if u64::try_from(escrow_count).map_err(|_| DurabilityError::InvalidStoredState)? >= capacity {
        return Ok(Effect::Rejected("CHAR_CMD_ACCOUNT_CHARACTER_LIMIT"));
    }
    // Acquire the semantic lock before capture; every capture read remains under
    // the Character root and admission serialization held by this transaction.
    let lock = id(tx).await?;
    let next = revision
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    transition(tx, r, character, seller, seller, 1, 2, revision, next).await?;
    sqlx::query("UPDATE game_character_roots SET lifecycle=2,character_revision=$2::numeric WHERE character_id=$1::uuid").bind(character).bind(next.to_string()).execute(&mut **tx).await?;
    let Some(snapshot) = capture(tx, r, p, v, &root, &lock, next, now).await? else {
        return Ok(Effect::Pending);
    };
    let Some(signed) = sign(tx, p, &snapshot).await? else {
        return Ok(Effect::Pending);
    };
    let digest = Sha256::digest(&snapshot);
    sqlx::query("INSERT INTO game_bazaar_locks(lock_id,listing_id,character_id,seller_account_id,world_id,revision,state,snapshot,snapshot_hash,signed_snapshot,terminal_owner,last_operation_id) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,1,'locked',$6,$7,$8,$4::uuid,$9::uuid)")
        .bind(&lock).bind(listing).bind(character).bind(seller).bind(world).bind(&snapshot).bind(digest.as_slice()).bind(&signed).bind(&r.operation).execute(&mut **tx).await?;
    Ok(Effect::Committed(format!(
        r#"{{"character_id":"{character}","lock_id":"{lock}","lock_revision":"1","eligibility_revision":"{}","signed_snapshot":{}}}"#,
        number(v, "eligibility_revision")?,
        std::str::from_utf8(&signed).map_err(|_| DurabilityError::Unavailable)?
    )))
}

async fn bind(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    now: i64,
) -> Result<Effect, DurabilityError> {
    let purpose = field(v, "purpose")?;
    let returning = purpose == "BAZAAR_ESCROW_RETURN";
    if !p.common_restore_scope
        || (!returning && (p.obligations_revision != Some(1) || !p.snapshot_survives_transfer))
    {
        return Ok(Effect::Pending);
    }
    let character = field(v, "character_id")?;
    let lock_id = field(v, "lock_id")?;
    let listing = field(v, "listing_id")?;
    let seller = field(v, "seller_account_id")?;
    let world = field(v, "world_id")?;
    let Some(lock)=sqlx::query("SELECT revision::text,listing_id::text,seller_account_id::text,world_id::text,state,signed_snapshot,transfer_operation_id::text FROM game_bazaar_locks WHERE lock_id=$1::uuid AND character_id=$2::uuid FOR UPDATE").bind(lock_id).bind(character).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));};
    if lock.try_get::<String, _>("listing_id")? != listing
        || lock.try_get::<String, _>("seller_account_id")? != seller
        || lock.try_get::<String, _>("world_id")? != world
    {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    let revision = lock
        .try_get::<String, _>("revision")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if revision != number(v, "expected_lock_revision")? {
        return Ok(Effect::Rejected(STALE));
    }
    if lock.try_get::<String, _>("state")? != "locked" {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));
    }
    if !super::transfer::offline(tx, character).await? {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_IN_SESSION"));
    }
    if !returning {
        if !signed_snapshot_trusted(tx, p, &lock.try_get::<Vec<u8>, _>("signed_snapshot")?).await? {
            return Ok(Effect::Pending);
        }
        let signed: Signed =
            serde_json::from_slice(&lock.try_get::<Vec<u8>, _>("signed_snapshot")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        let snapshot: Snapshot = serde_json::from_str(signed.snapshot.get())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if snapshot.projection_epoch != r.generation.to_string() {
            return Ok(Effect::Pending);
        }
        if snapshot.character_id != character
            || snapshot.lock_id != lock_id
            || snapshot.listing_id != listing
            || snapshot.seller_account_id != seller
            || snapshot.world_id != world
            || snapshot.eligibility_revision
                != p.obligations_revision
                    .ok_or(DurabilityError::InvalidStoredState)?
                    .to_string()
        {
            return Ok(Effect::Rejected(STALE));
        }
    }
    if let Some(old) = lock.try_get::<Option<String>, _>("transfer_operation_id")? {
        // An expired authorization is not a no-late-commit proof. Only Game's
        // immutable terminal refusal allows replacing it with a fresh binding.
        let refused=sqlx::query("SELECT 1 FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid AND result_code<>'CHAR_CMD_OK' AND recovery_generation=$3::numeric").bind(CHARACTER_ISSUER).bind(&old).bind(r.generation.to_string()).fetch_optional(&mut **tx).await?.is_some();
        if !refused {
            return Ok(Effect::Pending);
        }
    }
    let Some(root)=sqlx::query("SELECT account_id::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(character).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_FOUND"));};
    if root.try_get::<i16, _>("lifecycle")? != 2 {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));
    }
    let owner: String = root.try_get("account_id")?;
    let Some(escrow) = p.escrow_account_id.as_deref().filter(|id| uuid_v7(id)) else {
        return Ok(Effect::Pending);
    };
    if (purpose == "BAZAAR_LISTING_ESCROW" && owner != seller)
        || (purpose != "BAZAAR_LISTING_ESCROW" && owner != escrow)
    {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    if !returning && let Some(code) = obligation(tx, character).await? {
        return Ok(Effect::Rejected(code));
    }
    let transfer = field(v, "transfer_operation_id")?;
    if sqlx::query("SELECT 1 FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid")
        .bind(CHARACTER_ISSUER)
        .bind(transfer)
        .fetch_optional(&mut **tx)
        .await?
        .is_some()
    {
        return Ok(Effect::Rejected("CHAR_CMD_OPERATION_CONFLICT"));
    }
    let digest = decode_hex::<32>(field(v, "transfer_binding_digest")?)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (sale, funding, buyer) = if purpose == "BAZAAR_SETTLEMENT" {
        let sale = field(v, "sale_id")?;
        let funding = v
            .get("funding")
            .ok_or(DurabilityError::InvalidStoredState)?;
        let buyer = if funding["currency"] == "GOLD" {
            let Some(buyer) = sqlx::query_scalar::<_, String>(
                "SELECT buyer_account_id::text FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid",
            )
            .bind(field(funding, "reservation_id")?)
            .fetch_optional(&mut **tx)
            .await?
            else {
                return Ok(Effect::Pending);
            };
            match super::gold::fence_funding(tx, r, funding, &buyer, sale, transfer).await? {
                Effect::Committed(_) => {}
                other => return Ok(other),
            }
            Some(buyer)
        } else {
            None
        };
        (
            Some(sale.to_owned()),
            Some(serde_json::to_vec(funding).map_err(|_| DurabilityError::Unavailable)?),
            buyer,
        )
    } else {
        (None, None, None)
    };
    let next = revision
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    let deadline =
        i64::try_from(number(v, "expires_at_source")?).map_err(|_| DurabilityError::Unavailable)?;
    if deadline <= now {
        return Ok(Effect::Rejected("CHAR_CMD_INTENT_EXPIRED"));
    }
    sqlx::query("UPDATE game_bazaar_locks SET revision=$2::numeric,transfer_operation_id=$3::uuid,transfer_digest=$4,transfer_purpose=$5,transfer_deadline=$6,transfer_sale_id=$7::uuid,transfer_funding=$8,transfer_buyer=$9::uuid,last_operation_id=$10::uuid WHERE lock_id=$1::uuid")
        .bind(lock_id).bind(next.to_string()).bind(transfer).bind(digest.as_slice()).bind(purpose).bind(deadline).bind(sale).bind(funding).bind(buyer).bind(&r.operation).execute(&mut **tx).await?;
    Ok(Effect::Committed(format!(
        r#"{{"character_id":"{character}","lock_id":"{lock_id}","lock_revision":"{next}","transfer_operation_id":"{transfer}","transfer_binding_digest":"{}"}}"#,
        field(v, "transfer_binding_digest")?
    )))
}

async fn unlock(tx: &mut Tx<'_>, r: &Request, v: &Value) -> Result<Effect, DurabilityError> {
    let character = field(v, "character_id")?;
    let lock_id = field(v, "lock_id")?;
    let Some(lock)=sqlx::query("SELECT revision::text,state,listing_id::text,seller_account_id::text,world_id::text,terminal_owner::text,transfer_operation_id::text FROM game_bazaar_locks WHERE lock_id=$1::uuid AND character_id=$2::uuid FOR UPDATE").bind(lock_id).bind(character).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));};
    if lock.try_get::<String, _>("listing_id")? != field(v, "listing_id")?
        || lock.try_get::<String, _>("seller_account_id")? != field(v, "seller_account_id")?
        || lock.try_get::<String, _>("world_id")? != field(v, "world_id")?
    {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    let revision = lock
        .try_get::<String, _>("revision")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if revision != number(v, "expected_lock_revision")? {
        return Ok(Effect::Rejected(STALE));
    }
    if lock.try_get::<String, _>("state")? != "locked" {
        return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"));
    }
    if let Some(old) = lock.try_get::<Option<String>, _>("transfer_operation_id")?
        && sqlx::query("SELECT 1 FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid AND result_code<>'CHAR_CMD_OK' AND recovery_generation=$3::numeric").bind(CHARACTER_ISSUER).bind(old).bind(r.generation.to_string()).fetch_optional(&mut **tx).await?.is_none() {
        return Ok(Effect::Pending);
    }
    let Some(root)=sqlx::query("SELECT account_id::text,character_revision::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(character).fetch_optional(&mut **tx).await? else{return Ok(Effect::Rejected("CHAR_CMD_CHARACTER_NOT_FOUND"));};
    let owner: String = root.try_get("account_id")?;
    if root.try_get::<i16, _>("lifecycle")? != 2
        || lock
            .try_get::<Option<String>, _>("terminal_owner")?
            .as_deref()
            != Some(&owner)
    {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    // The escrow allocation is never a terminal owner. A listing-escrow commit
    // alone cannot authorize unlock; settlement/return must be its last leg.
    let last=sqlx::query("SELECT d.intent,d.recovery_generation::text FROM game_bazaar_character_transitions t JOIN game_bazaar_decisions d USING(issuer,operation_id) WHERE t.character_id=$1::uuid AND d.command='TransferCharacterOwnership' AND d.result_code='CHAR_CMD_OK' ORDER BY t.new_revision DESC LIMIT 1").bind(character).fetch_optional(&mut **tx).await?;
    if let Some(last) = last {
        if last.try_get::<String, _>("recovery_generation")? != r.generation.to_string() {
            return Ok(Effect::Pending);
        }
        let raw: Vec<u8> = last.try_get("intent")?;
        let value: Value =
            serde_json::from_slice(&raw).map_err(|_| DurabilityError::InvalidStoredState)?;
        if value["purpose"] == "BAZAAR_LISTING_ESCROW" {
            return Ok(Effect::Pending);
        }
    } else if owner != field(v, "seller_account_id")? {
        return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));
    }
    if sqlx::query("SELECT 1 FROM game_bazaar_gold_holds WHERE listing_id=$1::uuid AND state='DISPOSITION_PENDING' LIMIT 1 FOR UPDATE").bind(field(v,"listing_id")?).fetch_optional(&mut **tx).await?.is_some(){return Ok(Effect::Pending);}
    let old = root
        .try_get::<String, _>("character_revision")?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let next = old.checked_add(1).ok_or(DurabilityError::Unavailable)?;
    transition(tx, r, character, &owner, &owner, 2, 1, old, next).await?;
    sqlx::query("UPDATE game_character_roots SET lifecycle=1,character_revision=$2::numeric WHERE character_id=$1::uuid").bind(character).bind(next.to_string()).execute(&mut **tx).await?;
    let lock_next = revision
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    sqlx::query("UPDATE game_bazaar_locks SET state='UNLOCKED',revision=$2::numeric,transfer_operation_id=NULL,transfer_digest=NULL,transfer_purpose=NULL,transfer_deadline=NULL,last_operation_id=$3::uuid WHERE lock_id=$1::uuid").bind(lock_id).bind(lock_next.to_string()).bind(&r.operation).execute(&mut **tx).await?;
    Ok(Effect::Committed(format!(
        r#"{{"character_id":"{character}","lock_id":"{lock_id}","lock_revision":"{lock_next}","current_account_id":"{owner}"}}"#
    )))
}

#[allow(
    clippy::too_many_arguments,
    reason = "exact before/after Character root transition proof"
)]
async fn transition(
    tx: &mut Tx<'_>,
    r: &Request,
    character: &str,
    from: &str,
    to: &str,
    old_lifecycle: i16,
    new_lifecycle: i16,
    old: u64,
    new: u64,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_bazaar_character_transitions(operation_id,character_id,old_account,new_account,old_lifecycle,new_lifecycle,old_revision,new_revision,issuer) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5,$6,$7::numeric,$8::numeric,$9)").bind(&r.operation).bind(character).bind(from).bind(to).bind(old_lifecycle).bind(new_lifecycle).bind(old.to_string()).bind(new.to_string()).bind(&r.issuer).execute(&mut **tx).await?;
    Ok(())
}

/// Revision 1 is deliberately conservative: every unresolved obligation refuses
/// the listing. A future obligations profile must get its own revision.
pub(super) async fn obligation(
    tx: &mut Tx<'_>,
    character: &str,
) -> Result<Option<&'static str>, DurabilityError> {
    if sqlx::query(
        "SELECT 1 FROM game_house_ownership WHERE owner_character_id=$1::uuid LIMIT 1 FOR SHARE",
    )
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?
    .is_some()
    {
        return Ok(Some("CHAR_CMD_HOUSING_NOT_PREPARED"));
    }
    let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_market_offers WHERE character_id=$1::uuid AND (state<>5 OR held_credit_gold>0 OR matching_pending)) OR EXISTS(SELECT 1 FROM game_item_character_depot_locations WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_character_inbox_locations WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_mail_operations m JOIN game_mail_outbox o USING(mail_id) WHERE (m.sender_character_id=$1::uuid OR m.recipient_character_id=$1::uuid) AND o.published_at IS NULL) OR EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_character_wheel_state WHERE character_id=$1::uuid AND allocated_total>0) OR EXISTS(SELECT 1 FROM game_character_condition_snapshots WHERE character_id=$1::uuid AND jsonb_array_length(snapshot->'conditions')>0)")
        .bind(character).fetch_one(&mut **tx).await?;
    Ok(blocked.then_some(OBLIGATION))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Skill {
    key: String,
    level: String,
    progress_bps: u16,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    contract_version: u8,
    issuer_authority: String,
    audience: String,
    source_authority: String,
    listing_id: String,
    seller_account_id: String,
    character_id: String,
    world_id: String,
    lock_id: String,
    lock_revision: String,
    character_revision: String,
    eligibility_revision: String,
    projection_epoch: String,
    captured_at: String,
    name: String,
    vocation: u8,
    level: String,
    experience: String,
    created_at: Option<String>,
    health: String,
    health_max: String,
    mana: String,
    mana_max: String,
    capacity_centi_oz: String,
    speed: String,
    blessings: Vec<String>,
    skills: Vec<Skill>,
    world_transfer_eligible: Option<bool>,
    appearance: Option<Value>,
    items: Option<Vec<Value>>,
    store_grants: Option<Vec<String>>,
    outfits: Option<Vec<Value>>,
    mounts: Option<Vec<Value>>,
    achievements: Option<Vec<Value>>,
    quests: Option<Vec<Value>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    snapshot: Box<RawValue>,
    snapshot_hash: String,
    alg: String,
    kid: String,
    signature: String,
}

#[allow(
    clippy::too_many_arguments,
    reason = "locked root and complete immutable capture binding"
)]
async fn capture(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    root: &sqlx::postgres::PgRow,
    lock: &str,
    revision: u64,
    now: i64,
) -> Result<Option<Vec<u8>>, DurabilityError> {
    let character = field(v, "character_id")?;
    let Some(snapshot_source) = p
        .snapshot_source_authority
        .as_deref()
        .filter(|source| source_authority_valid(source))
    else {
        return Ok(None);
    };
    // The same artifact admission used by the item owner proves current native
    // Content is available; a configured path is not by itself Content truth.
    if super::items::admitted_artifact(tx, p, field(v, "world_id")?)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let Some((_, interpretation)) =
        super::super::character_authority::current_interpretation(tx).await?
    else {
        return Ok(None);
    };
    let content: String = root.try_get("content_revision")?;
    if interpretation.revisions()[2] != content {
        return Ok(None);
    }
    let Some(progression)=sqlx::query("SELECT level,total_experience FROM game_character_progression_state WHERE character_id=$1::uuid FOR SHARE").bind(character).fetch_optional(&mut **tx).await? else{return Ok(None);};
    let level: i64 = progression.try_get("level")?;
    let Ok(level) = u32::try_from(level) else {
        return Ok(None);
    };
    let experience: i64 = progression.try_get("total_experience")?;
    let build=sqlx::query("SELECT vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries FROM game_character_build_state WHERE character_id=$1::uuid FOR SHARE")
        .bind(character).fetch_optional(&mut **tx).await?
        .as_ref().map(|row|super::super::character_build::DurableBuildState::stored(row,"vocation","")).transpose()?.unwrap_or_default();
    let vocation = build.vocation().to_owned();
    let vocation_id = match vocation.as_str() {
        "druid" | "elder_druid" => 1,
        "knight" | "elite_knight" => 2,
        "monk" | "exalted_monk" => 3,
        "paladin" | "royal_paladin" => 4,
        "sorcerer" | "master_sorcerer" => 5,
        "none" => 6,
        _ => return Ok(None),
    };
    let Some((health_max, mana_max, _)) =
        oteryn_gameplay_characters::vitals::maxima(&vocation, level, 268435455, 16383)
    else {
        return Ok(None);
    };
    let Ok(capacity) =
        oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(level, &vocation)
    else {
        return Ok(None);
    };
    let Some(speed) = oteryn_gameplay_characters::speed::player_base_speed(level) else {
        return Ok(None);
    };
    let vitals=sqlx::query("SELECT health,mana FROM game_character_last_positions WHERE character_id=$1::uuid FOR SHARE").bind(character).fetch_optional(&mut **tx).await?;
    let (health, mana) = if let Some(vitals) = vitals {
        match (
            vitals.try_get::<Option<i32>, _>("health")?,
            vitals.try_get::<Option<i32>, _>("mana")?,
        ) {
            (Some(h), Some(m)) => {
                let (Ok(h), Ok(m)) = (u32::try_from(h), u32::try_from(m)) else {
                    return Ok(None);
                };
                (h.min(health_max), m.min(mana_max))
            }
            // The existing admission owner defines NULL saved vitals as maxima.
            (None, None) => (health_max, mana_max),
            _ => return Ok(None),
        }
    } else {
        (health_max, mana_max)
    };
    let formula =
        oteryn_gameplay_content::content::training_formula::CompiledTrainingFormula::from_profile(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/native-gameplay/build-training.json"
            )),
            &content,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut skills = Vec::with_capacity(8);
    let mut families = [build.magic(); 8];
    families[1..].copy_from_slice(&build.skills());
    for (family, (key, (stored, progress))) in [
        "magic",
        "fist",
        "club",
        "sword",
        "axe",
        "distance",
        "shielding",
        "fishing",
    ]
    .into_iter()
    .zip(families)
    .enumerate()
    {
        let Some(next) = stored.checked_add(1) else {
            return Ok(None);
        };
        let Some(required) = formula.required(&vocation, family, next).filter(|n| *n > 0) else {
            return Ok(None);
        };
        if progress >= required {
            return Ok(None);
        }
        let bps = (u128::from(progress) * 10000 / u128::from(required)) as u16;
        skills.push(Skill {
            key: key.to_owned(),
            level: stored.to_string(),
            progress_bps: bps,
        });
    }
    skills.sort_by(|a, b| a.key.cmp(&b.key));
    let blessings:Vec<String>=sqlx::query_scalar("SELECT blessing_key FROM game_character_blessings WHERE character_id=$1::uuid ORDER BY blessing_key LIMIT 33 FOR SHARE").bind(character).fetch_all(&mut **tx).await?;
    if blessings.len() > 32 || blessings.iter().any(|key| !key_valid(key, 128)) {
        return Ok(None);
    }
    let epoch = r.generation.to_string();
    let created:Option<i64>=sqlx::query_scalar("SELECT occurred_at/1000 FROM game_character_creation_receipts WHERE character_id=$1::uuid LIMIT 1").bind(character).fetch_optional(&mut **tx).await?;
    let snapshot = Snapshot {
        contract_version: 1,
        issuer_authority: wire::AUDIENCE.to_owned(),
        audience: wire::ISSUER.to_owned(),
        source_authority: snapshot_source.to_owned(),
        listing_id: field(v, "listing_id")?.to_owned(),
        seller_account_id: field(v, "seller_account_id")?.to_owned(),
        character_id: character.to_owned(),
        world_id: field(v, "world_id")?.to_owned(),
        lock_id: lock.to_owned(),
        lock_revision: "1".to_owned(),
        character_revision: revision.to_string(),
        eligibility_revision: number(v, "eligibility_revision")?.to_string(),
        projection_epoch: epoch,
        captured_at: now.to_string(),
        name: root.try_get("name")?,
        vocation: vocation_id,
        level: level.to_string(),
        experience: experience.to_string(),
        created_at: created.filter(|t| *t > 0).map(|t| t.to_string()),
        health: health.to_string(),
        health_max: health_max.to_string(),
        mana: mana.to_string(),
        mana_max: mana_max.to_string(),
        capacity_centi_oz: capacity.to_string(),
        speed: speed.to_string(),
        blessings,
        skills,
        world_transfer_eligible: None,
        appearance: None,
        items: None,
        store_grants: None,
        outfits: None,
        mounts: None,
        achievements: None,
        quests: None,
    };
    let bytes = serde_json::to_vec(&snapshot).map_err(|_| DurabilityError::Unavailable)?;
    Ok((bytes.len() <= 262144 && !bytes.contains(&b'\\')).then_some(bytes))
}

async fn sign(
    tx: &mut Tx<'_>,
    p: &Policy,
    snapshot: &[u8],
) -> Result<Option<Vec<u8>>, DurabilityError> {
    let (Some(path), Some(kid)) = (p.signing_key_file.as_deref(), p.signing_kid.as_deref()) else {
        return Ok(None);
    };
    if !key_valid(kid, 64) || !register_trust(tx, p).await? {
        return Ok(None);
    }
    let Some(public) = p
        .signing_public_keys
        .get(kid)
        .and_then(|key| decode_hex::<32>(key))
    else {
        return Ok(None);
    };
    let Some(seed) = private_seed(path) else {
        return Ok(None);
    };
    let signing = SigningKey::from_bytes(&seed);
    if signing.verifying_key().to_bytes() != public {
        return Ok(None);
    }
    let signing_input = format!(
        "oteryn-v2-bazaar-character-snapshot-v1.{}",
        URL_SAFE_NO_PAD.encode(snapshot)
    );
    let signature = URL_SAFE_NO_PAD.encode(signing.sign(signing_input.as_bytes()).to_bytes());
    let snapshot = RawValue::from_string(
        String::from_utf8(snapshot.to_vec()).map_err(|_| DurabilityError::Unavailable)?,
    )
    .map_err(|_| DurabilityError::Unavailable)?;
    let signed = Signed {
        snapshot_hash: hex(&Sha256::digest(snapshot.get().as_bytes())),
        snapshot,
        alg: "Ed25519".to_owned(),
        kid: kid.to_owned(),
        signature,
    };
    Ok(Some(
        serde_json::to_vec(&signed).map_err(|_| DurabilityError::Unavailable)?,
    ))
}

/// Publication and binding consult current/retiring policy, not only the key
/// that signed the historical capture. Revoked kids can never be re-trusted.
pub(super) async fn signed_snapshot_trusted(
    tx: &mut Tx<'_>,
    p: &Policy,
    raw: &[u8],
) -> Result<bool, DurabilityError> {
    if !register_trust(tx, p).await? {
        return Ok(false);
    }
    let Ok(signed) = serde_json::from_slice::<Signed>(raw) else {
        return Ok(false);
    };
    let Some(public) = p
        .signing_public_keys
        .get(&signed.kid)
        .and_then(|key| decode_hex::<32>(key))
    else {
        return Ok(false);
    };
    Ok(verify_signed(raw, &public))
}

/// Historical decision publication verifies immutable signing provenance only.
/// Revocation fences new sale authority, never rewrites an already committed
/// receipt or prevents its byte-identical reconciliation after recovery.
pub(super) async fn signed_snapshot_provenance(
    tx: &mut Tx<'_>,
    raw: &[u8],
) -> Result<bool, DurabilityError> {
    if raw.len() > 270000 {
        return Ok(false);
    }
    let Ok(signed) = serde_json::from_slice::<Signed>(raw) else {
        return Ok(false);
    };
    let public: Option<Option<Vec<u8>>> = sqlx::query_scalar(
        "SELECT public_key FROM game_bazaar_signing_keys WHERE kid=$1 FOR SHARE",
    )
    .bind(&signed.kid)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(Some(public)) = public else {
        return Ok(false);
    };
    let Ok(public) = <[u8; 32]>::try_from(public.as_slice()) else {
        return Ok(false);
    };
    Ok(verify_signed(raw, &public))
}

fn verify_signed(raw: &[u8], public: &[u8; 32]) -> bool {
    let Ok(signed) = serde_json::from_slice::<Signed>(raw) else {
        return false;
    };
    if raw.len() > 270000
        || raw.contains(&b'\\')
        || signed.alg != "Ed25519"
        || !key_valid(&signed.kid, 64)
        || serde_json::to_vec(&signed).ok().as_deref() != Some(raw)
        || signed.snapshot_hash != hex(&Sha256::digest(signed.snapshot.get().as_bytes()))
    {
        return false;
    }
    let Ok(snapshot) = serde_json::from_str::<Snapshot>(signed.snapshot.get()) else {
        return false;
    };
    if serde_json::to_string(&snapshot).ok().as_deref() != Some(signed.snapshot.get())
        || snapshot.contract_version != 1
        || snapshot.issuer_authority != wire::AUDIENCE
        || snapshot.audience != wire::ISSUER
    {
        return false;
    }
    let positive =
        |value: &str| crate::native_admission_source::character_commands::counter(value).is_some();
    let zero = |value: &str| value == "0" || positive(value);
    if ![
        &snapshot.listing_id,
        &snapshot.seller_account_id,
        &snapshot.character_id,
        &snapshot.world_id,
        &snapshot.lock_id,
    ]
    .into_iter()
    .all(|id| uuid_v7(id))
        || ![
            &snapshot.lock_revision,
            &snapshot.character_revision,
            &snapshot.eligibility_revision,
            &snapshot.projection_epoch,
            &snapshot.captured_at,
            &snapshot.level,
        ]
        .into_iter()
        .all(|n| positive(n))
        || ![
            &snapshot.experience,
            &snapshot.health,
            &snapshot.health_max,
            &snapshot.mana,
            &snapshot.mana_max,
            &snapshot.capacity_centi_oz,
            &snapshot.speed,
        ]
        .into_iter()
        .all(|n| zero(n))
        || !source_authority_valid(&snapshot.source_authority)
        || snapshot
            .created_at
            .as_ref()
            .is_some_and(|time| !positive(time))
        || crate::domain::character_name::CharacterName::parse(&snapshot.name).is_err()
        || !(1..=6).contains(&snapshot.vocation)
        || snapshot.blessings.len() > 32
        || snapshot.blessings.iter().any(|key| !key_valid(key, 128))
        || snapshot.blessings.windows(2).any(|pair| pair[0] >= pair[1])
        || snapshot.skills.len() > 16
        || snapshot
            .skills
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
        || snapshot.skills.iter().any(|skill| {
            !matches!(
                skill.key.as_str(),
                "fist" | "club" | "sword" | "axe" | "distance" | "shielding" | "fishing" | "magic"
            ) || !zero(&skill.level)
                || skill.progress_bps > 10000
        })
        || snapshot.appearance.is_some()
        || snapshot.items.is_some()
        || snapshot.store_grants.is_some()
        || snapshot.outfits.is_some()
        || snapshot.mounts.is_some()
        || snapshot.achievements.is_some()
        || snapshot.quests.is_some()
        || snapshot.health.parse::<u64>().ok() > snapshot.health_max.parse::<u64>().ok()
        || snapshot.mana.parse::<u64>().ok() > snapshot.mana_max.parse::<u64>().ok()
    {
        return false;
    }
    let Ok(decoded) = URL_SAFE_NO_PAD.decode(&signed.signature) else {
        return false;
    };
    if decoded.len() != 64 || URL_SAFE_NO_PAD.encode(&decoded) != signed.signature {
        return false;
    }
    let Ok(signature) = Signature::from_slice(&decoded) else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(public) else {
        return false;
    };
    let input = format!(
        "oteryn-v2-bazaar-character-snapshot-v1.{}",
        URL_SAFE_NO_PAD.encode(signed.snapshot.get().as_bytes())
    );
    key.verify_strict(input.as_bytes(), &signature).is_ok()
}

async fn register_trust(tx: &mut Tx<'_>, p: &Policy) -> Result<bool, DurabilityError> {
    // A restored configuration cannot re-authorize signing keys implicitly. The
    // operator must provision the trust set for this admitted recovery generation.
    let generation: Option<String> = sqlx::query_scalar("SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1 FOR SHARE")
        .fetch_optional(&mut **tx)
        .await?;
    if !matches!(
        (p.signing_trust_generation, generation.as_deref().and_then(|v| v.parse::<u64>().ok())),
        (Some(configured), Some(current)) if configured > 0 && configured == current
    ) {
        return Ok(false);
    }
    if p.signing_public_keys.is_empty()
        || p.signing_public_keys.len() > 2
        || p.revoked_signing_kids.len() > 1024
    {
        return Ok(false);
    }
    for kid in &p.revoked_signing_kids {
        if !key_valid(kid, 64) {
            return Ok(false);
        }
        sqlx::query("INSERT INTO game_bazaar_signing_keys(kid,public_key,revoked) VALUES($1,NULL,true) ON CONFLICT(kid) DO UPDATE SET revoked=true WHERE NOT game_bazaar_signing_keys.revoked").bind(kid).execute(&mut **tx).await?;
    }
    for (kid, key) in &p.signing_public_keys {
        if !key_valid(kid, 64) {
            return Ok(false);
        }
        let Some(public) = decode_hex::<32>(key) else {
            return Ok(false);
        };
        // A separate signature domain does not authorize reusing an admission
        // key. Retired admission keys also remain excluded from Bazaar custody.
        if sqlx::query("SELECT 1 FROM game_durability_admission_signing_trust_guards WHERE public_key=$1 LIMIT 1 FOR SHARE")
            .bind(public.as_slice())
            .fetch_optional(&mut **tx)
            .await?
            .is_some()
        {
            return Ok(false);
        }
        sqlx::query("INSERT INTO game_bazaar_signing_keys(kid,public_key,revoked) VALUES($1,$2,false) ON CONFLICT(kid) DO NOTHING").bind(kid).bind(public.as_slice()).execute(&mut **tx).await?;
        let row = sqlx::query(
            "SELECT public_key,revoked FROM game_bazaar_signing_keys WHERE kid=$1 FOR SHARE",
        )
        .bind(kid)
        .fetch_one(&mut **tx)
        .await?;
        if row.try_get::<bool, _>("revoked")?
            || row.try_get::<Option<Vec<u8>>, _>("public_key")?.as_deref()
                != Some(public.as_slice())
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(unix)]
fn private_seed(path: &str) -> Option<[u8; 32]> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let path = std::path::Path::new(path);
    if !path.is_absolute() {
        return None;
    }
    let before = std::fs::symlink_metadata(path).ok()?;
    if !before.is_file()
        || before.file_type().is_symlink()
        || before.permissions().mode() & 0o077 != 0
        || before.len() != 32
    {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    let opened = file.metadata().ok()?;
    if before.dev() != opened.dev() || before.ino() != opened.ino() {
        return None;
    }
    let canonical = path.canonicalize().ok()?;
    // Signing keys are provisioned outside any source checkout.
    if canonical.ancestors().any(|parent| {
        let marker = parent.join(".git");
        marker.is_file() || (marker.is_dir() && marker.join("HEAD").is_file())
    }) {
        return None;
    }
    let mut seed = [0; 32];
    file.read_exact(&mut seed).ok()?;
    Some(seed)
}

#[cfg(not(unix))]
fn private_seed(_path: &str) -> Option<[u8; 32]> {
    None
}

fn key_valid(key: &str, max: usize) -> bool {
    !key.is_empty()
        && key.len() <= max
        && key.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b"._:/-".contains(&b))
        })
}
fn decode_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let mut value = [0; N];
    for (i, byte) in value.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_fixture_signature_hash_and_domain_are_exact()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixtures: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/contracts/fixtures/bazaar/decisions.json"
        )))?;
        let snapshot = fixtures["vectors"]
            .as_array()
            .ok_or("vectors")?
            .iter()
            .find(|v| v["name"] == "locked_character_snapshot")
            .ok_or("snapshot")?;
        let key = decode_hex::<32>(
            snapshot["fixture_public_key_hex"]
                .as_str()
                .ok_or("public key")?,
        )
        .ok_or("hex")?;
        let raw = snapshot["wire"].as_str().ok_or("wire")?;
        assert!(verify_signed(raw.as_bytes(), &key));
        assert!(!verify_signed(
            raw.replace("Aldric", "Aldrik").as_bytes(),
            &key
        ));
        assert!(!verify_signed(format!("{raw}\n").as_bytes(), &key));
        assert!(!verify_signed(raw.as_bytes(), &[0; 32]));
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod key_file_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn signing_seed_requires_private_regular_external_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory =
            std::env::temp_dir().join(format!("bazaar-signing-key-test-{}", std::process::id()));
        std::fs::create_dir(&directory)?;
        let file = directory.join("seed");
        let link = directory.join("link");
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            std::fs::write(&file, [23; 32])?;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
            assert_eq!(private_seed(file.to_str().ok_or("path")?), Some([23; 32]));
            // Empty sandbox-protection placeholders are not source checkouts;
            // actual Git metadata (including worktree marker files) still denies.
            let marker = directory.join(".git");
            std::fs::create_dir(&marker)?;
            std::fs::write(marker.join("HEAD"), "ref: refs/heads/main\n")?;
            assert_eq!(private_seed(file.to_str().ok_or("path")?), None);
            std::fs::remove_dir_all(marker)?;
            std::os::unix::fs::symlink(&file, &link)?;
            assert_eq!(private_seed(link.to_str().ok_or("path")?), None);
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640))?;
            assert_eq!(private_seed(file.to_str().ok_or("path")?), None);
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
            std::fs::write(&file, [23; 31])?;
            assert_eq!(private_seed(file.to_str().ok_or("path")?), None);
            assert_eq!(private_seed("relative-seed"), None);
            Ok(())
        })();
        std::fs::remove_dir_all(directory)?;
        result
    }
}
