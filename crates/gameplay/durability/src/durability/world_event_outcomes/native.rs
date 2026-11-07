use super::*;
use crate::durability::raid_runner::RaidStepIdentity;
use crate::foundation::WorldId;

pub(crate) async fn enabled(tx: &mut Transaction<'_, Postgres>) -> Result<bool> {
    let present: bool = sqlx::query_scalar("SELECT to_regclass('game_t94_control') IS NOT NULL")
        .fetch_one(&mut **tx)
        .await?;
    if !present {
        return Ok(false);
    }
    sqlx::query_scalar("SELECT enabled AND reconciled AND NOT quarantined FROM game_t94_control WHERE singleton FOR SHARE").fetch_one(&mut **tx).await.map_err(Into::into)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    world: String,
    kind: String,
    occurrence: String,
    key: String,
    channel: String,
    phase: String,
    cut: String,
    generation: u64,
    proof: String,
    #[serde(default)]
    content_sha256: String,
    native_at: i64,
}
async fn capture(
    tx: &mut Transaction<'_, Postgres>,
    mut f: Fact,
    observed: i64,
) -> Result<Disposition> {
    let row=sqlx::query("SELECT c.enabled AND c.reconciled AS ready,c.recovery_generation::text AS generation,(SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1) AS current FROM game_t94_control c WHERE singleton FOR SHARE").fetch_one(&mut **tx).await?;
    if !row.try_get::<bool, _>("ready")? {
        return Ok(Disposition::Pending);
    }
    let generation: String = row.try_get("generation")?;
    if row.try_get::<Option<String>, _>("current")?.as_deref() != Some(&generation) {
        return Ok(Disposition::Pending);
    }
    if observed <= 0 {
        return Ok(Disposition::Pending);
    }
    f.generation = wire::decimal(&generation, false).map_err(invalid)?;
    f.native_at = observed;
    let observed: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let raw = serde_json::to_vec(&f).map_err(invalid)?;
    let hash = Sha256::digest(&raw).to_vec();
    let old:Option<Vec<u8>>=sqlx::query_scalar("SELECT receipt_sha256 FROM game_t94_native_receipts WHERE world_id=$1::uuid AND kind=$2 AND source_key=$3 AND source_cut=$4")
        .bind(&f.world).bind(&f.kind).bind(&f.key).bind(&f.cut).fetch_optional(&mut **tx).await?;
    if let Some(old) = old {
        return if old == hash {
            Ok(Disposition::Replayed)
        } else {
            Err(DurabilityError::InvalidStoredState)
        };
    }
    sqlx::query("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,$2,$3::uuid,$4,$5,$6::text::numeric,$7,$8,$9)")
        .bind(f.world).bind(f.kind).bind(f.occurrence).bind(f.key).bind(f.cut).bind(generation).bind(raw).bind(hash).bind(observed).execute(&mut **tx).await?;
    Ok(Disposition::Applied)
}
pub(crate) fn uuid_text(bytes: &[u8]) -> Result<String> {
    if bytes.len() != 16 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}
