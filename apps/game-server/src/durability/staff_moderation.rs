//! GM-1 / CP D1437: the staff action executor and the minimal moderation
//! writers (WRITE-MOD-1 is not on main). Every action is one semantic
//! transaction: the staff participant (`staff_action`) locks the control head,
//! the effect rows are written, then the same participant inserts the audit
//! link, operation and outbox row. Any failure before commit, the audit
//! included, rolls the effect back. A live effect (a kick, a mute pushed to
//! chat) and any disclosed data happen only after that commit.
//!
//! Sanctions are staff records (migration 0134), never Character aggregate
//! writes; no action creates value. Item-text reports follow WRITE0 §6.
use super::DurabilityRoot;
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::staff_action::{
    Begin, Pending, StaffError, StoredOutcome, begin_in_transaction, finish_in_transaction,
};
use crate::staff::audit::{Contract, ResultCode};
use crate::staff::live::{KickResult, StaffDesk};
use crate::staff::signature::{
    self, Action, ControlMessage, Decimal, Disposition, Error, History, Outcome, ReportState,
    SignedBytes, Target, VerifiedRequest, VerifierTrust,
};
use serde_json::{Value, json};
use sqlx::{PgConnection, Row};
use std::sync::Arc;

/// Longest staff mute: 365 days, the sanction table's own bound.
pub const MAX_MUTE_MS: u64 = 31_536_000_000;
/// WRITE0-RL-03: a report snapshot is kept 180 days after the report.
pub const REPORT_RETENTION_MS: i64 = 15_552_000_000;
/// WRITE0-RL-04: reports per reporting character per hour.
pub const REPORTS_PER_HOUR: i16 = 10;
/// WRITE0-RL-05: open reports per reporting account.
pub const OPEN_REPORTS_PER_ACCOUNT: i16 = 20;
/// WRITE0-RL-06: open report snapshots per World; the alarm fires at 80%.
pub const OPEN_SNAPSHOTS_PER_WORLD: i32 = 100_000;
/// Most reports one list answer names.
pub const REPORT_PAGE: i64 = 32;
/// Most OPEN reports one expiry run ends, one transaction each.
pub const REPORT_EXPIRY_BATCH: u32 = 64;
const HOUR_MS: i64 = 3_600_000;
const MAX_REVISION: u64 = u64::MAX;

/// The node a staff request must be scoped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaffNode {
    pub world: [u8; 16],
    pub channel: [u8; 16],
}

/// One served request: the stored outcome, data for the operator and the
/// live effect to apply after commit. A committed replay re-syncs live effects.
struct Executed {
    stored: StoredOutcome,
    data: Option<Value>,
    effect: Effect,
}
enum Effect {
    None,
    Kick([u8; 16]),
    Mute {
        character: [u8; 16],
        until: Option<u64>,
    },
}

/// What the action decided inside the transaction, before the audit row.
struct Decision {
    result: ResultCode,
    data: Option<Value>,
    effect: Effect,
}
impl Decision {
    fn refused(result: ResultCode) -> Self {
        Self {
            result,
            data: None,
            effect: Effect::None,
        }
    }
}

/// WRITE0 §6 admission of one item-text report; each refusal stores nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportAdmission {
    /// The report is OPEN; `first` is false for a deduplicated repeat, which
    /// returns the first report. `alarm` is set at 80% of WRITE0-RL-06.
    Open {
        report_id: String,
        first: bool,
        alarm: bool,
    },
    RateLimited,
    Quota,
    Capacity,
    Stale,
}

/// The reporter's view the report is about (WRITE0 §6: the revision read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTextReport {
    pub world: [u8; 16],
    pub reporter_character: [u8; 16],
    pub item_instance: [u8; 16],
    pub text_revision: u64,
}

fn uuid(bytes: &[u8; 16]) -> String {
    crate::node::uuid_text(bytes)
}
fn new_id() -> Result<String, StaffError> {
    Ok(uuid(
        &crate::node::operator_files::uuid_v7().map_err(|_| Error::Invalid)?,
    ))
}
fn same_uuid(text: &str, bytes: &[u8; 16]) -> bool {
    crate::node::uuid_bytes(text).as_ref() == Some(bytes)
}
fn ms(value: i64) -> Result<u64, StaffError> {
    Ok(u64::try_from(value).map_err(|_| Error::Clock)?)
}

/// The answer bytes of one control message: the stored outcome, then
/// optionally `\n` and bounded JCS data; `receipt-stored`, or `rejected`.
fn answer(stored: &StoredOutcome, data: Option<&Value>) -> Vec<u8> {
    let mut bytes = stored.bytes.clone();
    if let Some(data) = data {
        let room = signature::MAX_ANSWER_BYTES.saturating_sub(bytes.len() + 1);
        if let Ok(encoded) = oteryn_release_manifest::jcs::encode(data, room) {
            bytes.push(b'\n');
            bytes.extend_from_slice(&encoded);
        }
    }
    bytes
}

async fn begin_at(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    history: &History,
    request: VerifiedRequest,
    now: Option<u64>,
) -> Result<Begin, StaffError> {
    #[cfg(test)]
    if let Some(now) = now {
        return super::staff_action::fixture_begin_in_transaction(
            connection,
            trust,
            history,
            request,
            Ok(now),
        )
        .await;
    }
    // Production always reads the synchronized clock inside the participant.
    #[cfg(not(test))]
    let _ = now;
    begin_in_transaction(connection, trust, history, request).await
}

/// The ACTIVE staff mutes of `character` in the staff `environment` still
/// running at `now`, longest end. Another environment's sanctions never apply.
async fn active_mute_until(
    connection: &mut PgConnection,
    environment: &str,
    character: &str,
    now: i64,
) -> Result<Option<u64>, StaffError> {
    let until: Option<i64> = sqlx::query_scalar(
        "SELECT max(ends_at) FROM game_character_sanctions \
         WHERE character_id=$1::uuid AND status='ACTIVE' AND ends_at>$2 AND environment=$3",
    )
    .bind(character)
    .bind(now)
    .bind(environment)
    .fetch_one(&mut *connection)
    .await?;
    until.map(ms).transpose()
}

/// Database truth for replay, including ended mutes so no clock is needed.
async fn latest_active_mute_end(
    connection: &mut PgConnection,
    environment: &str,
    character: &str,
) -> Result<Option<u64>, StaffError> {
    let until: Option<i64> = sqlx::query_scalar(
        "SELECT max(ends_at) FROM game_character_sanctions \
         WHERE character_id=$1::uuid AND kind='MUTE' AND status='ACTIVE' AND environment=$2",
    )
    .bind(character)
    .bind(environment)
    .fetch_one(&mut *connection)
    .await?;
    until.map(ms).transpose()
}

