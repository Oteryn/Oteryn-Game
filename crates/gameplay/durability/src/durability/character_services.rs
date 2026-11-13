//! G-3 v2 decisions: portfolio first, immutable operation binding and atomic projections.
use super::character_authority::{
    ReconciledCharacterAuthority, Security, account_security, assert_recovery_fence, uuid_text,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::character_name::CharacterName;
use crate::native_admission_source::character_commands::{
    Decision, Input, source_authority_valid, uuid_v7,
};
use crate::native_admission_source::character_services_v2::{
    CharacterCommandIntentV2, PublishCharacterCommandReceiptV2,
};
use serde_json::{Value, json};
use sqlx::Row;
type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
/// Game policy, independent of the intent's technical TTL.
pub const DELETION_GRACE_SECONDS: i64 = 60 * 24 * 60 * 60;
/// D183: even the previous owner cannot reclaim a released name during this interval.
pub const RELEASED_NAME_HOLD_SECONDS: i64 = 30 * 24 * 60 * 60;

impl DurabilityRoot {
    /// A dependency failure commits neither mutation nor receipt. Replay returns the original bytes.
    pub async fn decide_character_service(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        source: &str,
        entry: (&str, &str),
        input: Input<'_>,
    ) -> Decision {
        let (command, operation) = entry;
        if !source_authority_valid(source) || !uuid_v7(operation) {
            return Decision::Transient;
        }
        let Ok(recovery) = authority.record_for(self) else {
            return Decision::Transient;
        };
        let raw = match input {
            Input::Intent(raw) if raw.len() <= 4096 => raw.to_vec(),
            _ => return Decision::Transient,
        };
        let source = source.to_owned();
        let command = command.to_owned();
        let operation = operation.to_owned();
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
                    let result = decide(
                        &mut tx,
                        &source,
                        &command,
                        &operation,
                        &raw,
                        recovery.recovery_generation,
                    )
                    .await?;
                    if matches!(result, Decision::Publish { .. }) {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await;
        match result {
            Ok(decision) => decision,
            _ => Decision::Transient,
        }
    }

    /// Immutable outcome lookup before any fresh eligibility dependency. No new decision.
    pub async fn replay_character_service(
        &self,authority:&ReconciledCharacterAuthority<'_, '_>,proof:&NodeIncarnationProof,
        command:&str,operation:&str,input:Input<'_>,
    )->Result<Option<Decision>,DurabilityError> {
        if !uuid_v7(operation) {return Err(DurabilityError::Unavailable);}
        let raw=match input {
            Input::Intent(raw) if raw.len()<=4096=>raw.to_vec(),
            Input::Intent(_) | Input::Oversized=>super::character_command::oversized_binding(command),
            Input::ReadConflict=>return Ok(Some(Decision::Conflict)),
            Input::Unsupported=>return Ok(None),
        };
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::Unavailable)?;
        let proof=proof.clone();let command=command.to_owned();let operation=operation.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await? {return Err(DurabilityError::Unavailable);}
            replay_receipt(&mut tx,&command,&operation,&raw,recovery.recovery_generation).await
        })).await
    }

    /// A refused publication stops this exact immutable receipt across restarts.
    pub async fn stop_character_service_receipt(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        operation: &str,
        receipt: &[u8],
    ) -> Result<(), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let operation = operation.to_owned();
        let receipt = receipt.to_vec();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await? {return Err(DurabilityError::Unavailable);}
            sqlx::query("INSERT INTO game_character_service_receipt_stops(operation_id,receipt_sha256,recovery_generation,stopped_at) SELECT operation_id,sha256(receipt),$3::numeric,game_character_service_now() FROM game_character_service_receipts WHERE operation_id=$1::uuid AND receipt=$2 ON CONFLICT DO NOTHING")
                .bind(operation).bind(receipt).bind(recovery.recovery_generation.to_string()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await
        })).await
    }

    /// One bounded finalization pass. It competes with cancel under the same portfolio/root locks.
    pub async fn finalize_character_service_deletion(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let proof = proof.clone();
        let pass = self.try_issue_semantic_pass()?;
        pass.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            if !prove_current_incarnation(&mut tx, &proof).await? { return Err(DurabilityError::Unavailable); }
            let candidate = sqlx::query("SELECT character_id::text,account_id::text FROM game_character_service_deletions WHERE finalized_at IS NULL AND effective_at<=game_character_service_now() ORDER BY effective_at,character_id LIMIT 1")
                .fetch_optional(&mut *tx).await?;
            let Some(candidate) = candidate else { return Ok(false); };
            let character: String = candidate.try_get("character_id")?; let account: String = candidate.try_get("account_id")?;
            portfolio(&mut tx, &account).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            let root = sqlx::query("SELECT name,lifecycle,character_revision::text FROM game_character_roots WHERE character_id=$1::uuid AND account_id=$2::uuid FOR UPDATE")
                .bind(&character).bind(&account).fetch_optional(&mut *tx).await?.ok_or(DurabilityError::Unavailable)?;
            let now = trusted_now(&mut tx, Some(&account)).await?;
            let deadline_row = sqlx::query("SELECT effective_at FROM game_character_service_deletions WHERE character_id=$1::uuid AND finalized_at IS NULL FOR UPDATE")
                .bind(&character).fetch_optional(&mut *tx).await?;
            if root.try_get::<i16,_>("lifecycle")? != 3 || deadline_row.is_none_or(|d| d.get::<i64,_>("effective_at") > now) { return Ok(false); }
            if in_session(&mut tx, &character, &account).await? { return Err(DurabilityError::Unavailable); }
            sqlx::query("UPDATE game_character_service_deletions SET finalized_at=$2 WHERE character_id=$1::uuid")
                .bind(&character).bind(now).execute(&mut *tx).await?;
            change(&mut tx, Change { character: &character, operation: None, purpose: "FINALIZE", revision: &root.try_get::<String,_>("character_revision")?, old_name: &root.try_get::<String,_>("name")?, new_name: &root.try_get::<String,_>("name")?, old_lifecycle: 3, new_lifecycle: 4, now }).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(true)
        })).await
    }
}