async fn capture_raid_inner(
    tx: &mut Transaction<'_, Postgres>,
    id: RaidStepIdentity,
    phase: &str,
    now_ms: i64,
) -> Result<Disposition> {
    if !enabled(tx).await? {
        return Ok(Disposition::Pending);
    }
    let (world,key,content):(String,String,String)=sqlx::query_as("SELECT world_id::text,raid_key,encode(bundle_digest,'hex') FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid")
        .bind(id.firing_id.as_slice()).fetch_one(&mut **tx).await?;
    let channel = uuid_text(id.channel.as_bytes())?;
    if phase == "completed" {
        let cancelled:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_native_receipts WHERE world_id=$1::uuid AND kind='RAID' AND occurrence_id=$2::uuid AND source_cut=$3 AND recovery_generation=(SELECT recovery_generation FROM game_t94_control WHERE singleton))")
            .bind(&world).bind(uuid_text(&id.firing_id)?).bind(format!("{channel}/{}/{}/cancelled",id.generation.get(),id.ordinal)).fetch_one(&mut **tx).await?;
        if cancelled {
            return Ok(Disposition::Replayed);
        }
    }
    let native_phase = if phase == "boss-admitted" {
        "admitted"
    } else if phase == "boss-source-unavailable" {
        "source-unavailable"
    } else {
        phase
    };
    let proof =
        serde_json::to_vec(&(id.ordinal, id.generation.get(), phase, now_ms)).map_err(invalid)?;
    capture(
        tx,
        Fact {
            world,
            kind: if phase.starts_with("boss-") {
                "BOSS"
            } else {
                "RAID"
            }
            .into(),
            occurrence: uuid_text(&id.firing_id)?,
            key,
            channel: channel.clone(),
            phase: native_phase.into(),
            cut: format!("{channel}/{}/{}/{phase}", id.generation.get(), id.ordinal),
            generation: 0,
            proof: wire::digest(&proof),
            content_sha256: content,
            native_at: 0,
        },
        now_ms / 1000,
    )
    .await
}
async fn capture_reset_inner(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Disposition> {
    if !enabled(tx).await? {
        return Ok(Disposition::Pending);
    }
    let world = uuid_text(world.as_bytes())?;
    let cut = format!("reset/{epoch}");
    let existing:Option<String>=sqlx::query_scalar("SELECT occurrence_id::text FROM game_t94_native_receipts WHERE world_id=$1::uuid AND kind='WORLD_CHANGE' AND source_key='world:gathering' AND source_cut=$2")
        .bind(&world).bind(&cut).fetch_optional(&mut **tx).await?;
    let occurrence = match existing {
        Some(id) => id,
        None => {
            sqlx::query_scalar("SELECT uuidv7()::text")
                .fetch_one(&mut **tx)
                .await?
        }
    };
    let (now,content):(i64,String)=sqlx::query_as("SELECT floor(extract(epoch FROM activated_at))::bigint,encode(target_digest,'hex') FROM game_world_reset_epochs WHERE world_id=$1::uuid AND reset_epoch=$2::text::numeric")
        .bind(&world).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    // The alternating public gathering is already derivable. It can never
    // satisfy advance_approved, and this is not a Fury Gates realization.
    capture(
        tx,
        Fact {
            world: world.clone(),
            kind: "WORLD_CHANGE".into(),
            occurrence,
            key: "world:gathering".into(),
            channel: String::new(),
            phase: "known-reset".into(),
            cut,
            generation: 0,
            proof: wire::digest(&serde_json::to_vec(&(&world, epoch, &content)).map_err(invalid)?),
            content_sha256: content,
            native_at: 0,
        },
        now,
    )
    .await
}

pub(super) async fn produce_event(
    tx: &mut Transaction<'_, Postgres>,
    c: &Control,
    world: &str,
    event: &str,
    b: &EventBinding,
    old: Option<&Snapshot>,
    now: i64,
) -> Result<Disposition> {
    let mut e = b.question.clone();
    if e.occurrence_id != b.occurrence_id || e.state != "SCHEDULED" || e.reason_code != "NONE" {
        return Err(DurabilityError::InvalidStoredState);
    }
    let correction_cut = if old
        .and_then(private_record)
        .is_some_and(|e| matches!(e.state.as_str(), "SETTLED" | "VOID"))
        && b.corrections_closed
    {
        b.correction_deadline
            .map(i64::try_from)
            .transpose()
            .map_err(invalid)?
    } else {
        None
    };
    let rows=sqlx::query("SELECT receipt,receipt_sha256,observed_at,expired_at FROM game_t94_native_receipts WHERE world_id=$1::uuid AND occurrence_id=$2::uuid AND source_key=$3 AND recovery_generation=$4::text::numeric AND kind IN ('RAID','BOSS','WORLD_CHANGE','DAILY_WORLD_GATE') AND ($5::bigint IS NULL OR observed_at<=$5) ORDER BY delivery_order LIMIT 1025")
        .bind(world).bind(&b.occurrence_id).bind(&e.source_key).bind(c.generation.to_string()).bind(correction_cut).fetch_all(&mut **tx).await?;
    if rows.is_empty() || rows.len() > 1024 {
        return Ok(Disposition::Pending);
    }
    if rows.iter().any(|r| {
        r.try_get::<Vec<u8>, _>("receipt")
            .is_ok_and(|raw| raw.is_empty())
    }) {
        return expired(tx, c, world, event, b, old, &rows, now).await;
    }
    let mut facts = BTreeMap::new();
    let mut admission = BTreeMap::new();
    for row in rows {
        let raw: Vec<u8> = row.try_get("receipt")?;
        let hash: Vec<u8> = row.try_get("receipt_sha256")?;
        if hash != Sha256::digest(&raw).as_slice() {
            return Err(DurabilityError::InvalidStoredState);
        }
        let f: Fact = serde_json::from_slice(&raw).map_err(invalid)?;
        if f.content_sha256 != e.content_sha256 {
            return Ok(Disposition::Pending);
        }
        if f.generation != c.generation
            || f.world != world
            || f.occurrence != b.occurrence_id
            || f.key != e.source_key
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if f.native_at <= 0 {
            return Ok(Disposition::Pending);
        }
        let at = f.native_at;
        let item = (f.phase.clone(), wire::digest(&raw), at);
        if f.phase == "admitted" {
            admission.insert(f.channel.clone(), item.clone());
        }
        if f.kind == e.kind
            || (e.kind == "BOSS"
                && f.kind == "RAID"
                && matches!(f.phase.as_str(), "source-unavailable" | "cancelled"))
        {
            facts.insert(f.channel, item);
        }
    }
    if matches!(e.kind.as_str(), "RAID" | "BOSS") {
        let channels:Vec<String>=sqlx::query_scalar("SELECT channel_id::text FROM game_world_raid_targets WHERE firing_id=$1::uuid ORDER BY channel_id LIMIT 1025").bind(&b.occurrence_id).fetch_all(&mut **tx).await?;
        if channels.is_empty() || channels.len() > 1024 || channels != b.channels {
            return Ok(Disposition::Pending);
        }
    } else if !b.channels.is_empty() {
        return Ok(Disposition::Pending);
    }
    if facts.values().any(|(phase, _, _)| {
        !matches!(
            phase.as_str(),
            "admitted"
                | "completed"
                | "boss-dead"
                | "cancelled"
                | "source-unavailable"
                | "known-reset"
                | "invalid-source"
        )
    }) {
        return Ok(Disposition::Pending);
    }
    sqlx::query("UPDATE game_t94_bindings SET native_seen_order=(SELECT coalesce(max(delivery_order),0) FROM game_t94_native_receipts WHERE world_id=$1::uuid AND occurrence_id=$3::uuid AND kind IN('RAID','BOSS','DAILY_WORLD_GATE','WORLD_CHANGE')) WHERE family='world_event_outcome' AND world_id=$1::uuid AND subject_id=$2::uuid")
        .bind(world).bind(event).bind(&b.occurrence_id).execute(&mut **tx).await?;
    let channels = if !b.advance_approved && b.void_reason.as_deref() == Some("INVALID_SOURCE") {
        facts.keys().cloned().collect::<Vec<_>>()
    } else if b.channels.is_empty() {
        vec![String::new()]
    } else {
        b.channels.clone()
    };
    if channels.is_empty() {
        return Ok(Disposition::Pending);
    }
    let can_schedule = b.advance_approved
        && e.kind != "WORLD_CHANGE"
        && channels.iter().all(|ch| admission.contains_key(ch))
        && facts.values().all(|(phase, _, _)| phase == "admitted")
        && (now as u64) < wire::decimal(&e.lock_at, false).map_err(invalid)?;
    if old.is_none() && can_schedule && matches!(e.kind.as_str(), "RAID" | "BOSS") {
        let unknown:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_firings f WHERE f.firing_id=$1::uuid AND f.state=1 AND f.deadline_ms::numeric>$2::text::numeric*1000 AND NOT EXISTS(SELECT 1 FROM game_world_raid_creatures k JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index) WHERE k.firing_id=f.firing_id AND k.state=2 AND p.placed))")
            .bind(&b.occurrence_id).bind(&e.lock_at).fetch_one(&mut **tx).await?;
        if !unknown {
            return Ok(Disposition::Pending);
        }
        let windows=sqlx::query("SELECT w.channel_id::text,w.first_wave_at_ms,w.deadline_ms FROM game_t94_raid_windows w JOIN game_runtime_scope_assignments a ON a.channel_id=w.channel_id AND a.world_id=$2::uuid AND a.scope_kind=1 AND a.state=1 AND a.ownership_generation=w.owner_generation JOIN game_durability_admission_runtime_guards g USING(scope_key) JOIN game_node_registrations n ON n.node_id=a.holder_node_id AND n.registration_revision=a.holder_registration_revision AND n.state=1 WHERE w.firing_id=$1::uuid AND g.ready AND g.ownership_generation=w.owner_generation AND w.recovery_generation=$3::text::numeric ORDER BY w.channel_id LIMIT 1025")
            .bind(&b.occurrence_id).bind(world).bind(c.generation.to_string()).fetch_all(&mut **tx).await?;
        let mut covered = Vec::new();
        let mut first = i64::MAX;
        let mut end = 0i64;
        for row in windows {
            covered.push(row.try_get::<String, _>("channel_id")?);
            first = first.min(row.try_get("first_wave_at_ms")?);
            end = end.max(row.try_get("deadline_ms")?);
        }
        if covered != b.channels
            || first < 0
            || end <= first
            || wire::decimal(&e.lock_at, false).map_err(invalid)? as u128 * 1000 > first as u128
            || wire::decimal(&e.scheduled_start, false).map_err(invalid)? != first as u64 / 1000
            || wire::decimal(&e.scheduled_end, false).map_err(invalid)? != end as u64 / 1000
        {
            return Ok(Disposition::Pending);
        }
    }
    let prior = match old {
        Some(Snapshot::Event(s)) => s.event.first(),
        Some(_) => return Err(DurabilityError::InvalidStoredState),
        None => None,
    };
    let mut proof = None;
    if let Some(previous) = prior {
        if previous.state == "CLOSED" {
            return Ok(Disposition::Pending);
        }
        if previous.state == "SCHEDULED"
            && wire::decimal(&previous.lock_at, false).map_err(invalid)? <= now as u64
        {
            e = previous.clone();
            e.state = "LOCKED".into();
        } else {
            let Some(mut decision) =
                terminal(b, e, prior, &channels, &facts, c, world, event, now)?
            else {
                return Ok(Disposition::Pending);
            };
            if decision.record.state == "SETTLED"
                && wire::decimal(&decision.record.decided_at[0], false).map_err(invalid)?
                    < wire::decimal(&decision.record.lock_at, false).map_err(invalid)?
            {
                invalidate(&mut decision, c.generation)?;
            }
            if decision.record == *previous
                && b.corrections_closed
                && b.correction_deadline
                    .is_some_and(|at| at <= now as u64 && at <= c.cut)
            {
                decision.record.state = "CLOSED".into();
                decision.record.reason_code = "FINAL_FREEZE".into();
            }
            e = decision.record;
            proof = Some(decision.proof);
        }
    } else if can_schedule {
        // The native run is admitted and no result is yet known.
    } else {
        let Some(mut decision) = terminal(b, e, None, &channels, &facts, c, world, event, now)?
        else {
            return Ok(Disposition::Pending);
        };
        if decision.record.state == "SETTLED" {
            invalidate(&mut decision, c.generation)?;
        }
        e = decision.record;
        proof = Some(decision.proof);
    }
    if e.state == "SCHEDULED" && prior.is_some() {
        return Ok(Disposition::Pending);
    }
    if prior.is_some_and(|p| *p == e) {
        return Ok(Disposition::Pending);
    }
    if let Some(proof) = proof {
        if proof.len() > 131072 {
            return Err(DurabilityError::InvalidStoredState);
        }
        sqlx::query("INSERT INTO game_t94_evidence(evidence_sha256,world_id,event_id,recovery_generation,evidence) VALUES(sha256($1),$2::uuid,$3::uuid,$4::text::numeric,$1) ON CONFLICT DO NOTHING")
            .bind(proof).bind(world).bind(event).bind(c.generation.to_string()).execute(&mut **tx).await?;
    }
    let s = Snapshot::Event(EventSnapshot {
        contract_version: 1,
        operation: "PublishWorldEventOutcomeV1".into(),
        source_authority: c.authority.clone(),
        world_id: world.into(),
        event_id: event.into(),
        projection_epoch: c.epoch.to_string(),
        projection_revision: revision(old)?,
        source_observed_at: now.to_string(),
        event: vec![e],
    });
    persist(tx, &s, old, &b.question_key, c, now, None).await
}
struct Decision {
    record: EventRecord,
    proof: Vec<u8>,
}
fn invalidate(d: &mut Decision, generation: u64) -> Result<()> {
    let source: serde_json::Value = serde_json::from_slice(&d.proof).map_err(invalid)?;
    d.record.state = "VOID".into();
    d.record.outcome.clear();
    d.record.reason_code = "INVALID_SOURCE".into();
    d.record.evidence_sha256.clear();
    d.proof =
        serde_json::to_vec(&("invalid-source", &d.record, source, generation)).map_err(invalid)?;
    d.record.evidence_sha256 = vec![wire::digest(&d.proof)];
    Ok(())
}
#[allow(clippy::too_many_arguments)] // Explicit source, subject and fence coordinates.
fn terminal(
    b: &EventBinding,
    mut e: EventRecord,
    prior: Option<&EventRecord>,
    channels: &[String],
    facts: &BTreeMap<String, (String, String, i64)>,
    c: &Control,
    world: &str,
    event: &str,
    now: i64,
) -> Result<Option<Decision>> {
    let mut outcome = None;
    let mut hashes = Vec::new();
    let mut decided = 0;
    let mut void_reason: Option<&str> = None;
    for ch in channels {
        let Some((phase, hash, at)) = facts.get(ch) else {
            return Ok(None);
        };
        let native_void =
            if !b.advance_approved && b.void_reason.as_deref() == Some("INVALID_SOURCE") {
                Some("INVALID_SOURCE")
            } else {
                match phase.as_str() {
                    "cancelled" => Some("CANCELLED"),
                    "source-unavailable" => Some("SOURCE_UNAVAILABLE"),
                    "known-reset" | "invalid-source" => Some("INVALID_SOURCE"),
                    _ => None,
                }
            };
        if let Some(reason) = native_void {
            if outcome.is_some() || void_reason.is_some_and(|old| old != reason) {
                if b.channel_policy == ChannelPolicy::UnanimousOrPending {
                    return Ok(None);
                }
                void_reason = Some("SOURCE_UNAVAILABLE");
            } else {
                void_reason = Some(reason)
            }
        } else {
            // Configuration never upgrades admission, metadata, loss or a
            // generic timeout into actual boss-death evidence.
            let native_result = matches!(
                (e.kind.as_str(), phase.as_str()),
                ("RAID", "completed")
                    | ("BOSS", "boss-dead")
                    | ("DAILY_WORLD_GATE" | "WORLD_CHANGE", "realized")
            );
            let Some(option) = b.outcomes.get(phase).filter(|_| native_result) else {
                return Ok(None);
            };
            if !e.options.contains(option) {
                return Ok(None);
            }
            if void_reason.is_some() || outcome.as_ref().is_some_and(|old| old != option) {
                if b.channel_policy == ChannelPolicy::UnanimousOrPending {
                    return Ok(None);
                }
                void_reason = Some("SOURCE_UNAVAILABLE");
            }
            outcome = Some(option.clone());
        }
        hashes.push(hash);
        decided = decided.max(*at);
    }
    let correction = prior.is_some_and(|e| matches!(e.state.as_str(), "SETTLED" | "VOID"));
    if let Some(reason) = void_reason {
        e.state = "VOID".into();
        e.outcome.clear();
        e.reason_code = reason.into();
    } else if let Some(option) = outcome {
        e.state = "SETTLED".into();
        e.outcome = vec![option];
        e.reason_code = "NATIVE_RESULT".into();
    } else {
        return Ok(None);
    }
    if decided <= 0 || decided > now {
        return Err(DurabilityError::InvalidStoredState);
    }
    e.decided_at = vec![decided.to_string()];
    let question = (
        &e.scheduled_start,
        &e.scheduled_end,
        &e.lock_at,
        &e.options,
        &e.ruleset_revision,
        &e.content_sha256,
        &b.channels,
        b.advance_approved,
        b.channel_policy,
    );
    let proof = serde_json::to_vec(&(
        event,
        world,
        &e.kind,
        &e.source_key,
        &e.occurrence_id,
        question,
        &hashes,
        &e.state,
        &e.outcome,
        &e.decided_at,
        c.generation,
    ))
    .map_err(invalid)?;
    e.evidence_sha256 = vec![wire::digest(&proof)];
    if correction {
        let p = prior.ok_or(DurabilityError::InvalidStoredState)?;
        if p.evidence_sha256 == e.evidence_sha256 && p.outcome == e.outcome {
            return Ok(Some(Decision {
                record: p.clone(),
                proof,
            }));
        }
        e.reason_code = "CORRECTION".into();
        e.correction_of = vec![wire::digest(&serde_json::to_vec(p).map_err(invalid)?)];
    }
    Ok(Some(Decision { record: e, proof }))
}

