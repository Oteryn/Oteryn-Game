use super::*;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Postgres, Transaction};

fn envelope(row: &sqlx::postgres::PgRow) -> Result<SignedEnvelope, DurabilityError> {
    Ok(SignedEnvelope {
        principal: row.try_get("principal")?,
        canonical: row.try_get("canonical")?,
        signature: row.try_get("signature")?,
    })
}
fn same(row: &sqlx::postgres::PgRow, auth: &Authenticated) -> Result<bool, DurabilityError> {
    Ok(envelope(row)? == auth.envelope
        && row.try_get::<String, _>("roster_sha256")? == auth.roster
        && row.try_get::<String, _>("key_fingerprint")? == auth.fingerprint)
}
async fn retained(
    tx: &mut PgConnection,
    trust: &StaffTrust,
    auth: &Authenticated,
) -> Result<Option<Option<Vec<u8>>>, DurabilityError> {
    let row=sqlx::query("SELECT l.*,o.outcome,o.outcome_sha256 FROM game_staff_control_links l JOIN game_staff_operations o USING(environment,operation_id) WHERE l.environment=$1 AND l.operation_id=$2::uuid AND l.kind=$3")
        .bind(trust.environment()).bind(auth.operation()).bind(match auth.message {Message::Request(_)=>"REQUEST",Message::Receipt(_)=>"RECEIPT"}).fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if !same(&row, auth)?
        || row.try_get::<String, _>("link_sha256")? != digest(&auth.envelope.canonical)
    {
        return Ok(Some(None));
    }
    let outcome: Vec<u8> = row.try_get("outcome")?;
    oteryn_protocol_oteryn::staff::decode_result(&outcome)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if row.try_get::<String, _>("outcome_sha256")? != digest(&outcome) {
        return Err(DurabilityError::InvalidStoredState);
    }
    if auth.seq() != Some(row.try_get::<i64, _>("control_seq")? as u64) {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let Message::Receipt(receipt) = &auth.message {
        if !receipt_binds(tx,trust,&auth.envelope,receipt,&auth.fingerprint,false).await? || !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_staff_action_receipts WHERE environment=$1 AND operation_id=$2::uuid AND control_seq=$3)").bind(trust.environment()).bind(auth.operation()).bind(auth.seq().ok_or(DurabilityError::InvalidStoredState)? as i64).fetch_one(&mut *tx).await? {return Err(DurabilityError::InvalidStoredState);}
        return Ok(Some(Some(result(
            3,
            auth.seq().ok_or(DurabilityError::InvalidStoredState)?,
            &auth.envelope.canonical,
            0,
            wire_result::StaffCode::Ok,
        )?)));
    }
    if let Message::Request(request) = &auth.message {
        let mirror=sqlx::query("SELECT o.*,a.payload FROM game_staff_operations o LEFT JOIN game_staff_action_outbox a USING(environment,operation_id) WHERE o.environment=$1 AND o.operation_id=$2::uuid").bind(trust.environment()).bind(auth.operation()).fetch_one(&mut *tx).await?;
        if !operation_matches(tx, trust, request, &mirror, &auth.envelope, &outcome).await? {
            return Err(DurabilityError::InvalidStoredState);
        }
        let payload: Option<Vec<u8>> = mirror.try_get("payload")?;
        if let Some(payload) = payload {
            if payload != audit_payload(&auth.envelope, &auth.fingerprint, &auth.roster, &outcome)?
            {
                return Err(DurabilityError::InvalidStoredState);
            }
        } else if !audit_expired(tx, trust, auth.operation()).await? {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(Some(Some(outcome)))
}
fn result(
    disposition: u64,
    seq: u64,
    bytes: &[u8],
    until: u64,
    code: wire_result::StaffCode,
) -> Result<Vec<u8>, DurabilityError> {
    encode_result(&StaffResult {
        code,
        disposition,
        control_seq: seq,
        link_sha256: Sha256::digest(bytes).to_vec(),
        mute_until_ms: until,
    })
    .map_err(|_| DurabilityError::InvalidStoredState)
}
/// Check every retained link against the restart-pinned activation history. Bounded
/// pages, no trusting DB-provided signers. A gap, fork or unknown activation stops all work.
pub(super) async fn verify_chain(
    tx: &mut PgConnection,
    trust: &Arc<StaffTrust>,
    head_seq: i64,
    head_digest: &str,
) -> Result<bool, DurabilityError> {
    let mut seq = 0i64;
    let mut previous = "0".repeat(64);
    let mut pending: Option<String> = None;
    while seq < head_seq {
        let rows=sqlx::query("SELECT *,replace(operation_id::text,'-','') AS operation_hex FROM game_staff_control_links WHERE environment=$1 AND control_seq>$2 AND control_seq<=$3 ORDER BY control_seq LIMIT 64")
            .bind(trust.environment()).bind(seq).bind(head_seq).fetch_all(&mut *tx).await?;
        if rows.is_empty() {
            return Ok(false);
        }
        for row in rows {
            let next: i64 = row.try_get("control_seq")?;
            let kind: String = row.try_get("kind")?;
            let e = envelope(&row)?;
            let sha = digest(&e.canonical);
            if next != seq + 1
                || row.try_get::<String, _>("previous_link_sha256")? != previous
                || row.try_get::<String, _>("link_sha256")? != sha
            {
                return Ok(false);
            }
            let Some((roster, role, fingerprint)) = trust
                .clone()
                .authenticate_async(e.clone(), kind == "RECEIPT", next as u64)
                .await
            else {
                return Ok(false);
            };
            let activation: String = row.try_get("activation_sha256")?;
            if trust.activation_at(next as u64) != Some((activation.as_str(), roster.as_str()))
                || row.try_get::<String, _>("roster_sha256")? != roster
                || row.try_get::<String, _>("key_fingerprint")? != fingerprint
            {
                return Ok(false);
            }
            let (signed_seq, signed_previous) = match kind.as_str() {
                "ACTIVATION" => {
                    if pending.is_some()
                        || row.try_get::<Option<String>, _>("operation_hex")?.is_some()
                    {
                        return Ok(false);
                    }
                    let Ok(r) = decode::<rules::Activation>(&e.canonical) else {
                        return Ok(false);
                    };
                    if sha != activation
                        || r.environment != trust.environment()
                        || r.roster_sha256 != roster
                    {
                        return Ok(false);
                    }
                    (r.control_seq, r.previous_link_sha256)
                }
                "REQUEST" => {
                    if pending.is_some() {
                        return Ok(false);
                    }
                    let Ok(r) = decode::<rules::Request>(&e.canonical) else {
                        return Ok(false);
                    };
                    if row
                        .try_get::<Option<String>, _>("operation_hex")?
                        .as_deref()
                        != Some(r.operation_id.as_str())
                        || !r.valid(role)
                        || r.environment != trust.environment()
                        || r.activation_sha256 != activation
                        || r.roster_sha256 != roster
                    {
                        return Ok(false);
                    }
                    let mirror=sqlx::query("SELECT o.*,a.payload,a.expires_at_ms FROM game_staff_operations o LEFT JOIN game_staff_action_outbox a USING(environment,operation_id) WHERE o.environment=$1 AND o.operation_id=$2::uuid AND o.control_seq=$3")
                        .bind(trust.environment()).bind(&r.operation_id).bind(next).fetch_optional(&mut *tx).await?;
                    let Some(mirror) = mirror else {
                        return Ok(false);
                    };
                    let outcome: Vec<u8> = mirror.try_get("outcome")?;
                    if mirror.try_get::<String, _>("outcome_sha256")? != digest(&outcome) {
                        return Ok(false);
                    }
                    if !operation_matches(tx, trust, &r, &mirror, &e, &outcome).await? {
                        return Ok(false);
                    }
                    if mirror.try_get::<bool, _>("requires_receipt")? {
                        pending = Some(r.operation_id.clone());
                    }
                    // Evidence cannot disappear while it is within the execution+retention window.
                    let payload: Option<Vec<u8>> = mirror.try_get("payload")?;
                    if let Some(payload) = payload {
                        if payload != audit_payload(&e, &fingerprint, &roster, &outcome)? {
                            return Ok(false);
                        }
                    } else if !audit_expired(tx, trust, &r.operation_id).await? {
                        return Ok(false);
                    }

                    (r.control_seq, r.previous_link_sha256)
                }
                "RECEIPT" => {
                    let Ok(r) = decode::<rules::Receipt>(&e.canonical) else {
                        return Ok(false);
                    };
                    if r.environment != trust.environment()
                        || r.activation_sha256 != activation
                        || r.roster_sha256 != roster
                    {
                        return Ok(false);
                    }
                    if row
                        .try_get::<Option<String>, _>("operation_hex")?
                        .as_deref()
                        != Some(r.operation_id.as_str())
                        || (r.disposition == rules::ReceiptDisposition::Committed
                            && pending.as_ref() != Some(&r.operation_id))
                        || (r.disposition == rules::ReceiptDisposition::Refused
                            && pending.is_some())
                    {
                        return Ok(false);
                    }
                    if !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_staff_action_receipts WHERE environment=$1 AND operation_id=$2::uuid AND control_seq=$3)").bind(trust.environment()).bind(&r.operation_id).bind(next).fetch_one(&mut *tx).await? {return Ok(false);}
                    if !receipt_binds(tx, trust, &e, &r, &fingerprint, false).await? {
                        return Ok(false);
                    }
                    pending = None;
                    (r.control_seq, r.previous_link_sha256)
                }
                _ => return Ok(false),
            };
            if rules::decimal(&signed_seq) != Some(next as u64) || signed_previous != previous {
                return Ok(false);
            }
            seq = next;
            previous = sha;
        }
    }
    let orphaned:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_control_links WHERE environment=$1 AND control_seq>$2) OR EXISTS(SELECT 1 FROM game_staff_operations o LEFT JOIN game_staff_control_links l ON l.environment=o.environment AND l.control_seq=o.control_seq AND l.kind='REQUEST' AND l.operation_id=o.operation_id WHERE o.environment=$1 AND l.control_seq IS NULL) OR EXISTS(SELECT 1 FROM game_character_sanctions s LEFT JOIN game_staff_operations o ON o.environment=s.environment AND o.operation_id=s.sanction_id WHERE s.environment=$1 AND (o.operation_id IS NULL OR o.kind<>'MUTE' OR o.result<>1))")
        .bind(trust.environment()).bind(head_seq).fetch_one(&mut *tx).await?;
    let stored_pending:Option<String>=sqlx::query_scalar("SELECT replace(pending_operation_id::text,'-','') FROM game_staff_control_heads WHERE environment=$1").bind(trust.environment()).fetch_one(&mut *tx).await?;
    Ok(!orphaned && stored_pending == pending && seq == head_seq && previous == head_digest)
}
fn audit_payload(
    e: &SignedEnvelope,
    fingerprint: &str,
    roster: &str,
    outcome: &[u8],
) -> Result<Vec<u8>, DurabilityError> {
    // Length-delimited protobuf; exact signed bytes, no text is accepted by the action grammar.
    #[derive(prost::Message)]
    struct Event {
        #[prost(bytes = "vec", tag = "1")]
        canonical: Vec<u8>,
        #[prost(string, tag = "2")]
        principal: String,
        #[prost(string, tag = "3")]
        fingerprint: String,
        #[prost(bytes = "vec", tag = "4")]
        signature: Vec<u8>,
        #[prost(string, tag = "5")]
        roster: String,
        #[prost(bytes = "vec", tag = "6")]
        outcome: Vec<u8>,
    }
    use prost::Message;
    let bytes = Event {
        canonical: e.canonical.clone(),
        principal: e.principal.clone(),
        fingerprint: fingerprint.into(),
        signature: e.signature.clone(),
        roster: roster.into(),
        outcome: outcome.to_vec(),
    }
    .encode_to_vec();
    if bytes.len() > 16384 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(bytes)
}
async fn receipt_binds(
    tx: &mut PgConnection,
    trust: &StaffTrust,
    e: &SignedEnvelope,
    r: &rules::Receipt,
    fingerprint: &str,
    require_pending: bool,
) -> Result<bool, DurabilityError> {
    let row=sqlx::query("SELECT l.principal,l.key_fingerprint,l.canonical,o.outcome_sha256,o.result,o.kind,o.requires_receipt FROM game_staff_operations o JOIN game_staff_control_links l USING(environment,control_seq) WHERE o.environment=$1 AND o.operation_id=$2::uuid")
        .bind(trust.environment()).bind(&r.operation_id).fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let result: i16 = row.try_get("result")?;
    let expected = if result == 1 {
        rules::ReceiptDisposition::Committed
    } else {
        rules::ReceiptDisposition::Refused
    };
    Ok(row.try_get::<String, _>("principal")? == e.principal
        && row.try_get::<String, _>("key_fingerprint")? == fingerprint
        && r.disposition == expected
        && row.try_get::<String, _>("kind")? != "INSPECT"
        && (!require_pending || row.try_get::<bool, _>("requires_receipt")?)
        && row.try_get::<String, _>("outcome_sha256")? == r.outcome_sha256
        && digest(&row.try_get::<Vec<u8>, _>("canonical")?) == r.request_sha256)
}
pub(super) async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    trust: &Arc<StaffTrust>,
    auth: &Authenticated,
    fence: CurrentCharacterItemFence,
    clock: TrustedStaffClock,
) -> Result<Option<Vec<u8>>, DurabilityError> {
    let env = trust.environment();
    let head=sqlx::query("SELECT control_seq,link_sha256,activation_sha256,pending_operation_id::text FROM game_staff_control_heads WHERE environment=$1 FOR UPDATE")
        .bind(env).fetch_optional(&mut **tx).await?;
    let Some(head) = head else {
        return Ok(None);
    };
    let seq: i64 = head.try_get("control_seq")?;
    let sha: String = head.try_get("link_sha256")?;
    if !verify_chain(tx, trust, seq, &sha).await?
        || !retained::verify_and_retain(tx, trust, seq).await?
    {
        return Ok(None);
    }
    if let Some(result) = retained(tx, trust, auth).await? {
        return Ok(result);
    }
    if seq == i64::MAX
        || auth.seq() != Some(seq as u64 + 1)
        || auth.previous() != sha
        || auth.activation() != trust.activation()
        || auth.roster != trust.current_roster()
        || head.try_get::<String, _>("activation_sha256")? != trust.activation()
    {
        return Ok(None);
    }
    let pending: Option<String> = head.try_get("pending_operation_id")?;
    let (result, pending) = match &auth.message {
        Message::Receipt(r) => {
            if (pending.is_some()
                && pending.as_deref().map(|id| id.replace('-', "")) != Some(r.operation_id.clone()))
                || (pending.is_none() && r.disposition == rules::ReceiptDisposition::Committed)
                || !receipt_binds(
                    tx,
                    trust,
                    &auth.envelope,
                    r,
                    &auth.fingerprint,
                    pending.is_some(),
                )
                .await?
            {
                return Ok(None);
            }
            (
                result(
                    3,
                    seq as u64 + 1,
                    &auth.envelope.canonical,
                    0,
                    wire_result::StaffCode::Ok,
                )?,
                None,
            )
        }
        Message::Request(r) => {
            if pending.is_some()
                || r.world_id != hex_id(fence.runtime_scope.world_id().as_bytes())
                || r.channel_id
                    != hex_id(
                        channel(fence.runtime_scope)
                            .ok_or(DurabilityError::InvalidStoredState)?
                            .as_bytes(),
                    )
            {
                return Ok(None);
            }
            let Some(target) = crate::domain::CharacterId::from_bytes(parse_id(&r.action)).ok()
            else {
                return Ok(None);
            };
            if crate::domain::CharacterId::from_bytes(parse_hex_id(&r.operation_id)).is_err() {
                return Ok(None);
            }
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=$2::uuid AND lifecycle=1)")
                .bind(target.as_bytes().as_slice()).bind(&r.world_id).fetch_one(&mut **tx).await?;
            let (kind, duration) = match &r.action {
                rules::Action::Inspect { .. } => ("INSPECT", 0),
                rules::Action::Mute { duration_ms, .. } => (
                    "MUTE",
                    rules::decimal(duration_ms).ok_or(DurabilityError::InvalidStoredState)?,
                ),
                rules::Action::Unmute { .. } => ("UNMUTE", 0),
            };
            let mut committed = exists;
            if let rules::Action::Unmute { sanction_id, .. } = &r.action {
                committed=exists && sqlx::query_scalar::<_,bool>("SELECT true FROM game_character_sanctions WHERE environment=$1 AND sanction_id=$2::uuid AND character_id=encode($3,'hex')::uuid AND lifted_at_ms IS NULL FOR UPDATE")
                    .bind(env).bind(sanction_id).bind(target.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.unwrap_or(false);
            }
            let until: Option<i64> = if exists {
                sqlx::query_scalar("SELECT ends_at_ms FROM game_character_sanctions WHERE environment=$1 AND character_id=encode($2,'hex')::uuid AND lifted_at_ms IS NULL ORDER BY ends_at_ms DESC LIMIT 1")
                .bind(env).bind(target.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            } else {
                None
            };
            // Sample after all awaited prerequisites, never on an exact replay.
            let Some((lower, upper)) = clock() else {
                return Ok(None);
            };
            if lower == 0
                || lower > upper
                || upper > i64::MAX as u64 - 31_622_400_000
                || !r.fresh(lower, upper)
            {
                return Ok(None);
            }
            let outcome = result(
                if committed { 1 } else { 2 },
                seq as u64 + 1,
                &auth.envelope.canonical,
                if kind == "INSPECT" {
                    until.unwrap_or(0) as u64
                } else {
                    0
                },
                if committed {
                    wire_result::StaffCode::Ok
                } else if exists && kind == "UNMUTE" {
                    wire_result::StaffCode::AlreadyReleased
                } else {
                    wire_result::StaffCode::TargetNotFound
                },
            )?;
            let requires_receipt = committed && kind != "INSPECT";
            // Deferred FKs/guards allow the operation, link, effect and audit to be one unit.
            insert_link(tx, trust, auth, "REQUEST").await?;
            sqlx::query("INSERT INTO game_staff_operations(environment,operation_id,control_seq,kind,target_character_id,world_id,channel_id,starts_at_ms,duration_ms,result,outcome,outcome_sha256,requires_receipt) VALUES($1,$2::uuid,$3,$4,encode($5,'hex')::uuid,$6::uuid,$7::uuid,$8,$9,$10,$11,$12,$13)")
                .bind(env).bind(&r.operation_id).bind(seq+1).bind(kind).bind(target.as_bytes().as_slice()).bind(&r.world_id).bind(&r.channel_id).bind(lower as i64).bind(duration as i64).bind(if committed {1i16}else{2i16}).bind(&outcome).bind(digest(&outcome)).bind(requires_receipt).execute(&mut **tx).await?;
            if committed {
                match &r.action {
                    rules::Action::Mute { .. } => {
                        sqlx::query("INSERT INTO game_character_sanctions(environment,sanction_id,character_id,starts_at_ms,ends_at_ms) VALUES($1,$2::uuid,encode($3,'hex')::uuid,$4,$5)")
                            .bind(env).bind(&r.operation_id).bind(target.as_bytes().as_slice()).bind(lower as i64).bind((lower+duration) as i64).execute(&mut **tx).await?;
                    }
                    rules::Action::Unmute { sanction_id, .. } => {
                        sqlx::query("UPDATE game_character_sanctions SET lift_operation_id=$3::uuid,lifted_at_ms=$4 WHERE environment=$1 AND sanction_id=$2::uuid AND lifted_at_ms IS NULL")
                            .bind(env).bind(sanction_id).bind(&r.operation_id).bind(lower as i64).execute(&mut **tx).await?;
                    }
                    rules::Action::Inspect { .. } => {}
                }
            }
            let payload = audit_payload(&auth.envelope, &auth.fingerprint, &auth.roster, &outcome)?;
            sqlx::query("INSERT INTO game_staff_action_outbox(environment,operation_id,payload,starts_at_ms,expires_at_ms) VALUES($1,$2::uuid,$3,$4,$5)")
                .bind(env).bind(&r.operation_id).bind(payload).bind(lower as i64).bind((lower+31_536_000_000) as i64).execute(&mut **tx).await?;
            (outcome, requires_receipt.then(|| r.operation_id.clone()))
        }
    };
    if let Message::Receipt(r) = &auth.message {
        insert_link(tx, trust, auth, "RECEIPT").await?;
        sqlx::query("INSERT INTO game_staff_action_receipts(environment,operation_id,control_seq) VALUES($1,$2::uuid,$3)")
            .bind(env).bind(&r.operation_id).bind(seq+1).execute(&mut **tx).await?;
    }
    sqlx::query("UPDATE game_staff_control_heads SET control_seq=$2,link_sha256=$3,pending_operation_id=$4::uuid WHERE environment=$1")
        .bind(env).bind(seq+1).bind(digest(&auth.envelope.canonical)).bind(pending).execute(&mut **tx).await?;
    Ok(Some(result))
}
async fn insert_link(
    tx: &mut PgConnection,
    trust: &StaffTrust,
    auth: &Authenticated,
    kind: &str,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,kind,link_sha256,previous_link_sha256,activation_sha256,roster_sha256,principal,key_fingerprint,canonical,signature,operation_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::uuid)")
        .bind(trust.environment()).bind(auth.seq().ok_or(DurabilityError::InvalidStoredState)? as i64).bind(kind).bind(digest(&auth.envelope.canonical)).bind(auth.previous()).bind(auth.activation()).bind(&auth.roster).bind(&auth.envelope.principal).bind(&auth.fingerprint).bind(&auth.envelope.canonical).bind(&auth.envelope.signature).bind(auth.operation()).execute(tx).await?;
    Ok(())
}
fn hex_id(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn parse_hex_id(text: &str) -> [u8; 16] {
    let mut out = [0; 16];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = text
            .get(i * 2..i * 2 + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0);
    }
    out
}
fn parse_id(action: &rules::Action) -> [u8; 16] {
    parse_hex_id(match action {
        rules::Action::Inspect { character_id }
        | rules::Action::Mute { character_id, .. }
        | rules::Action::Unmute { character_id, .. } => character_id,
    })
}

