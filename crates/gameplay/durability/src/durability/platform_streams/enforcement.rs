//! One command, one fenced decision, with immutable operation binding and transactional outbox.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::super::{DurabilityError, DurabilityRoot};
use super::enforcement_wire::*;
use sqlx::Row;
type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

struct Stream {
    revision: i64,
    profile: Profile,
    account: String,
    character: String,
    state: StreamState,
    start: i64,
    end: i64,
}
impl Stream {
    fn view(&self, now: i64) -> StreamState {
        if matches!(self.state, StreamState::Active | StreamState::Pending) {
            if now >= self.end {
                StreamState::Expired
            } else if now < self.start {
                StreamState::Pending
            } else {
                StreamState::Active
            }
        } else {
            self.state
        }
    }
}
async fn stream(tx: &mut Tx<'_>, id: &str) -> Result<Option<Stream>, DurabilityError> {
    let row=sqlx::query("SELECT decision_revision,profile,account_id::text,character_id::text,state,effective_at_ms,expires_at_ms FROM game_platform_enforcement_streams WHERE sanction_stream_id=$1::uuid FOR UPDATE").bind(id).fetch_optional(&mut **tx).await?;
    row.map(|r| {
        Ok(Stream {
            revision: r.try_get("decision_revision")?,
            profile: match r.try_get::<&str, _>("profile")? {
                "MUTE" => Profile::Mute,
                "JAIL" => Profile::Jail,
                _ => return Err(DurabilityError::InvalidStoredState),
            },
            account: r.try_get("account_id")?,
            character: r.try_get("character_id")?,
            state: match r.try_get::<&str, _>("state")? {
                "PENDING" => StreamState::Pending,
                "ACTIVE" => StreamState::Active,
                "REVOKED" => StreamState::Revoked,
                "EXPIRED" => StreamState::Expired,
                _ => return Err(DurabilityError::InvalidStoredState),
            },
            start: r.try_get("effective_at_ms")?,
            end: r.try_get("expires_at_ms")?,
        })
    })
    .transpose()
}
fn result(
    id: &CommandIdentity,
    hash: &str,
    generation: i64,
    now: i64,
    current: Option<&Stream>,
    code: Code,
) -> EnforcementResult {
    let state = current.map_or(StreamState::None, |s| s.view(now));
    EnforcementResult {
        schema: "oteryn.enforcement_result.v1".into(),
        operation_id: id.operation_id.clone(),
        sanction_stream_id: id.sanction_stream_id.clone(),
        command_digest: hash.into(),
        receipt_revision: 1,
        state: State::Rejected,
        result_code: code,
        accepted_decision_revision: current.map_or(0, |s| s.revision),
        stream_state: state,
        effective_until_ms: current
            .filter(|_| matches!(state, StreamState::Pending | StreamState::Active))
            .map(|s| s.end),
        runtime_effect: RuntimeEffect::None,
        capability_revision: CAPABILITY.into(),
        recovery_generation: generation,
        decided_at_ms: now,
    }
}
async fn outbox(tx: &mut Tx<'_>, r: &EnforcementResult, raw: &[u8]) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_platform_enforcement_result_outbox(operation_id,command_digest,receipt_revision,result_bytes) VALUES($1::uuid,$2,$3,$4) ON CONFLICT DO NOTHING").bind(&r.operation_id).bind(&r.command_digest).bind(i32::from(r.receipt_revision)).bind(raw).execute(&mut **tx).await?;
    Ok(())
}
async fn persist(
    tx: &mut Tx<'_>,
    id: &CommandIdentity,
    command: &[u8],
    r: &EnforcementResult,
) -> Result<Vec<u8>, DurabilityError> {
    let raw = r.encode().ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_platform_enforcement_receipts(operation_id,sanction_stream_id,command_bytes,command_digest,receipt_revision,result_bytes) VALUES($1::uuid,$2::uuid,$3,$4,$5,$6) ON CONFLICT(operation_id) DO UPDATE SET receipt_revision=EXCLUDED.receipt_revision,result_bytes=EXCLUDED.result_bytes WHERE game_platform_enforcement_receipts.command_bytes=EXCLUDED.command_bytes AND game_platform_enforcement_receipts.command_digest=EXCLUDED.command_digest")
  .bind(&id.operation_id).bind(&id.sanction_stream_id).bind(command).bind(&r.command_digest).bind(i32::from(r.receipt_revision)).bind(&raw).execute(&mut **tx).await?;
    outbox(tx, r, &raw).await?;
    Ok(raw)
}
impl DurabilityRoot {
    pub async fn decide_platform_enforcement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        raw: &[u8],
    ) -> Result<Vec<u8>, DurabilityError> {
        // A malformed command can be refused only when its two unique UUIDv7
        // identities are safely attributable; never manufacture wire identities.
        let id = CommandIdentity::parse(raw).ok_or(DurabilityError::InvalidStoredState)?;
        let parsed = Command::parse_for_decision(raw);
        let raw = raw.to_vec();
        let hash = digest(&raw);
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
   let mut tx=begin_semantic_transaction(holder,deadline).await?;
   assert_recovery_fence(&mut tx,&recovery).await?;
   if !prove_current_incarnation(&mut tx,&node).await? {return Err(DurabilityError::Unavailable);}
   for (namespace,key) in [("operation",&id.operation_id),("stream",&id.sanction_stream_id)] {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:platform-enforcement:'||$1||':'||$2,0))").bind(namespace).bind(key).execute(&mut *tx).await?;
   }
   let previous=sqlx::query("SELECT command_bytes,command_digest,result_bytes FROM game_platform_enforcement_receipts WHERE operation_id=$1::uuid FOR UPDATE").bind(&id.operation_id).fetch_optional(&mut *tx).await?;
   let mut revision=1;
   if let Some(row)=&previous {
    let previous_raw:Vec<u8>=row.try_get("result_bytes")?;
    let previous_result=EnforcementResult::parse(&previous_raw).ok_or(DurabilityError::InvalidStoredState)?;
    if row.try_get::<String,_>("command_digest")?==hash && row.try_get::<Vec<u8>,_>("command_bytes")?==raw {
     if previous_result.state!=State::AcceptedPending {
      outbox(&mut tx,&previous_result,&previous_raw).await?;commit_semantic_transaction(tx,deadline).await?;return Ok(previous_raw);
     }
     revision=previous_result.receipt_revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
    } else {
     // §4.3 cannot accept a conflicting digest or a second terminal body under
     // this operation. Preserve a local OPERATION_CONFLICT and republish only
     // the immutable result of the command originally served by Platform.
     sqlx::query("INSERT INTO game_platform_enforcement_conflicts(operation_id,command_digest,command_bytes,result_code) VALUES($1::uuid,$2,$3,'OPERATION_CONFLICT') ON CONFLICT DO NOTHING")
      .bind(&id.operation_id).bind(&hash).bind(&raw).execute(&mut *tx).await?;
     outbox(&mut tx,&previous_result,&previous_raw).await?;
     commit_semantic_transaction(tx,deadline).await?;return Ok(previous_raw);
    }
   }
   let current=stream(&mut tx,&id.sanction_stream_id).await?;
   let mut now:i64=sqlx::query_scalar("SELECT game_platform_enforcement_now_ms()").fetch_one(&mut *tx).await?;
   if now<=0 {return Err(DurabilityError::Unavailable);}
   let generation=i64::try_from(recovery.recovery_generation).map_err(|_|DurabilityError::InvalidStoredState)?;
   let (enabled,policy):(bool,Option<i64>)=sqlx::query_as("SELECT enabled,supported_policy_revision FROM game_platform_enforcement_configuration WHERE singleton FOR SHARE").fetch_one(&mut *tx).await?;
   if !enabled || policy.is_none() {return Err(DurabilityError::Unavailable);}
   let Some(c)=parsed else {
    // These exact immutable bytes can never execute. A terminal refusal leaves
    // the attempt reconciliable rather than pending forever on a parser error.
    let r=result(&id,&hash,generation,now,current.as_ref(),Code::InternalNonRetryable);
    let bytes=persist(&mut tx,&id,&raw,&r).await?;
    commit_semantic_transaction(tx,deadline).await?;return Ok(bytes);
   };
   let root=sqlx::query("SELECT account_id::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid")
    .bind(&c.character_id).fetch_optional(&mut *tx).await?;
   let stale=match c.intent {
    Intent::Apply=>current.is_some(),
    Intent::Replace|Intent::Revoke=>current.as_ref().is_none_or(|s|s.revision>=c.decision_revision || Some(s.revision)!=c.prior_decision_revision),
    Intent::Expire=>current.as_ref().is_none_or(|s|s.revision!=c.decision_revision),
   };
   let mut rejection=if c.command_version!=1 || c.profile==Profile::Jail {Some(Code::Unsupported)}
   else if policy.is_some_and(|supported|c.policy_revision>supported) || current.as_ref().is_some_and(|s|s.profile!=c.profile) {Some(Code::CapabilityConflict)}
   else if stale {Some(Code::StaleRevision)}
   else if root.is_none() {Some(Code::TargetNotApplicable)}
   else if root.as_ref().map(|r|r.try_get::<String,_>("account_id")).transpose()?.as_deref()!=Some(c.account_id.as_str()) {Some(Code::TargetMismatch)}
   else if root.as_ref().map(|r|r.try_get::<i16,_>("lifecycle")).transpose()?!=Some(1) {Some(Code::LifecycleConflict)}
   else {None};
   if rejection.is_none() && current.as_ref().is_some_and(|s|s.account!=c.account_id || s.character!=c.character_id) {
    rejection=Some(Code::TargetMismatch);
   }
   if rejection.is_none() {
    // Validate the subject before creating portfolio state. Then take the
    // canonical portfolio-before-CharacterLease fences and revalidate under lock.
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING").bind(&c.account_id).execute(&mut *tx).await?;
    sqlx::query("SELECT 1 FROM game_character_account_guards WHERE account_id=$1::uuid FOR UPDATE").bind(&c.account_id).fetch_one(&mut *tx).await?;
    lock_admission_relations(&mut tx).await?;
    let root=sqlx::query("SELECT account_id::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(&c.character_id).fetch_optional(&mut *tx).await?;
    now=sqlx::query_scalar("SELECT game_platform_enforcement_now_ms()").fetch_one(&mut *tx).await?;
    if now<=0 {return Err(DurabilityError::Unavailable);}
    rejection=if root.is_none() {Some(Code::TargetNotApplicable)}
     else if root.as_ref().map(|r|r.try_get::<String,_>("account_id")).transpose()?.as_deref()!=Some(c.account_id.as_str()) {Some(Code::TargetMismatch)}
     else if root.as_ref().map(|r|r.try_get::<i16,_>("lifecycle")).transpose()?!=Some(1) {Some(Code::LifecycleConflict)} else {None};
   }
   let mut r=result(&id,&hash,generation,now,current.as_ref(),rejection.unwrap_or(Code::Ok));
   r.receipt_revision=revision;
   if rejection.is_none() {
    let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND account_id=$2::uuid AND session_state IN(1,2))").bind(&c.character_id).bind(&c.account_id).fetch_one(&mut *tx).await?;
    r.runtime_effect=if live {RuntimeEffect::OnlineApplied} else {RuntimeEffect::OnNextAdmission};
    let state=match c.intent {
     Intent::Apply|Intent::Replace=>{
      let(start,end)=(c.effective_at_ms.ok_or(DurabilityError::InvalidStoredState)?,c.expires_at_ms.ok_or(DurabilityError::InvalidStoredState)?);
      let s=if now<start {StreamState::Pending} else if now>=end {StreamState::Expired} else {StreamState::Active};
      sqlx::query("INSERT INTO game_platform_enforcement_streams(sanction_stream_id,decision_revision,profile,account_id,character_id,state,effective_at_ms,expires_at_ms,reason_code,policy_revision) VALUES($1::uuid,$2,$3,$4::uuid,$5::uuid,$6,$7,$8,$9,$10) ON CONFLICT(sanction_stream_id) DO UPDATE SET decision_revision=EXCLUDED.decision_revision,state=EXCLUDED.state,effective_at_ms=EXCLUDED.effective_at_ms,expires_at_ms=EXCLUDED.expires_at_ms,reason_code=EXCLUDED.reason_code,policy_revision=EXCLUDED.policy_revision")
       .bind(&c.sanction_stream_id).bind(c.decision_revision).bind(c.profile.wire()).bind(&c.account_id).bind(&c.character_id).bind(s.wire()).bind(start).bind(end).bind(i32::from(c.reason_code.ok_or(DurabilityError::InvalidStoredState)?)).bind(c.policy_revision).execute(&mut *tx).await?;
      r.state=if c.intent==Intent::Apply {State::Applied} else {State::Replaced};s
     },
     Intent::Revoke=>{
      sqlx::query("UPDATE game_platform_enforcement_streams SET decision_revision=$2,state='REVOKED' WHERE sanction_stream_id=$1::uuid").bind(&c.sanction_stream_id).bind(c.decision_revision).execute(&mut *tx).await?;r.state=State::Revoked;StreamState::Revoked
     },
     Intent::Expire=>{
      let s=current.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
      if matches!(s.state,StreamState::Revoked|StreamState::Expired) || now>=s.end {
       // An expiry reconciliation must never revive or erase a newer revocation.
       if s.state!=StreamState::Revoked {sqlx::query("UPDATE game_platform_enforcement_streams SET state='EXPIRED' WHERE sanction_stream_id=$1::uuid").bind(&c.sanction_stream_id).execute(&mut *tx).await?;}
       r.state=State::Expired;if s.state==StreamState::Revoked {StreamState::Revoked} else {StreamState::Expired}
      } else {r.state=State::AcceptedPending;r.result_code=Code::PendingExpiry;r.runtime_effect=RuntimeEffect::None;s.view(now)}
     },
    };
    r.accepted_decision_revision=c.decision_revision;r.stream_state=state;
    r.effective_until_ms=if matches!(state,StreamState::Active|StreamState::Pending) {c.expires_at_ms.or(current.as_ref().map(|s|s.end))} else {None};
   }
   if let Some(previous)=previous {
    let bytes:Vec<u8>=previous.try_get("result_bytes")?;let old=EnforcementResult::parse(&bytes).ok_or(DurabilityError::InvalidStoredState)?;
    if old.state==r.state && old.result_code==r.result_code && old.stream_state==r.stream_state && old.accepted_decision_revision==r.accepted_decision_revision {
     outbox(&mut tx,&old,&bytes).await?;commit_semantic_transaction(tx,deadline).await?;return Ok(bytes);
    }
   }
   let bytes=persist(&mut tx,&id,&raw,&r).await?;commit_semantic_transaction(tx,deadline).await?;Ok(bytes)
  })).await
    }
    pub async fn platform_mute_for_session(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        session: crate::foundation::GameSessionId,
    ) -> Result<Option<u32>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let now:i64=sqlx::query_scalar("SELECT game_platform_enforcement_now_ms()").fetch_one(&mut *tx).await?;
   if now<=0 {return Err(DurabilityError::Unavailable);}
   let remaining:Option<i64>=sqlx::query_scalar("SELECT e.expires_at_ms-$2 FROM game_platform_enforcement_streams e JOIN game_durability_reconnect_sessions s ON s.character_id=e.character_id WHERE s.game_session_id=encode($1,'hex')::uuid AND s.session_state IN(1,2) AND e.profile='MUTE' AND e.state IN('PENDING','ACTIVE') AND e.effective_at_ms<=$2 AND $2<e.expires_at_ms ORDER BY e.expires_at_ms DESC LIMIT 1").bind(session.as_bytes().as_slice()).bind(now).fetch_optional(&mut *tx).await?;
   commit_semantic_transaction(tx,deadline).await?;
   Ok(remaining.map(|ms|u32::try_from(ms/1000+i64::from(ms%1000!=0)).unwrap_or(u32::MAX)))
  })).await
    }
}