fn kick_session(
    desk: &StaffDesk,
    environment: &str,
    target: &Target,
) -> Result<Option<[u8; 16]>, StaffError> {
    Ok(match target {
        Target::Character { character_id } => {
            let bytes = crate::node::uuid_bytes(character_id).ok_or(Error::Invalid)?;
            desk.session_for_character(&bytes)
        }
        Target::Account { account_ref } => desk.session_for_account_ref(environment, account_ref),
        _ => return Err(Error::Invalid.into()),
    })
}

/// The live effect of an identical retry of a COMMITTED request: a mute or an
/// unmute re-syncs the desk to database truth. A kick is bound to the session
/// present at its first execution, so its replay applies nothing and never
/// kicks a reconnected session; a lost kick needs a fresh signed request.
async fn replay_effect(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    action: &Action,
) -> Result<Effect, StaffError> {
    let character = match action {
        Action::Mute { character_id, .. } => Some(character_id.clone()),
        Action::Unmute { sanction_id } => {
            sqlx::query_scalar::<_, String>(
                "SELECT character_id::text FROM game_character_sanctions \
             WHERE sanction_id=$1::uuid AND environment=$2",
            )
            .bind(sanction_id)
            .bind(trust.environment())
            .fetch_optional(&mut *connection)
            .await?
        }
        _ => return Ok(Effect::None),
    };
    let Some(character) = character else {
        return Ok(Effect::None);
    };
    let until = latest_active_mute_end(connection, trust.environment(), &character).await?;
    Ok(Effect::Mute {
        character: crate::node::uuid_bytes(&character).ok_or(Error::Invalid)?,
        until,
    })
}

/// The target Character if it lives in this node's World.
async fn character_in_world(
    connection: &mut PgConnection,
    character: &str,
    world: &[u8; 16],
) -> Result<Option<String>, StaffError> {
    Ok(sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=$1::uuid AND world_id=$2::uuid",
    )
    .bind(character)
    .bind(uuid(world))
    .fetch_optional(&mut *connection)
    .await?)
}