pub(crate) async fn capture_raid(
    tx: &mut Transaction<'_, Postgres>,
    id: RaidStepIdentity,
    phase: &str,
    at: i64,
) -> Result<Disposition> {
    super::source_start(tx).await?;
    let result = capture_raid_inner(tx, id, phase, at).await;
    super::source_finish(tx, result).await
}
pub(crate) async fn capture_reset(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Disposition> {
    super::source_start(tx).await?;
    let result = capture_reset_inner(tx, world, epoch).await;
    super::source_finish(tx, result).await
}

pub(super) async fn capture_gate_unavailable(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Disposition> {
    if !enabled(tx).await? {
        return Ok(Disposition::Pending);
    }
    let world = uuid_text(world.as_bytes())?;
    let cut = format!("reset/{epoch}");
    let occurrence:Option<String>=sqlx::query_scalar("SELECT occurrence_id::text FROM game_t94_native_receipts WHERE world_id=$1::uuid AND kind='DAILY_WORLD_GATE' AND source_key='fury_gates' AND source_cut=$2")
  .bind(&world).bind(&cut).fetch_optional(&mut **tx).await?;
    let occurrence = match occurrence {
        Some(id) => id,
        None => {
            sqlx::query_scalar("SELECT uuidv7()::text")
                .fetch_one(&mut **tx)
                .await?
        }
    };
    let (at,content):(i64,String)=sqlx::query_as("SELECT floor(extract(epoch FROM activated_at))::bigint,encode(target_digest,'hex') FROM game_world_reset_epochs WHERE world_id=$1::uuid AND reset_epoch=$2::text::numeric")
  .bind(&world).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    let proof = wire::digest(
        &serde_json::to_vec(&(&world, epoch, &content, "no-accepted-realization"))
            .map_err(invalid)?,
    );
    capture(
        tx,
        Fact {
            world,
            kind: "DAILY_WORLD_GATE".into(),
            occurrence,
            key: "fury_gates".into(),
            channel: String::new(),
            phase: "source-unavailable".into(),
            cut,
            generation: 0,
            proof,
            content_sha256: content,
            native_at: 0,
        },
        at,
    )
    .await
}

pub(crate) async fn capture_window(
    tx: &mut Transaction<'_, Postgres>,
    id: RaidStepIdentity,
    definition: &oteryn_gameplay_encounter_rules::raid::RaidDefinition,
    fired: i64,
) -> Result<Disposition> {
    super::source_start(tx).await?;
    let result=async{
  if !enabled(tx).await?{return Ok(Disposition::Pending)}
  let delay=definition.waves.iter().map(|w|w.delay_ms).min().ok_or(DurabilityError::InvalidStoredState)?;
  let first=fired.checked_add(i64::from(delay)).ok_or(DurabilityError::InvalidStoredState)?;
  let end:i64=sqlx::query_scalar("SELECT deadline_ms FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid").bind(id.firing_id.as_slice()).fetch_one(&mut **tx).await?;
  let generation:String=sqlx::query_scalar("SELECT recovery_generation::text FROM game_t94_control WHERE singleton").fetch_one(&mut **tx).await?;
  sqlx::query("INSERT INTO game_t94_raid_windows(firing_id,channel_id,owner_generation,first_wave_at_ms,deadline_ms,recovery_generation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6::text::numeric) ON CONFLICT DO NOTHING")
   .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(id.generation.get().to_string()).bind(first).bind(end).bind(&generation).execute(&mut **tx).await?;
  let matches:bool=sqlx::query_scalar("SELECT first_wave_at_ms=$4 AND deadline_ms=$5 AND recovery_generation=$6::text::numeric FROM game_t94_raid_windows WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$3::text::numeric")
   .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(id.generation.get().to_string()).bind(first).bind(end).bind(generation).fetch_one(&mut **tx).await?;
  if !matches{return Err(DurabilityError::InvalidStoredState)}Ok(Disposition::Applied)
 }.await;
    super::source_finish(tx, result).await
}

fn private_record(snapshot: &Snapshot) -> Option<&EventRecord> {
    match snapshot {
        Snapshot::Event(s) => s.event.first(),
        _ => None,
    }
}
#[allow(clippy::too_many_arguments)] // Same subject coordinates as live evidence.
async fn expired(
    tx: &mut Transaction<'_, Postgres>,
    c: &Control,
    world: &str,
    event: &str,
    b: &EventBinding,
    old: Option<&Snapshot>,
    rows: &[sqlx::postgres::PgRow],
    now: i64,
) -> Result<Disposition> {
    let prior = old.and_then(private_record);
    if prior.is_some_and(|e| e.state == "CLOSED") {
        return Ok(Disposition::Pending);
    }
    let mut receipts = Vec::new();
    let mut decided = 0i64;
    for row in rows {
        if row.try_get::<Vec<u8>, _>("receipt")?.is_empty() {
            let Some(at) = row.try_get::<Option<i64>, _>("expired_at")? else {
                return Ok(Disposition::Pending);
            };
            decided = decided.max(at);
            receipts.push(row.try_get::<Vec<u8>, _>("receipt_sha256")?);
        }
    }
    if decided <= 0 || decided > now {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut record = b.question.clone();
    record.state = "VOID".into();
    record.outcome.clear();
    record.reason_code = "SOURCE_UNAVAILABLE".into();
    record.decided_at = vec![decided.to_string()];
    let proof = serde_json::to_vec(&(
        "native-retention-expiry",
        world,
        event,
        &b.question,
        &receipts,
        decided,
        c.generation,
    ))
    .map_err(invalid)?;
    record.evidence_sha256 = vec![wire::digest(&proof)];
    if let Some(previous) = prior {
        if previous.evidence_sha256 == record.evidence_sha256 {
            if b.corrections_closed
                && b.correction_deadline
                    .is_some_and(|at| at <= c.cut && at <= now as u64)
            {
                record = previous.clone();
                record.state = "CLOSED".into();
                record.reason_code = "FINAL_FREEZE".into();
            } else {
                return Ok(Disposition::Pending);
            }
        } else if matches!(previous.state.as_str(), "SETTLED" | "VOID") {
            record.reason_code = "CORRECTION".into();
            record.correction_of = vec![wire::digest(
                &serde_json::to_vec(previous).map_err(invalid)?,
            )];
        }
    }
    sqlx::query("INSERT INTO game_t94_evidence(evidence_sha256,world_id,event_id,recovery_generation,evidence) VALUES(sha256($1),$2::uuid,$3::uuid,$4::text::numeric,$1) ON CONFLICT DO NOTHING").bind(proof).bind(world).bind(event).bind(c.generation.to_string()).execute(&mut **tx).await?;
    let snapshot = Snapshot::Event(EventSnapshot {
        contract_version: 1,
        operation: "PublishWorldEventOutcomeV1".into(),
        source_authority: c.authority.clone(),
        world_id: world.into(),
        event_id: event.into(),
        projection_epoch: c.epoch.to_string(),
        projection_revision: revision(old)?,
        source_observed_at: now.to_string(),
        event: vec![record],
    });
    persist(tx, &snapshot, old, &b.question_key, c, now, None).await
}

/// A real completed run can prove loss of a promised boss source, never a kill.
/// Preserve a separately committed death receipt; a generic completion/cleanup
/// must not overwrite it with a timeout interpretation.
pub(crate) async fn capture_boss_completion(
    tx: &mut Transaction<'_, Postgres>,
    id: RaidStepIdentity,
    now: i64,
) -> Result<Disposition> {
    super::source_start(tx).await?;
    let result=async{
  if !enabled(tx).await?{return Ok(Disposition::Pending)}
  let occurrence=uuid_text(&id.firing_id)?;
  let promised:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_native_receipts WHERE world_id=(SELECT world_id FROM game_world_raid_firings WHERE firing_id=$1::uuid) AND kind='BOSS' AND occurrence_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_t94_subjects WHERE world_id=(SELECT world_id FROM game_world_raid_firings WHERE firing_id=$1::uuid) AND family='world_event_outcome' AND occurrence_id=$1::uuid AND (CASE WHEN octet_length(record)>0 THEN convert_from(record,'UTF8')::jsonb ELSE NULL END)->>'kind'='BOSS')").bind(&occurrence).fetch_one(&mut **tx).await?;
  if !promised{return Ok(Disposition::Pending)}
  let killed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_native_receipts WHERE world_id=(SELECT world_id FROM game_world_raid_firings WHERE firing_id=$1::uuid) AND kind='BOSS' AND occurrence_id=$1::uuid AND source_cut LIKE '%/boss-dead' AND recovery_generation=(SELECT recovery_generation FROM game_t94_control WHERE singleton))").bind(occurrence).fetch_one(&mut **tx).await?;
  if killed{return Ok(Disposition::Replayed)}
  capture_raid_inner(tx,id,"boss-source-unavailable",now).await
 }.await;
    super::source_finish(tx, result).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t94_native_terminal_phase_cannot_be_upgraded_by_configuration()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../docs/contracts/fixtures/world-event-guild-season/snapshots.json"
        ))?;
        let Snapshot::Event(s) =
            Snapshot::decode(fixtures["vectors"][0]["wire"].as_str().ok_or("wire")?)?
        else {
            return Err("fixture".into());
        };
        let e = s.event[0].clone();
        let mut b = EventBinding {
            channel_policy: Default::default(),
            occurrence_id: e.occurrence_id.clone(),
            question_key: "t94:test-native-phase".into(),
            withdraw_display: false,
            question: e.clone(),
            channels: vec![],
            advance_approved: true,
            outcomes: BTreeMap::from([
                ("completed".into(), "completed".into()),
                ("admitted".into(), "completed".into()),
                ("boss-dead".into(), "completed".into()),
                ("source-unavailable".into(), "completed".into()),
            ]),
            void_reason: None,
            corrections_closed: false,
            correction_deadline: None,
        };
        let c = Control {
            started: 0,
            epoch: 3,
            generation: 1,
            authority: s.source_authority,
            cut: 1790000200,
            resync: true,
        };
        let channels = vec![String::new()];
        let facts = |phase: &str| {
            BTreeMap::from([(String::new(), (phase.into(), "a".repeat(64), 1790000195))])
        };
        for phase in ["admitted", "boss-dead"] {
            assert!(
                terminal(
                    &b,
                    e.clone(),
                    None,
                    &channels,
                    &facts(phase),
                    &c,
                    &s.world_id,
                    &s.event_id,
                    1790000200
                )?
                .is_none()
            );
        }
        let result = terminal(
            &b,
            e.clone(),
            None,
            &channels,
            &facts("completed"),
            &c,
            &s.world_id,
            &s.event_id,
            1790000200,
        )?
        .ok_or("native completion")?;
        assert_eq!(result.record.state, "SETTLED");
        assert_eq!(result.record.reason_code, "NATIVE_RESULT");
        let mut boss = e.clone();
        boss.kind = "BOSS".into();
        assert!(
            terminal(
                &b,
                boss.clone(),
                None,
                &channels,
                &facts("completed"),
                &c,
                &s.world_id,
                &s.event_id,
                1790000200
            )?
            .is_none()
        );
        assert_eq!(
            terminal(
                &b,
                boss,
                None,
                &channels,
                &facts("boss-dead"),
                &c,
                &s.world_id,
                &s.event_id,
                1790000200
            )?
            .ok_or("boss-death")?
            .record
            .state,
            "SETTLED"
        );
        assert_eq!(
            terminal(
                &b,
                e.clone(),
                None,
                &channels,
                &facts("source-unavailable"),
                &c,
                &s.world_id,
                &s.event_id,
                1790000200
            )?
            .ok_or("native source loss")?
            .record
            .state,
            "VOID"
        );
        let all = vec!["first".into(), "second".into()];
        let mixed = BTreeMap::from([
            (
                "first".into(),
                ("completed".into(), "b".repeat(64), 1790000195),
            ),
            (
                "second".into(),
                ("source-unavailable".into(), "c".repeat(64), 1790000196),
            ),
        ]);
        assert!(
            terminal(
                &b,
                e.clone(),
                None,
                &all,
                &mixed,
                &c,
                &s.world_id,
                &s.event_id,
                1790000200
            )?
            .is_none()
        );
        b.channel_policy = ChannelPolicy::AllTargetsOrVoid;
        assert_eq!(
            terminal(
                &b,
                e.clone(),
                None,
                &all,
                &mixed,
                &c,
                &s.world_id,
                &s.event_id,
                1790000200
            )?
            .ok_or("aggregate void")?
            .record
            .state,
            "VOID"
        );
        b.void_reason = Some("SOURCE_UNAVAILABLE".into());
        assert_eq!(
            terminal(
                &b,
                e,
                None,
                &channels,
                &facts("source-unavailable"),
                &c,
                &s.world_id,
                &s.event_id,
                1790000200
            )?
            .ok_or("unavailable")?
            .record
            .state,
            "VOID"
        );
        Ok(())
    }
}