fn channel(scope: crate::foundation::RuntimeScopeRefV1) -> Option<crate::foundation::ChannelId> {
    match scope {
        crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } => Some(channel_id),
        _ => None,
    }
}

async fn audit_expired(
    tx: &mut PgConnection,
    trust: &StaffTrust,
    operation: &str,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT starts_at_ms+31536000000+900000<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint AND (NOT requires_receipt OR EXISTS(SELECT 1 FROM game_staff_action_receipts r WHERE r.environment=o.environment AND r.operation_id=o.operation_id)) FROM game_staff_operations o WHERE environment=$1 AND operation_id=$2::uuid").bind(trust.environment()).bind(operation).fetch_optional(tx).await?.unwrap_or(false))
}

async fn operation_matches(
    tx: &mut PgConnection,
    trust: &StaffTrust,
    r: &rules::Request,
    row: &sqlx::postgres::PgRow,
    e: &SignedEnvelope,
    outcome: &[u8],
) -> Result<bool, DurabilityError> {
    let (kind, target, duration) = match &r.action {
        rules::Action::Inspect { character_id } => ("INSPECT", character_id, 0),
        rules::Action::Mute {
            character_id,
            duration_ms,
        } => (
            "MUTE",
            character_id,
            rules::decimal(duration_ms).unwrap_or(0),
        ),
        rules::Action::Unmute { character_id, .. } => ("UNMUTE", character_id, 0),
    };
    let result: wire_result::StaffResult = match wire_result::decode_result(outcome) {
        Ok(r) => r,
        Err(_) => return Ok(false),
    };
    let stored_result: i16 = row.try_get("result")?;
    let starts: i64 = row.try_get("starts_at_ms")?;
    if row.try_get::<String, _>("kind")? != kind
        || row.try_get::<i64, _>("duration_ms")? != duration as i64
        || result.disposition != stored_result as u64
        || result.control_seq != rules::decimal(&r.control_seq).unwrap_or(0)
        || result.link_sha256 != Sha256::digest(&e.canonical).as_slice()
        || row.try_get::<bool, _>("requires_receipt")? != (stored_result == 1 && kind != "INSPECT")
    {
        return Ok(false);
    }
    let identities:bool=sqlx::query_scalar("SELECT replace(target_character_id::text,'-','')=$3 AND replace(world_id::text,'-','')=$4 AND replace(channel_id::text,'-','')=$5 FROM game_staff_operations WHERE environment=$1 AND operation_id=$2::uuid")
        .bind(trust.environment()).bind(&r.operation_id).bind(target).bind(&r.world_id).bind(&r.channel_id).fetch_one(&mut *tx).await?;
    if !identities {
        return Ok(false);
    }
    let effect:Option<SanctionEffect>=sqlx::query_as("SELECT replace(character_id::text,'-',''),starts_at_ms,ends_at_ms,replace(sanction_id::text,'-',''),lifted_at_ms FROM game_character_sanctions WHERE environment=$1 AND (sanction_id=$2::uuid OR lift_operation_id=$2::uuid) LIMIT 1")
        .bind(trust.environment()).bind(&r.operation_id).fetch_optional(&mut *tx).await?;
    Ok(match (&r.action, stored_result, effect) {
        (rules::Action::Mute { .. }, 1, Some((character, start, end, _, _))) => {
            character == *target && start == starts && end - start == duration as i64
        }
        (
            rules::Action::Unmute { sanction_id, .. },
            1,
            Some((character, _, _, sanction, lifted)),
        ) => character == *target && sanction == *sanction_id && lifted == Some(starts),
        (rules::Action::Inspect { .. }, _, None) | (_, 2, None) => true,
        _ => false,
    })
}
use oteryn_protocol_oteryn::staff as wire_result;

type SanctionEffect = (String, i64, i64, String, Option<i64>);