async fn decide(
    connection: &mut PgConnection,
    pending: &Pending,
    desk: &StaffDesk,
    trust: &VerifierTrust,
    node: StaffNode,
    action_id: &str,
) -> Result<Decision, StaffError> {
    let request = pending.request();
    let now = pending.occurred_at();
    let principal = &request.signed().principal;
    let role = trust.role_of(principal)?;
    let kind = request.request().action.kind();
    if !role.allows(kind) {
        return Ok(Decision::refused(ResultCode::TierInsufficient));
    }
    match &request.request().action {
        Action::Inspect { character_id } => {
            let Some(name) = character_in_world(connection, character_id, &node.world).await?
            else {
                return Ok(Decision::refused(ResultCode::TargetNotFound));
            };
            let bytes = crate::node::uuid_bytes(character_id).ok_or(Error::Invalid)?;
            let mutes = sqlx::query(
                "SELECT sanction_id::text,ends_at FROM game_character_sanctions \
                 WHERE character_id=$1::uuid AND status='ACTIVE' AND ends_at>$2 \
                 AND environment=$3 ORDER BY ends_at DESC LIMIT 16",
            )
            .bind(character_id)
            .bind(now)
            .bind(trust.environment())
            .fetch_all(&mut *connection)
            .await?
            .iter()
            .map(|row| {
                Ok::<_, sqlx::Error>(json!({
                    "sanction_id": row.try_get::<String, _>(0)?,
                    "ends_at": row.try_get::<i64, _>(1)?.to_string(),
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;
            let reports: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_text_reports \
                 WHERE reporter_character_id=$1::uuid AND state='OPEN'",
            )
            .bind(character_id)
            .fetch_one(&mut *connection)
            .await?;
            Ok(Decision {
                result: ResultCode::Ok,
                data: Some(json!({
                    "character_id": character_id,
                    "name": name,
                    "online": desk.session_for_character(&bytes).is_some(),
                    "active_mutes": mutes,
                    "open_reports_filed": reports.to_string(),
                })),
                effect: Effect::None,
            })
        }
        Action::Kick { target } => {
            let session = kick_session(desk, trust.environment(), target)?;
            Ok(match session {
                Some(session) => Decision {
                    result: ResultCode::Ok,
                    data: None,
                    effect: Effect::Kick(session),
                },
                None => Decision::refused(ResultCode::TargetNotInChannel),
            })
        }
        Action::Mute {
            character_id,
            duration_ms,
        } => {
            if character_in_world(connection, character_id, &node.world)
                .await?
                .is_none()
            {
                return Ok(Decision::refused(ResultCode::TargetNotFound));
            }
            let duration = i64::try_from(duration_ms.0).map_err(|_| Error::Invalid)?;
            let ends_at = now.checked_add(duration).ok_or(Error::Invalid)?;
            let sanction = new_id()?;
            sqlx::query(
                "INSERT INTO game_character_sanctions(sanction_id,environment,character_id,kind,\
                 starts_at,ends_at,actor_principal,mute_action_id,status,revision) \
                 VALUES($1::uuid,$2,$3::uuid,'MUTE',$4,$5,$6,$7::uuid,'ACTIVE',1)",
            )
            .bind(&sanction)
            .bind(trust.environment())
            .bind(character_id)
            .bind(now)
            .bind(ends_at)
            .bind(principal)
            .bind(action_id)
            .execute(&mut *connection)
            .await?;
            let until =
                active_mute_until(connection, trust.environment(), character_id, now).await?;
            Ok(Decision {
                result: ResultCode::Ok,
                data: Some(json!({"sanction_id": sanction, "ends_at": ends_at.to_string()})),
                effect: Effect::Mute {
                    character: crate::node::uuid_bytes(character_id).ok_or(Error::Invalid)?,
                    until,
                },
            })
        }
        Action::Unmute { sanction_id } => {
            let row = sqlx::query(
                "SELECT s.character_id::text,s.status,s.ends_at FROM game_character_sanctions s \
                 JOIN game_character_roots r ON r.character_id=s.character_id \
                 WHERE s.sanction_id=$1::uuid AND s.environment=$2 AND r.world_id=$3::uuid \
                 FOR UPDATE OF s",
            )
            .bind(sanction_id)
            .bind(trust.environment())
            .bind(uuid(&node.world))
            .fetch_optional(&mut *connection)
            .await?;
            let Some(row) = row else {
                return Ok(Decision::refused(ResultCode::TargetNotFound));
            };
            let character: String = row.try_get(0)?;
            let status: String = row.try_get(1)?;
            let ends_at: i64 = row.try_get(2)?;
            if status != "ACTIVE" || ends_at <= now {
                return Ok(Decision::refused(ResultCode::AlreadyReleased));
            }
            let changed = sqlx::query(
                "UPDATE game_character_sanctions SET status='LIFTED',revision=2,lifted_at=$2,\
                 lift_action_id=$3::uuid WHERE sanction_id=$1::uuid AND status='ACTIVE'",
            )
            .bind(sanction_id)
            .bind(now)
            .bind(action_id)
            .execute(&mut *connection)
            .await?
            .rows_affected();
            if changed != 1 {
                return Ok(Decision::refused(ResultCode::AlreadyReleased));
            }
            let until = active_mute_until(connection, trust.environment(), &character, now).await?;
            Ok(Decision {
                result: ResultCode::Ok,
                data: None,
                effect: Effect::Mute {
                    character: crate::node::uuid_bytes(&character).ok_or(Error::Invalid)?,
                    until,
                },
            })
        }
        Action::ReportsList { state, after } => {
            let state = match state {
                ReportState::Open => "OPEN",
                ReportState::Closed => "CLOSED",
                ReportState::Expired => "EXPIRED",
            };
            let after_entry = match after {
                Some(cursor) => Some(i64::try_from(cursor.entry_seq.0).map_err(|_| Error::Bounds)?),
                None => None,
            };
            // Every report enters a state under its World row lock (migration
            // 0134); waiting on it here lets each writer still in flight commit
            // first, and the read below then sees every entry drawn before.
            sqlx::query(
                "SELECT 1 FROM game_item_text_report_worlds WHERE world_id=$1::uuid FOR SHARE",
            )
            .bind(uuid(&node.world))
            .fetch_optional(&mut *connection)
            .await?;
            // Keyset order: a page starts strictly after the signed cursor in
            // entry order, so a report that enters this state later (a close,
            // an expiry) still follows it. One row past the page tells whether
            // another page follows.
            let rows = sqlx::query(
                "SELECT report_id::text,item_instance_id::text,text_revision::text,created_at,\
                 writer_character_id::text,snapshot_text,cleared_action_id IS NOT NULL,entry_seq \
                 FROM game_item_text_reports WHERE world_id=$1::uuid AND state=$2 \
                 AND ($4::bigint IS NULL OR (entry_seq,report_id)>($4::bigint,$5::uuid)) \
                 ORDER BY entry_seq,report_id LIMIT $3",
            )
            .bind(uuid(&node.world))
            .bind(state)
            .bind(REPORT_PAGE + 1)
            .bind(after_entry)
            .bind(after.as_ref().map(|cursor| cursor.report_id.as_str()))
            .fetch_all(&mut *connection)
            .await?;
            // The snapshot reaches only the operator's answer, never a log;
            // rows past the page or the answer bound are left for the next page.
            let mut reports = Vec::new();
            let mut next = None;
            let mut size = signature::MAX_OUTCOME_BYTES + 512;
            let mut truncated = false;
            for row in rows {
                let report_id: String = row.try_get(0)?;
                let created_at: i64 = row.try_get(3)?;
                let entry_seq: i64 = row.try_get(7)?;
                let report = json!({
                    "report_id": report_id,
                    "item_instance_id": row.try_get::<String, _>(1)?,
                    "text_revision": row.try_get::<String, _>(2)?,
                    "created_at": created_at.to_string(),
                    "writer_character_id": row.try_get::<Option<String>, _>(4)?,
                    "text": row.try_get::<Option<String>, _>(5)?,
                    "cleared": row.try_get::<bool, _>(6)?,
                });
                let length = serde_json::to_vec(&report).map_or(usize::MAX, |b| b.len());
                size = size.saturating_add(length + 1);
                if reports.len() == REPORT_PAGE as usize || size > signature::MAX_ANSWER_BYTES {
                    truncated = true;
                    break;
                }
                next = Some(json!({"entry_seq": entry_seq.to_string(), "report_id": report_id}));
                reports.push(report);
            }
            Ok(Decision {
                result: ResultCode::Ok,
                data: Some(json!({
                    "state": state,
                    "reports": reports,
                    "truncated": truncated,
                    // Sign this as the next request's `after` to read on.
                    "next": if truncated { next } else { None },
                })),
                effect: Effect::None,
            })
        }
        Action::ReportClose { report_id } => {
            let Some((account, world)) = report_owner(connection, report_id, &node.world).await?
            else {
                return Ok(Decision::refused(ResultCode::TargetNotFound));
            };
            Ok(
                match end_report(
                    connection,
                    report_id,
                    &account,
                    &world,
                    now,
                    Some(action_id),
                )
                .await?
                {
                    true => Decision {
                        result: ResultCode::Ok,
                        data: None,
                        effect: Effect::None,
                    },
                    false => Decision::refused(ResultCode::Stale),
                },
            )
        }
        Action::ReportClearText {
            report_id,
            text_revision,
        } => {
            clear_report_text(
                connection,
                report_id,
                *text_revision,
                &node.world,
                action_id,
            )
            .await
        }
        Action::RosterActivate { .. } => Err(StaffError::UnsupportedAction),
    }
}

/// The reporter account and World of a report in this node's World.
async fn report_owner(
    connection: &mut PgConnection,
    report_id: &str,
    world: &[u8; 16],
) -> Result<Option<(String, String)>, StaffError> {
    let row = sqlx::query(
        "SELECT reporter_account_id::text,world_id::text FROM game_item_text_reports \
         WHERE report_id=$1::uuid AND world_id=$2::uuid",
    )
    .bind(report_id)
    .bind(uuid(world))
    .fetch_optional(&mut *connection)
    .await?;
    row.map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .transpose()
}

async fn lock_open_counters(
    connection: &mut PgConnection,
    account: &str,
    world: &str,
) -> Result<(i16, i32), StaffError> {
    sqlx::query(
        "INSERT INTO game_item_text_report_accounts(account_id,open_reports) VALUES($1::uuid,0) \
         ON CONFLICT DO NOTHING",
    )
    .bind(account)
    .execute(&mut *connection)
    .await?;
    let open: i16 = sqlx::query_scalar(
        "SELECT open_reports FROM game_item_text_report_accounts WHERE account_id=$1::uuid FOR UPDATE",
    )
    .bind(account)
    .fetch_one(&mut *connection)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_text_report_worlds(world_id,open_snapshots) VALUES($1::uuid,0) \
         ON CONFLICT DO NOTHING",
    )
    .bind(world)
    .execute(&mut *connection)
    .await?;
    let snapshots: i32 = sqlx::query_scalar(
        "SELECT open_snapshots FROM game_item_text_report_worlds WHERE world_id=$1::uuid FOR UPDATE",
    )
    .bind(world)
    .fetch_one(&mut *connection)
    .await?;
    Ok((open, snapshots))
}

/// WRITE0 §6 close or expiry: counters, then the report, then one conditional
/// update from OPEN; the counters fall only if that update changed a row.
async fn end_report(
    connection: &mut PgConnection,
    report_id: &str,
    account: &str,
    world: &str,
    now: i64,
    closed_by: Option<&str>,
) -> Result<bool, StaffError> {
    lock_open_counters(connection, account, world).await?;
    sqlx::query("SELECT state FROM game_item_text_reports WHERE report_id=$1::uuid FOR UPDATE")
        .bind(report_id)
        .fetch_one(&mut *connection)
        .await?;
    let changed = match closed_by {
        // A close never ends before the report began: the staff request clock
        // and the reporter's node clock may differ.
        Some(action) => sqlx::query(
            "UPDATE game_item_text_reports SET state='CLOSED',revision=2,\
             ended_at=GREATEST($2,created_at),closed_action_id=$3::uuid \
             WHERE report_id=$1::uuid AND state='OPEN'",
        )
        .bind(report_id)
        .bind(now)
        .bind(action),
        None => sqlx::query(
            "UPDATE game_item_text_reports SET state='EXPIRED',revision=2,ended_at=$2,\
             snapshot_text=NULL WHERE report_id=$1::uuid AND state='OPEN' \
             AND expires_at<=LEAST($2,floor(extract(epoch FROM clock_timestamp())*1000)::bigint)",
        )
        .bind(report_id)
        .bind(now),
    }
    .execute(&mut *connection)
    .await?
    .rows_affected()
        == 1;
    if changed {
        sqlx::query(
            "UPDATE game_item_text_report_accounts SET open_reports=open_reports-1 \
             WHERE account_id=$1::uuid",
        )
        .bind(account)
        .execute(&mut *connection)
        .await?;
        sqlx::query(
            "UPDATE game_item_text_report_worlds SET open_snapshots=open_snapshots-1 \
             WHERE world_id=$1::uuid",
        )
        .bind(world)
        .execute(&mut *connection)
        .await?;
    }
    Ok(changed)
}

/// WRITE0 §6 removal: under the item row lock, delete the live text only while
/// it is still the reported revision, and advance the revision; else STALE.
async fn clear_report_text(
    connection: &mut PgConnection,
    report_id: &str,
    text_revision: Decimal,
    world: &[u8; 16],
    action_id: &str,
) -> Result<Decision, StaffError> {
    let row = sqlx::query(
        "SELECT item_instance_id::text,text_revision::text FROM game_item_text_reports \
         WHERE report_id=$1::uuid AND world_id=$2::uuid",
    )
    .bind(report_id)
    .bind(uuid(world))
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(Decision::refused(ResultCode::TargetNotFound));
    };
    let item: String = row.try_get(0)?;
    let reported: String = row.try_get(1)?;
    if reported != text_revision.0.to_string() {
        return Ok(Decision::refused(ResultCode::Stale));
    }
    sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=$1::uuid FOR UPDATE")
        .bind(&item)
        .fetch_optional(&mut *connection)
        .await?;
    let last: Option<String> = sqlx::query_scalar(
        "SELECT last_revision::text FROM game_item_text_revisions \
         WHERE item_instance_id=$1::uuid FOR UPDATE",
    )
    .bind(&item)
    .fetch_optional(&mut *connection)
    .await?;
    let current: Option<String> = sqlx::query_scalar(
        "SELECT text_revision::text FROM game_item_texts WHERE item_instance_id=$1::uuid FOR UPDATE",
    )
    .bind(&item)
    .fetch_optional(&mut *connection)
    .await?;
    let last = last.and_then(|last| last.parse::<u64>().ok());
    let (Some(last), Some(current)) = (last, current) else {
        return Ok(Decision::refused(ResultCode::Stale));
    };
    if current != reported || last == MAX_REVISION {
        return Ok(Decision::refused(ResultCode::Stale));
    }
    sqlx::query("DELETE FROM game_item_texts WHERE item_instance_id=$1::uuid")
        .bind(&item)
        .execute(&mut *connection)
        .await?;
    sqlx::query(
        "UPDATE game_item_text_revisions SET last_revision=$2::text::numeric \
         WHERE item_instance_id=$1::uuid",
    )
    .bind(&item)
    .bind((last + 1).to_string())
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE game_item_text_reports SET cleared_action_id=$2::uuid \
         WHERE report_id=$1::uuid AND cleared_action_id IS NULL",
    )
    .bind(report_id)
    .bind(action_id)
    .execute(&mut *connection)
    .await?;
    Ok(Decision {
        result: ResultCode::Ok,
        data: None,
        effect: Effect::None,
    })
}

/// WRITE0 §6 steps 2-7 in the caller's transaction, after its step-1 fence.
pub(crate) async fn create_item_text_report_in_transaction(
    connection: &mut PgConnection,
    report: &ItemTextReport,
    now: i64,
    world_capacity: i32,
) -> Result<ReportAdmission, StaffError> {
    let reporter = uuid(&report.reporter_character);
    let world = uuid(&report.world);
    let item = uuid(&report.item_instance);
    let account: String = sqlx::query_scalar(
        "SELECT account_id::text FROM game_character_roots WHERE character_id=$1::uuid AND world_id=$2::uuid",
    )
    .bind(&reporter)
    .bind(&world)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(Error::Binding)?;
    let hour = now.div_euclid(HOUR_MS);
    // 2. Counter rows, upserted then locked, in the fixed order.
    sqlx::query(
        "INSERT INTO game_item_text_report_hourly(reporter_character_id,hour,reports) \
         VALUES($1::uuid,$2,0) ON CONFLICT DO NOTHING",
    )
    .bind(&reporter)
    .bind(hour)
    .execute(&mut *connection)
    .await?;
    let hourly: i16 = sqlx::query_scalar(
        "SELECT reports FROM game_item_text_report_hourly \
         WHERE reporter_character_id=$1::uuid AND hour=$2 FOR UPDATE",
    )
    .bind(&reporter)
    .bind(hour)
    .fetch_one(&mut *connection)
    .await?;
    let (open, snapshots) = lock_open_counters(connection, &account, &world).await?;
    // 3. Dedup among OPEN reports only.
    let existing = open_report(connection, &reporter, &item, report.text_revision).await?;
    if let Some(report_id) = existing {
        return Ok(ReportAdmission::Open {
            report_id,
            first: false,
            alarm: false,
        });
    }
    // 4. Limits against the locked counters.
    if hourly >= REPORTS_PER_HOUR {
        return Ok(ReportAdmission::RateLimited);
    }
    if open >= OPEN_REPORTS_PER_ACCOUNT {
        return Ok(ReportAdmission::Quota);
    }
    if snapshots >= world_capacity.min(OPEN_SNAPSHOTS_PER_WORLD) {
        return Ok(ReportAdmission::Capacity);
    }
    // 5. The reported revision is still the item's current text.
    let current: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM game_item_texts t \
         JOIN game_item_text_revisions r ON r.item_instance_id=t.item_instance_id \
         JOIN game_item_instances i ON i.item_instance_id=t.item_instance_id \
         WHERE t.item_instance_id=$1::uuid AND i.world_id=$2::uuid \
         AND t.text_revision=$3::text::numeric AND r.last_revision=t.text_revision \
         FOR SHARE OF t",
    )
    .bind(&item)
    .bind(&world)
    .bind(report.text_revision.to_string())
    .fetch_optional(&mut *connection)
    .await?;
    if current.is_none() {
        return Ok(ReportAdmission::Stale);
    }
    // 6. Insert the snapshot, copied in SQL so the text never enters this
    // process; a uniqueness race maps to the first report.
    let report_id = new_id()?;
    let inserted: Option<String> = sqlx::query_scalar(
        "INSERT INTO game_item_text_reports(report_id,world_id,reporter_character_id,\
         reporter_account_id,item_instance_id,text_revision,snapshot_text,writer_character_id,\
         written_at,created_at,expires_at,state,revision) \
         SELECT $1::uuid,$2::uuid,$3::uuid,$4::uuid,t.item_instance_id,t.text_revision,t.text,\
         t.writer_character_id,t.written_at,$6,$7,'OPEN',1 FROM game_item_texts t \
         WHERE t.item_instance_id=$5::uuid \
         ON CONFLICT (reporter_character_id,item_instance_id,text_revision) WHERE state='OPEN' \
         DO NOTHING RETURNING report_id::text",
    )
    .bind(&report_id)
    .bind(&world)
    .bind(&reporter)
    .bind(&account)
    .bind(&item)
    .bind(now)
    .bind(now.checked_add(REPORT_RETENTION_MS).ok_or(Error::Clock)?)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(report_id) = inserted else {
        let first = open_report(connection, &reporter, &item, report.text_revision).await?;
        return Ok(ReportAdmission::Open {
            report_id: first.ok_or(StaffError::Conflict)?,
            first: false,
            alarm: false,
        });
    };
    // 7. Increment the three counters.
    sqlx::query(
        "UPDATE game_item_text_report_hourly SET reports=reports+1 \
         WHERE reporter_character_id=$1::uuid AND hour=$2",
    )
    .bind(&reporter)
    .bind(hour)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE game_item_text_report_accounts SET open_reports=open_reports+1 WHERE account_id=$1::uuid",
    )
    .bind(&account)
    .execute(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE game_item_text_report_worlds SET open_snapshots=open_snapshots+1 WHERE world_id=$1::uuid",
    )
    .bind(&world)
    .execute(&mut *connection)
    .await?;
    let alarm = i64::from(snapshots) + 1 >= i64::from(OPEN_SNAPSHOTS_PER_WORLD) * 4 / 5;
    Ok(ReportAdmission::Open {
        report_id,
        first: true,
        alarm,
    })
}