async fn trusted_now(tx: &mut Tx<'_>, account: Option<&str>) -> Result<i64, DurabilityError> {
    let now: Option<i64> = sqlx::query_scalar("SELECT game_character_service_now()")
        .fetch_one(&mut **tx)
        .await?;
    let now = now.filter(|n| *n > 0).ok_or(DurabilityError::Unavailable)?;
    if let Some(account) = account {
        let future: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_service_revisions WHERE account_id=$1::uuid AND revised_at>$2)")
            .bind(account).bind(now).fetch_one(&mut **tx).await?;
        if future {
            return Err(DurabilityError::Unavailable);
        }
    }
    Ok(now)
}
async fn portfolio(tx: &mut Tx<'_>, account: &str) -> Result<(), DurabilityError> {
    // The empty portfolio mutex carries no ownership authority. Match creation's
    // lock protocol for a canonical account that has not owned a character yet.
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING").bind(account).execute(&mut **tx).await?;
    sqlx::query(
        "SELECT account_id FROM game_character_account_guards WHERE account_id=$1::uuid FOR UPDATE",
    )
    .bind(account)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DurabilityError::Unavailable)?;
    Ok(())
}
async fn in_session(
    tx: &mut Tx<'_>,
    character: &str,
    account: &str,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND session_state IN(1,2)) OR EXISTS(SELECT 1 FROM game_durability_admission_character_guards WHERE character_id=$1::uuid AND holder_game_session_id IS NOT NULL) OR EXISTS(SELECT 1 FROM game_durability_admission_account_guards WHERE account_id=$2::uuid AND presence_character_id=$1::uuid)")
        .bind(character).bind(account).fetch_one(&mut **tx).await?)
}

fn text<'a>(intent: &'a CharacterCommandIntentV2, key: &str) -> Result<&'a str, DurabilityError> {
    intent
        .field(key)
        .and_then(Value::as_str)
        .ok_or(DurabilityError::Unavailable)
}
async fn replay_receipt(tx:&mut Tx<'_>,command:&str,operation:&str,raw:&[u8],recovery:u64)->Result<Option<Decision>,DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-operation:'||replace($1,'-',''),0))")
        .bind(operation).execute(&mut **tx).await?;
    if let Some(stored) = sqlx::query("SELECT command,intent,receipt,result_code FROM game_character_service_receipts WHERE operation_id=$1::uuid")
        .bind(operation).fetch_optional(&mut **tx).await? {
        let stopped:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_service_receipt_stops WHERE operation_id=$1::uuid AND recovery_generation>=$2::numeric)").bind(operation).bind(recovery.to_string()).fetch_one(&mut **tx).await?;
        if stopped {return Ok(Some(Decision::Conflict));}
        return Ok(Some(if stored.try_get::<String,_>("command")? == command && stored.try_get::<Vec<u8>,_>("intent")? == raw {
            Decision::Publish { receipt: stored.try_get("receipt")?, result_code: stored.try_get("result_code")? }
        } else { Decision::Conflict }));
    }
    Ok(None)
}

