//! T94 Game authority: one semantic transaction owns interpretation, revision,
//! immutable audit and outbox. Source hooks run inside their native transactions.
mod audit;
pub use audit::{GuildSeasonStandingsDecidedV1, WorldEventOutcomeDecidedV1};
mod defaults;
mod delivery;
mod native;
mod recovery;
mod season;
mod visibility;
pub use delivery::ProducerStore;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
pub(super) use native::{capture_boss_completion, capture_raid, capture_reset, capture_window};
use oteryn_gameplay_native_admission_source::world_event_outcomes::{
    self as wire, EventRecord, EventSnapshot, SeasonRecord, Snapshot,
};
pub use season::Contribution;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, DurabilityError>;
fn invalid<T>(_: T) -> DurabilityError {
    DurabilityError::InvalidStoredState
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Pending,
    Applied,
    Replayed,
    Conflict,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChannelPolicy {
    #[default]
    UnanimousOrPending,
    AllTargetsOrVoid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventBinding {
    #[serde(default)]
    pub channel_policy: ChannelPolicy,
    pub occurrence_id: String,
    pub question_key: String,
    #[serde(default)]
    pub withdraw_display: bool,
    pub question: EventRecord,
    /// Empty by default. Every admitted target Channel must be represented.
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub advance_approved: bool,
    /// Native phase -> approved option. No timeout-to-killed inference exists.
    #[serde(default)]
    pub outcomes: BTreeMap<String, String>,
    #[serde(default)]
    pub void_reason: Option<String>,
    #[serde(default)]
    pub corrections_closed: bool,
    #[serde(default)]
    pub correction_deadline: Option<u64>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Attribution {
    #[default]
    PrincipalOnly,
    QualifiedPartyRoster,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonBinding {
    #[serde(default)]
    pub attribution: Attribution,
    pub season: SeasonRecord,
    #[serde(default)]
    pub withdraw_display: bool,
    #[serde(default)]
    pub rules: BTreeMap<String, u64>,
    #[serde(default)]
    pub eligible_guilds: Vec<String>,
    #[serde(default)]
    pub tombstone_name: Option<String>,
    #[serde(default)]
    pub corrections_closed: bool,
    #[serde(default)]
    pub correction_deadline: Option<u64>,
    #[serde(default)]
    pub complete_through: u64,
}
struct Control {
    started: u64,
    epoch: u64,
    generation: u64,
    authority: String,
    cut: u64,
    resync: bool,
}
async fn control(tx: &mut Transaction<'_, Postgres>, generation: u64) -> Result<Option<Control>> {
    let row=sqlx::query("SELECT enabled,reconciled,quarantined,projection_epoch::text,recovery_generation::text,source_authority,complete_through::text,resync_complete,source_started_at FROM game_t94_control WHERE singleton FOR UPDATE").fetch_one(&mut **tx).await?;
    if row.try_get::<bool, _>("quarantined")? {
        return Err(DurabilityError::Unavailable);
    }
    if !row.try_get::<bool, _>("enabled")? || !row.try_get::<bool, _>("reconciled")? {
        return Ok(None);
    }
    let stored =
        wire::decimal(&row.try_get::<String, _>("recovery_generation")?, false).map_err(invalid)?;
    if stored != generation {
        return Err(DurabilityError::Unavailable);
    }
    Ok(Some(Control {
        started: u64::try_from(row.try_get::<i64, _>("source_started_at")?).map_err(invalid)?,
        epoch: wire::decimal(&row.try_get::<String, _>("projection_epoch")?, false)
            .map_err(invalid)?,
        generation,
        authority: row.try_get("source_authority")?,
        cut: wire::decimal(&row.try_get::<String, _>("complete_through")?, true)
            .map_err(invalid)?,
        resync: row.try_get("resync_complete")?,
    }))
}
fn fields(s: &Snapshot) -> (&str, &str, &str, &str, &str, &str) {
    match s {
        Snapshot::Event(s) => (
            "world_event_outcome",
            &s.world_id,
            &s.event_id,
            &s.projection_epoch,
            &s.projection_revision,
            &s.source_observed_at,
        ),
        Snapshot::Season(s) => (
            "guild_season",
            &s.world_id,
            &s.season_id,
            &s.projection_epoch,
            &s.projection_revision,
            &s.source_observed_at,
        ),
    }
}
async fn previous(
    tx: &mut Transaction<'_, Postgres>,
    family: &str,
    world: &str,
    subject: &str,
) -> Result<Option<Snapshot>> {
    let raw:Option<Vec<u8>>=sqlx::query_scalar("SELECT p.wire FROM game_t94_subjects s JOIN game_t94_publications p USING(family,world_id,subject_id) WHERE s.family=$1 AND s.world_id=$2::uuid AND s.subject_id=$3::uuid ORDER BY p.projection_epoch DESC,p.projection_revision DESC LIMIT 1 FOR UPDATE OF s")
        .bind(family).bind(world).bind(subject).fetch_optional(&mut **tx).await?;
    raw.map(|v| wire::Snapshot::decode(std::str::from_utf8(&v).map_err(invalid)?).map_err(invalid))
        .transpose()
}
/// Persist only after native/domain validation. No external caller can bypass it.
async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    s: &Snapshot,
    old: Option<&Snapshot>,
    question: &str,
    c: &Control,
    now: i64,
    display: Option<bool>,
) -> Result<Disposition> {
    let bytes = s.encode().map_err(invalid)?;
    let (family, world, subject, epoch, revision, observed) = fields(s);
    if wire::decimal(epoch, false).map_err(invalid)? != c.epoch {
        return Ok(Disposition::Conflict);
    }
    if old.is_some_and(|old| s == old) {
        return Ok(Disposition::Replayed);
    }
    if display.is_none() {
        s.follows(old, u64::try_from(now).map_err(invalid)?)
            .map_err(invalid)?;
    } else {
        let old = old.ok_or(DurabilityError::InvalidStoredState)?;
        let (of, ow, os, oe, or, oo) = fields(old);
        if (family, world, subject, epoch) != (of, ow, os, oe)
            || wire::decimal(or, false).map_err(invalid)?.checked_add(1)
                != Some(wire::decimal(revision, false).map_err(invalid)?)
            || wire::decimal(observed, false).map_err(invalid)?
                < wire::decimal(oo, false).map_err(invalid)?
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let body = match s {
        Snapshot::Event(s) => s.event.first().map(|e| {
            (
                serde_json::to_vec(e),
                Some(e.occurrence_id.clone()),
                e.state == "CLOSED",
            )
        }),
        Snapshot::Season(s) => s
            .season
            .first()
            .map(|e| (serde_json::to_vec(e), None, e.state != "OPEN")),
    };
    let (record, occurrence, final_state) = if let Some((record, occurrence, final_state)) = body {
        (record.map_err(invalid)?, occurrence, final_state)
    } else {
        let stored:(Vec<u8>,Option<String>,bool)=sqlx::query_as("SELECT record,occurrence_id::text,final_state FROM game_t94_subjects WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid FOR UPDATE")
            .bind(family).bind(world).bind(subject).fetch_one(&mut **tx).await?;
        if stored.0.is_empty() {
            return Err(DurabilityError::Unavailable);
        }
        stored
    };
    if display.is_some() {
        let expected:Vec<u8>=sqlx::query_scalar("SELECT record_sha256 FROM game_t94_subjects WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid").bind(family).bind(world).bind(subject).fetch_one(&mut **tx).await?;
        if expected != Sha256::digest(&record).as_slice() {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    if display.is_none()
        && let Snapshot::Event(snapshot) = s
        && let Some(event) = snapshot.event.first()
        && matches!(event.state.as_str(), "SETTLED" | "VOID" | "CLOSED")
    {
        let evidence = event
            .evidence_sha256
            .first()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_evidence WHERE evidence_sha256=$1 AND world_id=$2::uuid AND event_id=$3::uuid AND sha256(evidence)=evidence_sha256 AND octet_length(evidence)>0)").bind(audit::hex(evidence)?).bind(world).bind(subject).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(DurabilityError::Unavailable);
        }
    }
    let duplicate:Option<Vec<u8>>=sqlx::query_scalar("SELECT wire FROM game_t94_publications WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid AND projection_epoch=$4::text::numeric AND projection_revision=$5::text::numeric")
        .bind(family).bind(world).bind(subject).bind(epoch).bind(revision).fetch_optional(&mut **tx).await?;
    if let Some(stored) = duplicate {
        if stored == bytes.as_bytes() {
            return Ok(Disposition::Replayed);
        }
        sqlx::query("UPDATE game_t94_control SET quarantined=true WHERE singleton")
            .execute(&mut **tx)
            .await?;
        eprintln!("family=t94 operation=interpret result=PLATFORM_CONFLICT timing_ms=0");
        return Ok(Disposition::Conflict);
    }
    // Separate subject authority survives serving/audit expiry. Never reuse a native question.
    sqlx::query("INSERT INTO game_t94_subjects(family,world_id,subject_id,occurrence_id,question_key,projection_revision,record,record_sha256,final_state,final_at) VALUES($1,$2::uuid,$3::uuid,$4::uuid,$5,$6::text::numeric,$7,$8,$9,CASE WHEN $9 THEN $10 ELSE NULL END) ON CONFLICT(family,world_id,subject_id) DO UPDATE SET projection_revision=excluded.projection_revision,record=excluded.record,record_sha256=excluded.record_sha256,final_state=excluded.final_state,final_at=CASE WHEN excluded.final_state THEN coalesce(game_t94_subjects.final_at,excluded.final_at) ELSE NULL END")
        .bind(family).bind(world).bind(subject).bind(occurrence).bind(question).bind(revision).bind(&record).bind(Sha256::digest(&record).as_slice()).bind(final_state).bind(now).execute(&mut **tx).await?;
    let (event, transaction): (String, String) =
        sqlx::query_as("SELECT uuidv7()::text,uuidv7()::text")
            .fetch_one(&mut **tx)
            .await?;
    let audit = if display.is_some() {
        Vec::new()
    } else {
        audit::encode(
            s,
            &record,
            &event,
            &transaction,
            c.generation,
            now.checked_mul(1000)
                .ok_or(DurabilityError::InvalidStoredState)?,
        )?
    };
    sqlx::query("INSERT INTO game_t94_publications(family,world_id,subject_id,projection_epoch,projection_revision,source_observed_at,wire,wire_sha256,audit_event_id,transaction_id,occurred_at_ms,audit) VALUES($1,$2::uuid,$3::uuid,$4::text::numeric,$5::text::numeric,$6,$7,$8,$9::uuid,$10::uuid,$11,$12)")
        .bind(family).bind(world).bind(subject).bind(epoch).bind(revision).bind(observed.parse::<i64>().map_err(invalid)?).bind(bytes.as_bytes()).bind(Sha256::digest(bytes.as_bytes()).as_slice()).bind(event).bind(transaction).bind(now.checked_mul(1000).ok_or(DurabilityError::InvalidStoredState)?).bind(audit).execute(&mut **tx).await?;
    if let Some(withdrawn) = display {
        sqlx::query("UPDATE game_t94_subjects SET display_withdrawn=$4 WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid").bind(family).bind(world).bind(subject).bind(withdrawn).execute(&mut **tx).await?;
    }
    if let Snapshot::Season(s) = s
        && let Some(season) = s.season.first()
    {
        for guild in &season.standings {
            sqlx::query("INSERT INTO game_t94_privacy_watch(guild_id,world_id,season_id,public_name) VALUES($1::uuid,$2::uuid,$3::uuid,$4) ON CONFLICT(guild_id,world_id,season_id) DO UPDATE SET public_name=excluded.public_name").bind(&guild.guild_id).bind(world).bind(subject).bind(&guild.guild_name).execute(&mut **tx).await?;
        }
    }
    Ok(Disposition::Applied)
}
fn revision(old: Option<&Snapshot>) -> Result<String> {
    match old {
        None => Ok("1".into()),
        Some(s) => Ok(wire::decimal(fields(s).4, false)
            .map_err(invalid)?
            .checked_add(1)
            .ok_or(DurabilityError::InvalidStoredState)?
            .to_string()),
    }
}

impl DurabilityRoot {
    /// One bounded source-interpretation pass. An unapproved or missing rule is PENDING.
    pub async fn t94_produce(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<Disposition> {
        let recovery = authority.record_for(self).map_err(invalid)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let Some(c)=control(&mut tx,recovery.recovery_generation).await? else{return Ok(Disposition::Pending)};
            if !c.resync {
                let result=recovery::resync_page(&mut tx,&c).await?;
                commit_semantic_transaction(tx,deadline).await?;
                return Ok(result);
            }
            if season::correct_one(&mut tx,&c).await? {
                commit_semantic_transaction(tx,deadline).await?;
                return Ok(Disposition::Applied);
            }
            let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let row=sqlx::query("SELECT family,world_id::text,subject_id::text,binding,floor(extract(epoch FROM approved_at))::bigint AS approved_at FROM game_t94_bindings WHERE approved AND NOT EXISTS(SELECT 1 FROM game_t94_subjects s WHERE s.family=game_t94_bindings.family AND s.world_id=game_t94_bindings.world_id AND s.subject_id=game_t94_bindings.subject_id AND s.final_state AND s.display_withdrawn=coalesce((convert_from(game_t94_bindings.binding,'UTF8')::jsonb->>'withdraw_display')::boolean,false) AND NOT EXISTS(SELECT 1 FROM game_t94_privacy_watch w LEFT JOIN game_guilds g ON g.guild_id=w.guild_id AND g.world_id=w.world_id WHERE w.world_id=s.world_id AND w.season_id=s.subject_id AND (g.guild_id IS NULL OR g.state=4) AND coalesce(convert_from(game_t94_bindings.binding,'UTF8')::jsonb->>'tombstone_name','')<>w.public_name)) ORDER BY checked_at,family,world_id,subject_id LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut *tx).await?;
            let Some(row)=row else{commit_semantic_transaction(tx,deadline).await?;return Ok(Disposition::Pending)};
            let family:String=row.try_get("family")?;let world:String=row.try_get("world_id")?;let subject:String=row.try_get("subject_id")?;
            let binding:Vec<u8>=row.try_get("binding")?;
            let old=previous(&mut tx,&family,&world,&subject).await?;
            let value:serde_json::Value=serde_json::from_slice(&binding).map_err(invalid)?;
            let withdraw=value.get("withdraw_display").and_then(serde_json::Value::as_bool).unwrap_or(false);
            let tombstone=value.get("tombstone_name").and_then(serde_json::Value::as_str);
            if let Some(result)=visibility::apply(&mut tx,&c,&family,&world,&subject,withdraw,tombstone,&binding,old.as_ref(),now).await? {
                sqlx::query("UPDATE game_t94_bindings SET checked_at=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid").bind(&family).bind(&world).bind(&subject).execute(&mut *tx).await?;
                commit_semantic_transaction(tx,deadline).await?;return Ok(result);
            }
            let result=if family=="world_event_outcome"{
                native::produce_event(&mut tx,&c,&world,&subject,&serde_json::from_slice::<EventBinding>(&binding).map_err(invalid)?,old.as_ref(),now).await?
            }else{
                let rule:SeasonBinding=serde_json::from_slice(&binding).map_err(invalid)?;
                if !rule.rules.is_empty() && row.try_get::<i64,_>("approved_at")? as u64>=wire::decimal(&rule.season.starts_at,false).map_err(invalid)? {Disposition::Pending}
                else{season::produce(&mut tx,&c,&world,&subject,&rule,old.as_ref(),now).await?}
            };
            sqlx::query("UPDATE game_t94_bindings SET checked_at=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid")
                .bind(&family).bind(&world).bind(&subject).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }
}

async fn source_start(tx: &mut Transaction<'_, Postgres>) -> Result<()> {
    sqlx::query("SAVEPOINT t94_native_source")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn source_finish(
    tx: &mut Transaction<'_, Postgres>,
    result: Result<Disposition>,
) -> Result<Disposition> {
    let disposition = match result {
        Ok(d) => d,
        Err(_) => {
            sqlx::query("ROLLBACK TO SAVEPOINT t94_native_source")
                .execute(&mut **tx)
                .await?;
            sqlx::query("UPDATE game_t94_control SET quarantined=true WHERE singleton")
                .execute(&mut **tx)
                .await?;
            eprintln!("family=t94 operation=capture result=PLATFORM_UNAVAILABLE timing_ms=0");
            Disposition::Pending
        }
    };
    sqlx::query("RELEASE SAVEPOINT t94_native_source")
        .execute(&mut **tx)
        .await?;
    Ok(disposition)
}
pub(crate) async fn capture_kill<const N: usize>(
    tx: &mut Transaction<'_, Postgres>,
    input: &crate::durability::party_kill_xp::PartyKillXpRequest<N>,
) -> Result<Disposition> {
    source_start(tx).await?;
    let result = season::capture_kill(tx, input).await;
    source_finish(tx, result).await
}