async fn open_report(
    connection: &mut PgConnection,
    reporter: &str,
    item: &str,
    revision: u64,
) -> Result<Option<String>, StaffError> {
    Ok(sqlx::query_scalar(
        "SELECT report_id::text FROM game_item_text_reports WHERE reporter_character_id=$1::uuid \
         AND item_instance_id=$2::uuid AND text_revision=$3::text::numeric AND state='OPEN'",
    )
    .bind(reporter)
    .bind(item)
    .bind(revision.to_string())
    .fetch_optional(&mut *connection)
    .await?)
}

impl DurabilityRoot {
    /// GM-1: serve one staff control message and return the exact answer.
    /// The signature and the node scope are checked before any connection.
    pub async fn serve_staff_control(
        &self,
        trust: Arc<VerifierTrust>,
        history: Arc<History>,
        desk: Arc<StaffDesk>,
        node: StaffNode,
        message: ControlMessage,
    ) -> Result<Vec<u8>, StaffError> {
        match message {
            ControlMessage::Request(signed) => {
                self.serve_staff_request(trust, history, desk, node, signed, None)
                    .await
            }
            ControlMessage::Receipt(signed) => {
                self.store_staff_receipt(trust, signed).await?;
                Ok(b"receipt-stored".to_vec())
            }
        }
    }