async fn decide(
    tx: &mut Tx<'_>,
    source: &str,
    command: &str,
    operation: &str,
    raw: &[u8],
    recovery: u64,
) -> Result<Decision, DurabilityError> {
    if let Some(stored)=replay_receipt(tx,command,operation,raw,recovery).await? {return Ok(stored);}
    let occupied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_command_receipts WHERE operation_id=$1::uuid UNION ALL SELECT 1 FROM game_character_operation_receipts WHERE operation_id=$1::uuid UNION ALL SELECT 1 FROM game_bazaar_decisions WHERE issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND operation_id=$1::uuid)")
        .bind(operation).fetch_one(&mut **tx).await?;
    if occupied
        || super::character_command::account_deletion::operation_exists(tx, &uuid_text(operation)?)
            .await?
    {
        return Ok(Decision::Conflict);
    }
    // A readable contradictory key is never a terminal no-commit refund, even
    // when another malformed member prevents a full canonical decode.
    if let Ok(head) = serde_json::from_slice::<Value>(raw)
        && (head
            .get("command")
            .and_then(Value::as_str)
            .is_some_and(|value| value != command)
            || head
                .get("operation_id")
                .and_then(Value::as_str)
                .is_some_and(|value| value != operation))
    {
        return Ok(Decision::Conflict);
    }
    let intent = CharacterCommandIntentV2::decode(raw).ok();
    let mut code = "CHAR_CMD_INTENT_INVALID";
    let mut facts = Vec::new();
    let mut variant = None;
    let mut source_revision = None;
    let mut now = trusted_now(tx, None).await?;
    if let Some(intent) = intent.as_ref() {
        if text(intent, "operation_id")? != operation || text(intent, "command")? != command {
            return Ok(Decision::Conflict);
        }
        variant = Some(text(intent, "variant")?);
        source_revision = Some(text(intent, "source_revision")?);
        let account = text(intent, "account_id")?;
        let character = text(intent, "character_id")?;
        portfolio(tx, account).await?;
        super::db::lock_admission_relations(tx).await?;
        let root = sqlx::query("SELECT account_id::text,name,name_key,lifecycle,character_revision::text FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE")
            .bind(character).fetch_optional(&mut **tx).await?;
        now = trusted_now(tx, Some(account)).await?;
        let issued = text(intent, "issued_at_source")?
            .parse::<u64>()
            .map_err(|_| DurabilityError::Unavailable)?;
        let expires = text(intent, "expires_at_source")?
            .parse::<u64>()
            .map_err(|_| DurabilityError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-service-revision:'||$1||':'||$2,0))")
            .bind(variant).bind(source_revision).execute(&mut **tx).await?;
        let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_service_receipts WHERE variant=$1 AND source_revision=$2::numeric AND result_code<>'CHAR_CMD_OPERATION_CONFLICT')")
            .bind(variant).bind(source_revision).fetch_one(&mut **tx).await?;
        code = if duplicate {
            "CHAR_CMD_OPERATION_CONFLICT"
        } else if issued > (now as u64).saturating_add(5) {
            "CHAR_CMD_INTENT_INVALID"
        } else if expires <= now as u64 {
            "CHAR_CMD_INTENT_EXPIRED"
        } else if sqlx::query_scalar::<_, bool>("SELECT game_account_closure_closed($1::uuid)")
            .bind(account)
            .fetch_one(&mut **tx)
            .await?
        {
            "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE"
        } else {
            match account_security(tx, &uuid_text(account)?, now).await? {
                Security::Unknown => return Err(DurabilityError::Unavailable),
                Security::Refused => "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE",
                Security::Allowed => {
                    if let Some(root) = root.as_ref() {
                        if root.try_get::<String, _>("account_id")? != account {
                            "CHAR_CMD_OWNER_MISMATCH"
                        } else {
                            execute(tx, intent, root, now, &mut facts).await?
                        }
                    } else {
                        "CHAR_CMD_CHARACTER_NOT_FOUND"
                    }
                }
            }
        };
    }
    let mut fields = vec![
        ("contract_version".into(), json!(2)),
        (
            "operation".into(),
            json!("PublishCharacterCommandReceiptV2"),
        ),
        ("source_authority".into(), json!(source)),
        ("command".into(), json!(command)),
        ("operation_id".into(), json!(operation)),
        (
            "outcome".into(),
            json!(if code == "CHAR_CMD_OK" {
                "COMMITTED"
            } else {
                "REJECTED"
            }),
        ),
        ("result_code".into(), json!(code)),
    ];
    fields.extend(facts);
    fields.extend([
        ("recovery_generation".into(), json!(recovery.to_string())),
        ("decided_at".into(), json!(now.to_string())),
    ]);
    let receipt = PublishCharacterCommandReceiptV2::from_fields(fields)
        .and_then(|v| v.encode())
        .map_err(|_| DurabilityError::Unavailable)?;
    sqlx::query("INSERT INTO game_character_service_receipts(operation_id,command,variant,source_revision,intent,receipt,result_code,recovery_generation,decided_at) VALUES($1::uuid,$2,$3,$4::numeric,$5,$6,$7,$8::numeric,$9)")
        .bind(operation).bind(command).bind(variant).bind(source_revision).bind(raw).bind(&receipt).bind(code).bind(recovery.to_string()).bind(now).execute(&mut **tx).await?;
    Ok(Decision::Publish {
        receipt,
        result_code: code.into(),
    })
}