    /// The source-included PostgreSQL fixture runs the same executor on its
    /// own synchronized clock reading.
    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "Used by the source-included PostgreSQL fixture, not library unit tests"
    )]
    pub(crate) async fn fixture_serve_staff_request(
        &self,
        trust: Arc<VerifierTrust>,
        history: Arc<History>,
        desk: Arc<StaffDesk>,
        node: StaffNode,
        signed: SignedBytes,
        now: u64,
    ) -> Result<Vec<u8>, StaffError> {
        self.serve_staff_request(trust, history, desk, node, signed, Some(now))
            .await
    }

    async fn serve_staff_request(
        &self,
        trust: Arc<VerifierTrust>,
        history: Arc<History>,
        desk: Arc<StaffDesk>,
        node: StaffNode,
        signed: SignedBytes,
        now: Option<u64>,
    ) -> Result<Vec<u8>, StaffError> {
        let request = trust.verify_request(signed)?;
        if matches!(request.request().action, Action::RosterActivate { .. }) {
            return Err(StaffError::UnsupportedAction);
        }
        let in_scope = request.request().scope.as_ref().is_some_and(|scope| {
            same_uuid(&scope.world_id, &node.world) && same_uuid(&scope.channel_id, &node.channel)
        });
        if !in_scope {
            return Err(Error::Binding.into());
        }
        // A principal without a role fails closed before anything is written.
        trust.role_of(&request.signed().principal)?;
        if let Action::Mute { duration_ms, .. } = &request.request().action
            && duration_ms.0 > MAX_MUTE_MS
        {
            return Err(Error::Invalid.into());
        }
        let contract = Contract::registered()?;
        let worker_desk = Arc::clone(&desk);
        let executed = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result: Result<Executed, StaffError> = async {
                        let action = request.request().action.clone();
                        match begin_at(&mut tx, &trust, &history, request, now).await? {
                            Begin::Replay(stored) => {
                                let effect = if stored.outcome.disposition == Disposition::Committed
                                {
                                    replay_effect(&mut tx, &trust, &action).await?
                                } else {
                                    Effect::None
                                };
                                Ok(Executed {
                                    stored,
                                    data: None,
                                    effect,
                                })
                            }
                            Begin::Fresh(pending) => {
                                let action_id = new_id()?;
                                let transaction_id = new_id()?;
                                let decision = decide(
                                    &mut tx,
                                    &pending,
                                    &worker_desk,
                                    &trust,
                                    node,
                                    &action_id,
                                )
                                .await?;
                                let occurred_at = Decimal(ms(pending.occurred_at())?);
                                let committed = decision.result == ResultCode::Ok;
                                let stored = finish_in_transaction(
                                    &mut tx,
                                    pending,
                                    contract,
                                    Outcome {
                                        disposition: if committed {
                                            Disposition::Committed
                                        } else {
                                            Disposition::Refused
                                        },
                                        result: decision.result,
                                        action_id,
                                        transaction_id,
                                        occurred_at,
                                    },
                                    super::character_authority::SERVER_BUILD_ID,
                                )
                                .await?;
                                Ok(Executed {
                                    stored,
                                    data: decision.data,
                                    effect: decision.effect,
                                })
                            }
                        }
                    }
                    .await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await??;
        // The transaction committed: apply the audited effect or its replay re-sync.
        let mut data = executed.data;
        match executed.effect {
            Effect::None => {}
            Effect::Mute { character, until } => desk.set_mute(character, until),
            Effect::Kick(session) => {
                let result: KickResult = desk.kick(&session).await;
                data = Some(json!({"kick": result.token()}));
            }
        }
        Ok(answer(&executed.stored, data.as_ref()))
    }

    /// The end (Unix ms) of `character`'s longest staff mute in the staff
    /// `environment` running at `now`.
    pub async fn staff_active_mute_until(
        &self,
        environment: &str,
        character: [u8; 16],
        now: u64,
    ) -> Result<Option<u64>, StaffError> {
        let now = i64::try_from(now).map_err(|_| Error::Clock)?;
        let environment = environment.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    Ok(active_mute_until(&mut tx, &environment, &uuid(&character), now).await)
                })
            })
            .await?
    }

    /// WRITE0 §6 report admission in one transaction. The caller's fence
    /// (WRITE-WIRE-1, the player report intent) is not composed yet.
    pub async fn create_item_text_report(
        &self,
        report: ItemTextReport,
        now: u64,
        world_capacity: i32,
    ) -> Result<ReportAdmission, StaffError> {
        let now = i64::try_from(now).map_err(|_| Error::Clock)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = create_item_text_report_in_transaction(
                        &mut tx,
                        &report,
                        now,
                        world_capacity,
                    )
                    .await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?
    }

    /// WRITE0 §6 expiry: each OPEN report past WRITE0-RL-03 expires in its own
    /// transaction (the close lock order); then ended snapshots past it are
    /// deleted and hourly rows of finished hours removed. Returns reports expired.
    /// Retention ends at the earlier of `now` and the database clock, which
    /// migration 0134 enforces on every snapshot removal.
    pub async fn expire_item_text_reports(&self, now: u64) -> Result<u64, StaffError> {
        let now = i64::try_from(now).map_err(|_| Error::Clock)?;
        let mut expired = 0_u64;
        for _ in 0..REPORT_EXPIRY_BATCH {
            let ended = self
                .try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        let result = async {
                            let row = sqlx::query(
                                "SELECT report_id::text,reporter_account_id::text,world_id::text \
                                 FROM game_item_text_reports WHERE state='OPEN' AND expires_at<=\
                                 LEAST($1,floor(extract(epoch FROM clock_timestamp())*1000)::bigint) \
                                 ORDER BY expires_at,report_id LIMIT 1",
                            )
                            .bind(now)
                            .fetch_optional(&mut *tx)
                            .await?;
                            let Some(row) = row else {
                                return Ok(None);
                            };
                            let (report, account, world): (String, String, String) =
                                (row.try_get(0)?, row.try_get(1)?, row.try_get(2)?);
                            Ok::<_, StaffError>(Some(
                                end_report(&mut tx, &report, &account, &world, now, None).await?,
                            ))
                        }
                        .await;
                        if result.is_ok() {
                            commit_semantic_transaction(tx, deadline).await?;
                        }
                        Ok(result)
                    })
                })
                .await??;
            match ended {
                None => break,
                Some(true) => expired = expired.saturating_add(1),
                Some(false) => {}
            }
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        sqlx::query(
                            "UPDATE game_item_text_reports SET snapshot_text=NULL WHERE report_id IN \
                             (SELECT report_id FROM game_item_text_reports WHERE state<>'OPEN' \
                              AND snapshot_text IS NOT NULL AND expires_at<=LEAST($1,\
                              floor(extract(epoch FROM clock_timestamp())*1000)::bigint) LIMIT 1024)",
                        )
                        .bind(now)
                        .execute(&mut *tx)
                        .await?;
                        sqlx::query("DELETE FROM game_item_text_report_hourly WHERE hour<$1")
                            .bind(now.div_euclid(HOUR_MS))
                            .execute(&mut *tx)
                            .await?;
                        Ok::<_, StaffError>(())
                    }
                    .await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await??;
        Ok(expired)
    }
}