async fn execute(
    tx: &mut Tx<'_>,
    intent: &CharacterCommandIntentV2,
    root: &sqlx::postgres::PgRow,
    now: i64,
    facts: &mut Vec<(String, Value)>,
) -> Result<&'static str, DurabilityError> {
    let command = text(intent, "command")?;
    let character = text(intent, "character_id")?;
    let account = text(intent, "account_id")?;
    let operation = text(intent, "operation_id")?;
    let revision: String = root.try_get("character_revision")?;
    let lifecycle: i16 = root.try_get("lifecycle")?;
    let name: String = root.try_get("name")?;
    if command == "CancelCharacterDeletion" && lifecycle == 4 {
        return Ok("CHAR_CMD_DELETION_WINDOW_CLOSED");
    }
    if revision != text(intent, "expected_character_revision")? {
        return Ok("CHAR_CMD_STALE_PRECONDITION");
    }
    if (command == "CancelCharacterDeletion" && lifecycle != 3)
        || (command != "CancelCharacterDeletion" && lifecycle != 1)
    {
        return Ok("CHAR_CMD_LIFECYCLE_CONFLICT");
    }
    if in_session(tx, character, account).await? {
        return Ok("CHAR_CMD_CHARACTER_IN_SESSION");
    }
    if matches!(command, "ChangeCharacterSex" | "TransferCharacterWorld") {
        return Ok("CHAR_CMD_UNSUPPORTED");
    }
    let mut new_name = name.clone();
    let mut extra = Vec::new();
    let (purpose, new_lifecycle) = match command {
        "RenameCharacter" => {
            let Ok(parsed) = CharacterName::parse(text(intent, "requested_name")?) else {
                return Ok("CHAR_CMD_NAME_INVALID");
            };
            let key = parsed.comparison_key();
            if key == root.try_get::<String, _>("name_key")? {
                return Ok("CHAR_CMD_NAME_UNAVAILABLE");
            }
            sqlx::query(
                "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:character-name:'||$1,0))",
            )
            .bind(&key)
            .execute(&mut **tx)
            .await?;
            sqlx::query("SELECT game_character_service_release_name($1)")
                .bind(&key)
                .execute(&mut **tx)
                .await?;
            let taken: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_name_reservations WHERE name_key=$1)",
            )
            .bind(&key)
            .fetch_one(&mut **tx)
            .await?;
            if taken {
                return Ok("CHAR_CMD_NAME_UNAVAILABLE");
            }
            new_name = text(intent, "requested_name")?.to_owned();
            extra.extend([
                ("previous_name".into(), json!(name)),
                ("name".into(), json!(new_name)),
            ]);
            ("RENAME", 1)
        }
        "ScheduleCharacterDeletion" => {
            let effective = now
                .checked_add(DELETION_GRACE_SECONDS)
                .ok_or(DurabilityError::Unavailable)?;
            sqlx::query("INSERT INTO game_character_service_deletions(character_id,account_id,operation_id,scheduled_at,effective_at) VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5)")
                .bind(character).bind(account).bind(operation).bind(now).bind(effective).execute(&mut **tx).await?;
            extra.extend([
                ("lifecycle".into(), json!("DELETION_PENDING")),
                ("deletion_operation_id".into(), json!(operation)),
                ("deletion_scheduled_at".into(), json!(now.to_string())),
                ("deletion_effective_at".into(), json!(effective.to_string())),
            ]);
            ("SCHEDULE", 3)
        }
        "CancelCharacterDeletion" => {
            let deletion = sqlx::query("SELECT operation_id::text,effective_at FROM game_character_service_deletions WHERE character_id=$1::uuid AND finalized_at IS NULL FOR UPDATE")
                .bind(character).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::Unavailable)?;
            if deletion.try_get::<String, _>("operation_id")?
                != text(intent, "deletion_operation_id")?
            {
                return Ok("CHAR_CMD_STALE_PRECONDITION");
            }
            if now >= deletion.try_get::<i64, _>("effective_at")? {
                return Ok("CHAR_CMD_DELETION_WINDOW_CLOSED");
            }
            sqlx::query("DELETE FROM game_character_service_deletions WHERE character_id=$1::uuid")
                .bind(character)
                .execute(&mut **tx)
                .await?;
            extra.extend([
                ("lifecycle".into(), json!("ACTIVE")),
                (
                    "deletion_operation_id".into(),
                    json!(text(intent, "deletion_operation_id")?),
                ),
            ]);
            ("CANCEL", 1)
        }
        _ => return Ok("CHAR_CMD_UNSUPPORTED"),
    };
    change(
        tx,
        Change {
            character,
            operation: Some(operation),
            purpose,
            revision: &revision,
            old_name: &name,
            new_name: &new_name,
            old_lifecycle: lifecycle,
            new_lifecycle,
            now,
        },
    )
    .await?;
    let projection = sqlx::query("SELECT revision::text,(SELECT projection_epoch::text FROM game_character_account_projection_epoch WHERE epoch_scope=1) AS epoch FROM game_character_service_revisions WHERE account_id=$1::uuid")
        .bind(account).fetch_one(&mut **tx).await?;
    let new_revision = revision
        .parse::<u64>()
        .ok()
        .and_then(|v| v.checked_add(1))
        .ok_or(DurabilityError::Unavailable)?;
    facts.extend([
        ("character_id".into(), json!(character)),
        ("account_id".into(), json!(account)),
        ("character_revision".into(), json!(new_revision.to_string())),
        (
            "projection_epoch".into(),
            json!(projection.try_get::<String, _>("epoch")?),
        ),
        (
            "projection_revision".into(),
            json!(projection.try_get::<String, _>("revision")?),
        ),
    ]);
    facts.extend(extra);
    Ok("CHAR_CMD_OK")
}
struct Change<'a> {
    character: &'a str,
    operation: Option<&'a str>,
    purpose: &'a str,
    revision: &'a str,
    old_name: &'a str,
    new_name: &'a str,
    old_lifecycle: i16,
    new_lifecycle: i16,
    now: i64,
}
async fn change(tx: &mut Tx<'_>, c: Change<'_>) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_character_service_changes(character_id,operation_id,purpose,old_revision,new_revision,old_name,new_name,old_lifecycle,new_lifecycle,occurred_at) VALUES($1::uuid,$2::uuid,$3,$4::numeric,$4::numeric+1,$5,$6,$7,$8,$9)")
        .bind(c.character).bind(c.operation).bind(c.purpose).bind(c.revision).bind(c.old_name).bind(c.new_name).bind(c.old_lifecycle).bind(c.new_lifecycle).bind(c.now).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_character_roots SET name=$2,lifecycle=$3,character_revision=character_revision+1 WHERE character_id=$1::uuid AND character_revision=$4::numeric")
        .bind(c.character).bind(c.new_name).bind(c.new_lifecycle).bind(c.revision).execute(&mut **tx).await?;
    Ok(())
}