/// One retained operator file pair from the ops state directory: the signed
/// `staff-link-*.json` control message and, when the node answered, its
/// `.outcome` (the stored outcome bytes, or `receipt-stored`).
#[derive(Debug, Clone)]
pub struct RetainedLink {
    pub message: ControlMessage,
    pub outcome: Option<Vec<u8>>,
}
/// What a restore replay did with one link above the restored head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreKind {
    /// A COMMITTED mute, re-applied through the request's own checks.
    Replayed,
    /// A COMMITTED mute the current state refuses: no effect is applied, and
    /// the receipted outcome is kept as chain history.
    RefusedNow(ResultCode),
    /// A REFUSED mutation with its REFUSED receipt: never run, refusal stored.
    RefusalStored,
    /// A read: its outcome is stored, nothing is disclosed.
    Recorded,
    /// A COMMITTED unmute, kick or report transition: recorded, never re-run.
    NotReapplied,
    /// An operator receipt, stored on its chain link.
    Receipt,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreStep {
    pub sequence: u64,
    pub operation_id: String,
    pub kind: RestoreKind,
}
#[derive(Debug)]
pub enum RestoreError {
    /// Requests whose execution is not proven by a verifying receipt. Nothing
    /// was applied; they wait for the signed `restore resolve` of DATA-RESTORE-OPS-2.
    Ambiguous(Vec<String>),
    Staff(StaffError),
}
impl From<StaffError> for RestoreError {
    fn from(error: StaffError) -> Self {
        Self::Staff(error)
    }
}
impl From<Error> for RestoreError {
    fn from(error: Error) -> Self {
        Self::Staff(error.into())
    }
}
impl From<super::DurabilityError> for RestoreError {
    fn from(error: super::DurabilityError) -> Self {
        Self::Staff(error.into())
    }
}

/// A retained link above the restored head, before chain selection.
struct Candidate {
    link: signature::ChainLink,
    operation_id: String,
    /// The node answered it: a request has its outcome file, a receipt was stored.
    confirmed: bool,
    outcome: Option<Vec<u8>>,
}
enum Planned {
    Request {
        request: VerifiedRequest,
        outcome: Outcome,
        bytes: Vec<u8>,
        kind: RestoreKind,
    },
    Receipt(signature::VerifiedReceipt),
}

fn retained_candidate(item: RetainedLink) -> Result<(u64, Candidate), RestoreError> {
    use oteryn_release_manifest::jcs;
    let (signed, kind) = match item.message {
        ControlMessage::Request(signed) => (signed, signature::LinkKind::Request),
        ControlMessage::Receipt(signed) => (signed, signature::LinkKind::Receipt),
    };
    let digest = signature::sha256(&signed.bytes);
    let (sequence, previous, roster, operation_id, confirmed) = if kind
        == signature::LinkKind::Request
    {
        let value: signature::Request =
            jcs::decode(&signed.bytes, signature::MAX_REQUEST_BYTES).map_err(|_| Error::Invalid)?;
        if matches!(value.action, Action::RosterActivate { .. }) {
            // Activations are re-provisioned by the owner, never replayed.
            return Err(Error::Binding.into());
        }
        let confirmed = item.outcome.is_some();
        (
            value.control_seq.0,
            value.previous_link_sha256,
            value.roster_sha256,
            value.operation_id,
            confirmed,
        )
    } else {
        let value: signature::Receipt =
            jcs::decode(&signed.bytes, signature::MAX_REQUEST_BYTES).map_err(|_| Error::Invalid)?;
        let confirmed = item.outcome.as_deref() == Some(b"receipt-stored".as_slice());
        (
            value.control_seq.0,
            value.previous_link_sha256,
            value.roster_sha256,
            value.operation_id,
            confirmed,
        )
    };
    let link = signature::ChainLink {
        sequence,
        previous,
        digest,
        roster,
        fingerprint: String::new(),
        signed,
        kind,
    };
    Ok((
        sequence,
        Candidate {
            link,
            operation_id,
            confirmed,
            outcome: item.outcome,
        },
    ))
}

/// The next link after `(sequence, digest)`: the only candidate, or the only
/// confirmed one among several. Two confirmed successors are a fork; several
/// unconfirmed ones leave the restore ambiguous.
fn next_link(
    pool: &mut [Option<Candidate>],
    sequence: u64,
    digest: &str,
) -> Result<Option<Candidate>, RestoreError> {
    let next = sequence.checked_add(1).ok_or(Error::Chain)?;
    let at: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.as_ref()
                .is_some_and(|c| c.link.sequence == next && c.link.previous == digest)
        })
        .map(|(index, _)| index)
        .collect();
    let chosen = match at.as_slice() {
        [] => return Ok(None),
        [only] => *only,
        several => {
            let confirmed: Vec<usize> = several
                .iter()
                .copied()
                .filter(|index| pool[*index].as_ref().is_some_and(|c| c.confirmed))
                .collect();
            match confirmed.as_slice() {
                [only] => *only,
                [] => {
                    return Err(RestoreError::Ambiguous(
                        several
                            .iter()
                            .filter_map(|index| pool[*index].as_ref())
                            .map(|c| c.operation_id.clone())
                            .collect(),
                    ));
                }
                _ => return Err(Error::Chain.into()),
            }
        }
    };
    Ok(pool[chosen].take())
}

impl DurabilityRoot {
    /// ALPHA-OPS ruling 8 for GM-1 staff data, run by `oteryn-game-ops staff
    /// restore-replay` while the node is stopped after a database restore.
    ///
    /// The restored chain is verified from its first activation to its head
    /// before anything is applied. Retained links above the head must extend
    /// it gaplessly under the current activation; a gap, fork or changed link
    /// stops the replay, including a retained link past a gap without an
    /// outcome. A mutation applies only with its verifying receipt: a
    /// COMMITTED mute is re-run through its own checks at its recorded time
    /// (no freshness), a refusal is stored, and no unmute, kick or report
    /// transition is re-run. Any request without a verifying receipt stops the
    /// replay with nothing applied. A second run is a no-op.
    pub async fn staff_restore_replay(
        &self,
        trust: Arc<VerifierTrust>,
        history: Arc<History>,
        retained: Vec<RetainedLink>,
    ) -> Result<Vec<RestoreStep>, RestoreError> {
        let environment = trust.environment().to_owned();
        if history.current().activation_digest() != trust.activation_digest() {
            return Err(Error::Binding.into());
        }
        let head = self.staff_receipt_head(environment.clone()).await?;
        if head.activation_digest != trust.activation_digest() {
            return Err(Error::Binding.into());
        }
        let mut below: std::collections::BTreeMap<u64, Vec<(String, bool)>> =
            std::collections::BTreeMap::new();
        let mut pool: Vec<Option<Candidate>> = Vec::new();
        for item in retained {
            let (sequence, candidate) = retained_candidate(item)?;
            if sequence <= head.sequence {
                below
                    .entry(sequence)
                    .or_default()
                    .push((candidate.link.digest, candidate.confirmed));
                continue;
            }
            let same = pool
                .iter_mut()
                .flatten()
                .find(|c| c.link.digest == candidate.link.digest);
            match same {
                Some(existing) => {
                    existing.confirmed |= candidate.confirmed;
                    if existing.outcome.is_none() {
                        existing.outcome = candidate.outcome;
                    }
                }
                None => pool.push(Some(candidate)),
            }
        }
        // The whole restored chain verifies before anything above it counts:
        // every link chains and every request carries the same operation,
        // receipt and audit evidence `oteryn-game-ops staff audit-verify`
        // requires, so a rewritten restored row stops the replay.
        let contract = Contract::registered()?;
        let mut cursor = history.cursor();
        let mut after = 0;
        loop {
            let rows = self
                .staff_evidence_page(environment.clone(), after, head.sequence)
                .await?;
            if rows.is_empty() {
                break;
            }
            for row in rows {
                after = row.link.sequence;
                if let signature::VerifiedLink::Request(request) = cursor.push(&row.link)? {
                    // Only the request at the restored head may still await its
                    // receipt, which then is a retained link above the head.
                    super::staff_action::verify_request_evidence(
                        &history,
                        contract,
                        &request,
                        row.operation.as_ref(),
                        row.link.sequence == head.sequence,
                        signature::synchronized_now_ms,
                    )?;
                }
                if let Some(retained) = below.get(&row.link.sequence)
                    && retained
                        .iter()
                        .any(|(digest, confirmed)| *confirmed && *digest != row.link.digest)
                {
                    // An answered link the restored database does not hold.
                    return Err(Error::Chain.into());
                }
            }
        }
        cursor.finish(head.sequence, &head.digest)?;
        if self
            .staff_evidence_has_orphans(environment.clone(), head.sequence)
            .await?
        {
            return Err(Error::Chain.into());
        }
        // Extend the verified head with the retained links, one at a time.
        let (mut sequence, mut digest) = (head.sequence, head.digest.clone());
        let mut chain: Vec<(Candidate, signature::VerifiedLink)> = Vec::new();
        while let Some(mut candidate) = next_link(&mut pool, sequence, &digest)? {
            candidate.link.fingerprint = match candidate.link.kind {
                signature::LinkKind::Receipt => trust
                    .verify_receipt(candidate.link.signed.clone())?
                    .fingerprint()
                    .to_owned(),
                _ => trust
                    .verify_request(candidate.link.signed.clone())?
                    .fingerprint()
                    .to_owned(),
            };
            let verified = cursor.push(&candidate.link)?;
            sequence = candidate.link.sequence;
            digest = candidate.link.digest.clone();
            chain.push((candidate, verified));
        }
        cursor.finish(sequence, &digest)?;
        if pool.iter().flatten().any(|c| c.confirmed) {
            // An answered link off the chain: a gap, a fork or a changed link.
            return Err(Error::Chain.into());
        }
        let disconnected: Vec<String> = pool
            .iter()
            .flatten()
            .filter(|candidate| candidate.link.sequence > sequence)
            .map(|candidate| candidate.operation_id.clone())
            .collect();
        if !disconnected.is_empty() {
            return Err(RestoreError::Ambiguous(disconnected));
        }
        let mut ambiguous = Vec::new();
        let mut planned = Vec::new();
        for (index, (candidate, verified)) in chain.iter().enumerate() {
            let request = match verified {
                signature::VerifiedLink::Receipt(receipt) => {
                    planned.push((candidate.link.sequence, Planned::Receipt(receipt.clone())));
                    continue;
                }
                signature::VerifiedLink::Request(request) => request,
            };
            let Some(bytes) = candidate.outcome.clone() else {
                ambiguous.push(candidate.operation_id.clone());
                continue;
            };
            let outcome: Outcome =
                oteryn_release_manifest::jcs::decode(&bytes, signature::MAX_OUTCOME_BYTES)
                    .map_err(|_| Error::Invalid)?;
            if outcome.canonical_bytes()? != bytes
                || (outcome.disposition == Disposition::Committed)
                    != (outcome.result == ResultCode::Ok)
            {
                return Err(Error::Invalid.into());
            }
            signature::check_execution_stamp(request.request(), outcome.occurred_at)?;
            let action = &request.request().action;
            let kind = if action.kind().is_read() {
                RestoreKind::Recorded
            } else {
                let operation = crate::node::uuid_bytes(&request.request().operation_id)
                    .ok_or(Error::Invalid)?;
                let receipt = chain[index + 1..].iter().find_map(|(_, link)| match link {
                    signature::VerifiedLink::Receipt(receipt)
                        if same_uuid(&receipt.receipt().operation_id, &operation)
                            && receipt.receipt().request_sha256 == request.digest() =>
                    {
                        Some(receipt)
                    }
                    _ => None,
                });
                let Some(receipt) = receipt else {
                    ambiguous.push(candidate.operation_id.clone());
                    continue;
                };
                if receipt.receipt().outcome_sha256 != signature::sha256(&bytes)
                    || receipt.receipt().disposition != outcome.disposition
                {
                    return Err(Error::Receipt.into());
                }
                match (outcome.disposition, action) {
                    (Disposition::Refused, _) => RestoreKind::RefusalStored,
                    (Disposition::Committed, Action::Mute { .. }) => RestoreKind::Replayed,
                    (Disposition::Committed, _) => RestoreKind::NotReapplied,
                }
            };
            planned.push((
                candidate.link.sequence,
                Planned::Request {
                    request: request.clone(),
                    outcome,
                    bytes,
                    kind,
                },
            ));
        }
        if !ambiguous.is_empty() {
            return Err(RestoreError::Ambiguous(ambiguous));
        }
        let applied = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result =
                        restore_in_transaction(&mut tx, &trust, contract, &head, planned).await;
                    if result.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await?;
        Ok(applied?)
    }
}

/// The apply half of the restore replay, in one transaction under the head
/// lock: the head must still be the one verified, and the retained links are
/// written in chain order through the same participant as live requests.
async fn restore_in_transaction(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    contract: Contract,
    verified: &super::staff_action::Head,
    planned: Vec<(u64, Planned)>,
) -> Result<Vec<RestoreStep>, StaffError> {
    let environment = trust.environment();
    if super::staff_action::lock_head(connection, environment).await? != *verified {
        return Err(StaffError::Conflict);
    }
    if super::staff_action::evidence_orphans_in_transaction(
        connection,
        environment,
        verified.sequence,
    )
    .await?
    {
        return Err(Error::Chain.into());
    }
    let mut steps = Vec::with_capacity(planned.len());
    for (sequence, step) in planned {
        match step {
            Planned::Receipt(receipt) => {
                let operation_id = receipt.receipt().operation_id.clone();
                super::staff_action::receipt_in_transaction(connection, trust, &receipt).await?;
                steps.push(RestoreStep {
                    sequence,
                    operation_id,
                    kind: RestoreKind::Receipt,
                });
            }
            Planned::Request {
                request,
                outcome,
                bytes,
                mut kind,
            } => {
                let operation_id = request.request().operation_id.clone();
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_staff_operations \
                     WHERE environment=$1 AND operation_id=$2::uuid)",
                )
                .bind(environment)
                .bind(&operation_id)
                .fetch_one(&mut *connection)
                .await?;
                if exists {
                    return Err(StaffError::OperationConflict);
                }
                let occurred_at = i64::try_from(outcome.occurred_at.0).map_err(|_| Error::Clock)?;
                if kind == RestoreKind::Replayed {
                    kind = replay_mute(connection, trust, &request, &outcome, occurred_at).await?;
                }
                let pending =
                    super::staff_action::restored_pending(connection, request, occurred_at).await?;
                let stored = finish_in_transaction(
                    connection,
                    pending,
                    contract,
                    outcome,
                    super::character_authority::SERVER_BUILD_ID,
                )
                .await?;
                if stored.bytes != bytes {
                    return Err(Error::Binding.into());
                }
                steps.push(RestoreStep {
                    sequence,
                    operation_id,
                    kind,
                });
            }
        }
    }
    Ok(steps)
}

/// A COMMITTED mute re-run through its request's own checks at its recorded
/// time; a check the current state fails applies nothing.
async fn replay_mute(
    connection: &mut PgConnection,
    trust: &VerifierTrust,
    request: &VerifiedRequest,
    outcome: &Outcome,
    occurred_at: i64,
) -> Result<RestoreKind, StaffError> {
    let Action::Mute {
        character_id,
        duration_ms,
    } = &request.request().action
    else {
        return Err(Error::Invalid.into());
    };
    let principal = &request.signed().principal;
    if !trust
        .role_of(principal)?
        .allows(request.request().action.kind())
    {
        return Ok(RestoreKind::RefusedNow(ResultCode::TierInsufficient));
    }
    if duration_ms.0 > MAX_MUTE_MS {
        return Err(Error::Invalid.into());
    }
    let world = request
        .request()
        .scope
        .as_ref()
        .and_then(|scope| crate::node::uuid_bytes(&scope.world_id))
        .ok_or(Error::Invalid)?;
    if character_in_world(connection, character_id, &world)
        .await?
        .is_none()
    {
        return Ok(RestoreKind::RefusedNow(ResultCode::TargetNotFound));
    }
    let duration = i64::try_from(duration_ms.0).map_err(|_| Error::Invalid)?;
    let ends_at = occurred_at.checked_add(duration).ok_or(Error::Invalid)?;
    sqlx::query(
        "INSERT INTO game_character_sanctions(sanction_id,environment,character_id,kind,\
         starts_at,ends_at,actor_principal,mute_action_id,status,revision) \
         VALUES($1::uuid,$2,$3::uuid,'MUTE',$4,$5,$6,$7::uuid,'ACTIVE',1)",
    )
    .bind(new_id()?)
    .bind(trust.environment())
    .bind(character_id)
    .bind(occurred_at)
    .bind(ends_at)
    .bind(principal)
    .bind(&outcome.action_id)
    .execute(&mut *connection)
    .await?;
    Ok(RestoreKind::Replayed)
}
