use super::*;
use crate::durability::party_kill_xp::PartyKillXpRequest;
use oteryn_gameplay_native_admission_source::world_event_outcomes::{
    Score, SeasonSnapshot, Standing,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    pub season_id: String,
    pub challenge_id: String,
    pub occurrence_id: String,
    pub guild_id: String,
    pub source_receipt_id: String,
    pub revision: u64,
    #[serde(default)]
    pub withdraw: bool,
}

/// Called in the sealed lethal occurrence's transaction, after all participating
/// Character/session fences passed. No later membership lookup can reattribute it.
pub(crate) async fn capture_kill<const N: usize>(
    tx: &mut Transaction<'_, Postgres>,
    input: &PartyKillXpRequest<N>,
) -> Result<Disposition> {
    let enabled = native::enabled(tx).await?;
    if !enabled {
        return Ok(Disposition::Pending);
    }
    let Some(member) = input.members.first() else {
        return Ok(Disposition::Pending);
    };
    let world = native::uuid_text(member.fence.runtime_scope.world_id().as_bytes())?;
    let bindings=sqlx::query("SELECT subject_id::text,binding FROM game_t94_bindings WHERE family='guild_season' AND world_id=$1::uuid AND approved AND (convert_from(binding,'UTF8')::jsonb->'season'->>'starts_at')::numeric>(SELECT source_started_at FROM game_t94_control WHERE singleton) AND floor(extract(epoch FROM approved_at))<(convert_from(binding,'UTF8')::jsonb->'season'->>'starts_at')::numeric AND convert_from(binding,'UTF8')::jsonb->'rules' ? $2 AND (convert_from(binding,'UTF8')::jsonb->'season'->>'starts_at')::numeric<=floor(extract(epoch FROM clock_timestamp())) AND (convert_from(binding,'UTF8')::jsonb->'season'->>'ends_at')::numeric>floor(extract(epoch FROM clock_timestamp())) ORDER BY subject_id LIMIT 33 FOR SHARE")
  .bind(&world).bind(&input.creature).fetch_all(&mut **tx).await?;
    if bindings.len() > 32 {
        sqlx::query("UPDATE game_t94_control SET quarantined=true WHERE singleton")
            .execute(&mut **tx)
            .await?;
        return Ok(Disposition::Pending);
    }
    if bindings.is_empty() {
        return Ok(Disposition::Pending);
    }
    let mut has_source = false;
    for row in &bindings {
        let binding: SeasonBinding =
            serde_json::from_slice(&row.try_get::<Vec<u8>, _>("binding")?).map_err(invalid)?;
        has_source |=
            binding.rules.contains_key(&input.creature) && !binding.eligible_guilds.is_empty();
    }
    if !has_source {
        return Ok(Disposition::Pending);
    }
    let generation:Option<String>=sqlx::query_scalar("SELECT c.recovery_generation::text FROM game_t94_control c WHERE c.singleton AND c.enabled AND c.reconciled AND c.recovery_generation=(SELECT recovery_generation FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1)")
  .fetch_optional(&mut **tx).await?;
    let Some(generation) = generation else {
        return Ok(Disposition::Pending);
    };
    let accepted: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let cut = wire::digest(&input.death_key);
    let receipt:Option<(String,String)>=sqlx::query_as("SELECT receipt_id::text,occurrence_id::text FROM game_t94_native_receipts WHERE world_id=$1::uuid AND kind='GUILD_SOURCE' AND source_key=$2 AND source_cut=$3")
  .bind(&world).bind(&input.creature).bind(&cut).fetch_optional(&mut **tx).await?;
    let (receipt_id, occurrence) = if let Some(existing) = receipt {
        existing
    } else {
        let occurrence: String = sqlx::query_scalar("SELECT uuidv7()::text")
            .fetch_one(&mut **tx)
            .await?;
        let raw = serde_json::to_vec(&(
            &world,
            &input.creature,
            &cut,
            &occurrence,
            input.death_at_ms,
            &generation,
        ))
        .map_err(invalid)?;
        let id:String=sqlx::query_scalar("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,'GUILD_SOURCE',$2::uuid,$3,$4,$5::text::numeric,$6,sha256($6),$7) RETURNING receipt_id::text")
   .bind(&world).bind(&occurrence).bind(&input.creature).bind(&cut).bind(&generation).bind(raw).bind(accepted).fetch_one(&mut **tx).await?;
        (id, occurrence)
    };
    let actors: Vec<Vec<u8>> = input
        .members
        .iter()
        .map(|m| m.fence.character_id.as_bytes().to_vec())
        .collect();
    let principal = native::uuid_text(input.principal.as_bytes())?;
    let memberships=sqlx::query("SELECT g.guild_id::text,g.name,g.revision::text,m.character_id::text,m.joined_at FROM game_guild_members m JOIN game_guilds g USING(guild_id) WHERE m.character_id=ANY(SELECT encode(x,'hex')::uuid FROM unnest($1::bytea[]) x) AND g.world_id=$2::uuid AND g.state IN(1,2) ORDER BY g.guild_id,m.character_id FOR SHARE OF g,m")
  .bind(actors).bind(&world).fetch_all(&mut **tx).await?;
    for row in bindings {
        let season: String = row.try_get("subject_id")?;
        let b: SeasonBinding =
            serde_json::from_slice(&row.try_get::<Vec<u8>, _>("binding")?).map_err(invalid)?;
        let at = u64::try_from(accepted).map_err(invalid)?;
        if at < wire::decimal(&b.season.starts_at, false).map_err(invalid)?
            || at >= wire::decimal(&b.season.ends_at, false).map_err(invalid)?
        {
            continue;
        }
        let frozen:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_subjects WHERE family='guild_season' AND world_id=$1::uuid AND subject_id=$2::uuid AND final_state)").bind(&world).bind(&season).fetch_one(&mut **tx).await?;
        if frozen {
            continue;
        }
        let Some(score) = b.rules.get(&input.creature) else {
            continue;
        };
        for challenge in &b.season.challenges {
            if challenge.source_key != input.creature {
                continue;
            }
            if *score > wire::decimal(&challenge.max_score, false).map_err(invalid)? {
                return Err(DurabilityError::InvalidStoredState);
            }
            for member in &memberships {
                if b.attribution == Attribution::PrincipalOnly
                    && member.try_get::<String, _>("character_id")? != principal
                {
                    continue;
                }
                let guild: String = member.try_get("guild_id")?;
                if !b.eligible_guilds.contains(&guild) {
                    continue;
                }
                let membership = serde_json::to_vec(&(
                    &guild,
                    member.try_get::<String, _>("revision")?,
                    member.try_get::<String, _>("character_id")?,
                    member.try_get::<i64, _>("joined_at")?,
                    &cut,
                ))
                .map_err(invalid)?;
                sqlx::query("INSERT INTO game_t94_contributions(season_id,challenge_id,occurrence_id,guild_id,world_id,score,guild_name,membership_cut,source_receipt_id,revision,receipt_sha256) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,$6::text::numeric,$7,sha256($8),$9::uuid,1,sha256($10)) ON CONFLICT DO NOTHING")
     .bind(&season).bind(&challenge.challenge_id).bind(&occurrence).bind(&guild).bind(&world).bind(score.to_string()).bind(member.try_get::<String,_>("name")?).bind(membership).bind(&receipt_id).bind(format!("{receipt_id}/{season}/{}/{guild}/{score}",challenge.challenge_id).into_bytes()).execute(&mut **tx).await?;
            }
        }
    }
    Ok(Disposition::Applied)
}

pub(super) async fn produce(
    tx: &mut Transaction<'_, Postgres>,
    c: &Control,
    world: &str,
    season: &str,
    b: &SeasonBinding,
    old: Option<&Snapshot>,
    now: i64,
) -> Result<Disposition> {
    if !b.rules.is_empty()
        && wire::decimal(&b.season.starts_at, false).map_err(invalid)? <= c.started
    {
        return Ok(Disposition::Pending);
    }
    if !b.season.standings.is_empty() || !matches!(b.season.state.as_str(), "OPEN" | "VOID") {
        return Err(DurabilityError::InvalidStoredState);
    }
    if b.complete_through == 0 || b.complete_through > now as u64 {
        return Ok(Disposition::Pending);
    }
    let prior = match old {
        Some(Snapshot::Season(s)) => s.season.first(),
        Some(_) => return Err(DurabilityError::InvalidStoredState),
        None => None,
    };
    if prior.is_some_and(|s| s.state != "OPEN") {
        return Ok(Disposition::Pending);
    }
    let mut s = b.season.clone();
    // A single transaction's consistent cut, bounded at one beyond every full
    // guild/challenge Cartesian product. Refuse instead of dropping rows.
    let rows=sqlx::query("SELECT guild_id::text,min(guild_name) AS name,challenge_id::text,sum(score)::text AS score FROM game_t94_contributions WHERE world_id=$1::uuid AND season_id=$2::uuid GROUP BY guild_id,challenge_id ORDER BY guild_id,challenge_id LIMIT 32001")
  .bind(world).bind(season).fetch_all(&mut **tx).await?;
    if rows.len() > 32000 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("UPDATE game_t94_bindings SET native_seen_order=(SELECT coalesce(max(delivery_order),0) FROM game_t94_native_receipts WHERE world_id=$1::uuid AND kind='GUILD_SOURCE') WHERE family='guild_season' AND world_id=$1::uuid AND subject_id=$2::uuid").bind(world).bind(season).execute(&mut **tx).await?;
    let mut guilds: BTreeMap<String, Standing> = BTreeMap::new();
    for row in rows {
        let guild: String = row.try_get("guild_id")?;
        let challenge: String = row.try_get("challenge_id")?;
        if !b.eligible_guilds.contains(&guild) {
            return Ok(Disposition::Pending);
        }
        let g = guilds.entry(guild.clone()).or_insert_with(|| Standing {
            guild_id: guild,
            guild_name: String::new(),
            scores: s
                .challenges
                .iter()
                .map(|c| Score {
                    challenge_id: c.challenge_id.clone(),
                    score: "0".into(),
                })
                .collect(),
            total_score: "0".into(),
            rank: "0".into(),
        });
        g.guild_name = row.try_get("name")?;
        let Some(index) = s
            .challenges
            .iter()
            .position(|c| c.challenge_id == challenge)
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let amount = wire::decimal(&row.try_get::<String, _>("score")?, true).map_err(invalid)?;
        if amount > wire::decimal(&s.challenges[index].max_score, false).map_err(invalid)? {
            return Err(DurabilityError::InvalidStoredState);
        }
        g.scores[index].score = amount.to_string();
    }
    if guilds.len() > 1000 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for guild in &b.eligible_guilds {
        guilds.entry(guild.clone()).or_insert_with(|| Standing {
            guild_id: guild.clone(),
            guild_name: String::new(),
            scores: s
                .challenges
                .iter()
                .map(|c| Score {
                    challenge_id: c.challenge_id.clone(),
                    score: "0".into(),
                })
                .collect(),
            total_score: "0".into(),
            rank: "0".into(),
        });
    }
    if guilds.len() > 1000 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for guild in guilds.values_mut() {
        let live:Option<(String,i16)>=sqlx::query_as("SELECT name,state FROM game_guilds WHERE guild_id=$1::uuid AND world_id=$2::uuid FOR SHARE").bind(&guild.guild_id).bind(world).fetch_optional(&mut **tx).await?;
        guild.guild_name = match live {
            Some((name, state)) if state != 4 => name,
            _ => match &b.tombstone_name {
                Some(name) => name.clone(),
                None => return Ok(Disposition::Pending),
            },
        };
        let total = guild.scores.iter().try_fold(0u64, |sum, score| {
            sum.checked_add(wire::decimal(&score.score, true).map_err(invalid)?)
                .ok_or(DurabilityError::InvalidStoredState)
        })?;
        guild.total_score = total.to_string();
    }
    let mut standings: Vec<_> = guilds.into_values().collect();
    // Numeric vectors, never the lexicographic order of decimal strings.
    standings.sort_by(|a, b| {
        let total = |g: &Standing| g.total_score.parse::<u64>().unwrap_or(0);
        let vector = |g: &Standing| {
            g.scores
                .iter()
                .map(|s| s.score.parse::<u64>().unwrap_or(0))
                .collect::<Vec<_>>()
        };
        total(b)
            .cmp(&total(a))
            .then_with(|| vector(b).cmp(&vector(a)))
            .then_with(|| a.guild_id.cmp(&b.guild_id))
    });
    for (rank, g) in standings.iter_mut().enumerate() {
        g.rank = (rank + 1).to_string()
    }
    s.standings = standings;
    s.computed_at = now.to_string();
    s.source_complete_through = b.complete_through.to_string();
    s.state = "OPEN".into();
    if let Some(old) = prior {
        if old.standings != s.standings {
            s.correction_of = vec![wire::digest(&serde_json::to_vec(old).map_err(invalid)?)];
        } else {
            s.correction_of = old.correction_of.clone();
        }
    }
    if b.season.state == "VOID" {
        if let Some(old) = prior {
            s = old.clone();
            s.state = "VOID".into();
        } else {
            s.state = "VOID".into();
        }
    }
    let pending_corrections:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_corrections WHERE approved AND receipt IS NULL AND convert_from(request,'UTF8')::jsonb->>'season_id'=$1)").bind(season).fetch_one(&mut **tx).await?;
    let freeze = wire::decimal(&s.freeze_at, false).map_err(invalid)?;
    let end = wire::decimal(&s.ends_at, false).map_err(invalid)?;
    if !pending_corrections
        && s.state == "OPEN"
        && prior.is_some_and(|p| p.standings == s.standings)
        && now as u64 >= freeze
        && b.complete_through >= end
        && b.corrections_closed
        && b.correction_deadline.is_some_and(|at| at <= now as u64)
    {
        s.state = "FROZEN".into();
    }
    if prior.is_some_and(|p| p.standings == s.standings && p.state == s.state) {
        return Ok(Disposition::Pending);
    }
    let snapshot = Snapshot::Season(SeasonSnapshot {
        contract_version: 1,
        operation: "PublishGuildSeasonV1".into(),
        source_authority: c.authority.clone(),
        world_id: world.into(),
        season_id: season.into(),
        projection_epoch: c.epoch.to_string(),
        projection_revision: revision(old)?,
        source_observed_at: now.to_string(),
        season: vec![s],
    });
    persist(tx, &snapshot, old, "", c, now, None).await
}

/// Game-owned, approved interpretation corrections. They cannot author an
/// arbitrary score: only withdraw or restore the already-approved native rule.
pub(super) async fn correct_one(tx: &mut Transaction<'_, Postgres>, c: &Control) -> Result<bool> {
    let row=sqlx::query("SELECT request_id::text,request,floor(extract(epoch FROM approved_at))::bigint AS approved_cut FROM game_t94_corrections WHERE approved AND receipt IS NULL ORDER BY created_at,request_id LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(false) };
    let request_id: String = row.try_get("request_id")?;
    let raw: Vec<u8> = row.try_get("request")?;
    let r: Contribution = match serde_json::from_slice(&raw) {
        Ok(r) => r,
        Err(_) => {
            correction_receipt(tx, &request_id, &raw, "REFUSED_MALFORMED", c.generation).await?;
            return Ok(true);
        }
    };
    if r.revision == 0
        || [
            &r.season_id,
            &r.challenge_id,
            &r.occurrence_id,
            &r.guild_id,
            &r.source_receipt_id,
        ]
        .iter()
        .any(|id| !wire::uuid7_valid(id))
    {
        correction_receipt(tx, &request_id, &raw, "REFUSED_MALFORMED", c.generation).await?;
        return Ok(true);
    }
    let frozen:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_subjects WHERE family='guild_season' AND subject_id=$1::uuid AND final_state)").bind(&r.season_id).fetch_one(&mut **tx).await?;
    if frozen {
        correction_receipt(tx, &request_id, &raw, "REFUSED_FROZEN", c.generation).await?;
        return Ok(true);
    }
    let stored=sqlx::query("SELECT x.*,x.revision::text AS revision_text,x.world_id::text AS world_text,n.receipt AS native,n.receipt_sha256 AS native_hash,n.source_key,n.recovery_generation::text AS source_generation FROM game_t94_contributions x JOIN game_t94_native_receipts n ON n.receipt_id=x.source_receipt_id WHERE x.season_id=$1::uuid AND x.challenge_id=$2::uuid AND x.occurrence_id=$3::uuid AND x.guild_id=$4::uuid AND x.source_receipt_id=$5::uuid FOR UPDATE OF x")
  .bind(&r.season_id).bind(&r.challenge_id).bind(&r.occurrence_id).bind(&r.guild_id).bind(&r.source_receipt_id).fetch_optional(&mut **tx).await?;
    let Some(stored) = stored else {
        correction_receipt(tx, &request_id, &raw, "REFUSED_SOURCE", c.generation).await?;
        return Ok(true);
    };
    let world: String = stored.try_get("world_text")?;
    let final_state:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_subjects WHERE family='guild_season' AND world_id=$1::uuid AND subject_id=$2::uuid AND final_state)").bind(&world).bind(&r.season_id).fetch_one(&mut **tx).await?;
    let old_revision =
        wire::decimal(&stored.try_get::<String, _>("revision_text")?, false).map_err(invalid)?;
    let result = if final_state {
        "REFUSED_FROZEN"
    } else if old_revision.checked_add(1) != Some(r.revision) {
        "REFUSED_REVISION"
    } else {
        "APPLIED"
    };
    if result == "APPLIED" {
        let native: Vec<u8> = stored.try_get("native")?;
        let native_hash: Vec<u8> = stored.try_get("native_hash")?;
        if native.is_empty()
            || native_hash != Sha256::digest(&native).as_slice()
            || wire::decimal(&stored.try_get::<String, _>("source_generation")?, false)
                .map_err(invalid)?
                != c.generation
        {
            return Ok(false);
        }
        let binding:Option<Vec<u8>>=sqlx::query_scalar("SELECT binding FROM game_t94_bindings WHERE family='guild_season' AND world_id=$1::uuid AND subject_id=$2::uuid AND approved").bind(&world).bind(&r.season_id).fetch_optional(&mut **tx).await?;
        let Some(binding) = binding else {
            return Ok(false);
        };
        let b: SeasonBinding = serde_json::from_slice(&binding).map_err(invalid)?;
        let accepted = u64::try_from(row.try_get::<i64, _>("approved_cut")?).map_err(invalid)?;
        if b.corrections_closed && b.correction_deadline.is_some_and(|at| accepted > at) {
            correction_receipt(tx, &request_id, &raw, "REFUSED_CLOSED", c.generation).await?;
            return Ok(true);
        }
        let source_key: String = stored.try_get("source_key")?;
        let Some(max) = b.rules.get(&source_key) else {
            return Ok(false);
        };
        let score = if r.withdraw { 0 } else { *max };
        let evidence = serde_json::to_vec(&(
            &request_id,
            &native_hash,
            stored.try_get::<Vec<u8>, _>("membership_cut")?,
            &r.season_id,
            &r.challenge_id,
            &r.occurrence_id,
            &r.guild_id,
            r.revision,
            score,
            c.generation,
        ))
        .map_err(invalid)?;
        let receipt:String=sqlx::query_scalar("INSERT INTO game_t94_native_receipts(world_id,kind,occurrence_id,source_key,source_cut,recovery_generation,receipt,receipt_sha256,observed_at) VALUES($1::uuid,'GUILD_SOURCE',$2::uuid,$3,$4,$5::text::numeric,$6,sha256($6),floor(extract(epoch FROM statement_timestamp()))::bigint) RETURNING receipt_id::text")
   .bind(&world).bind(&r.occurrence_id).bind(source_key).bind(format!("correction/{request_id}")).bind(c.generation.to_string()).bind(&evidence).fetch_one(&mut **tx).await?;
        sqlx::query("UPDATE game_t94_contributions SET score=$5::text::numeric,revision=$6::text::numeric,source_receipt_id=$7::uuid,receipt_sha256=sha256($8) WHERE season_id=$1::uuid AND challenge_id=$2::uuid AND occurrence_id=$3::uuid AND guild_id=$4::uuid")
   .bind(&r.season_id).bind(&r.challenge_id).bind(&r.occurrence_id).bind(&r.guild_id).bind(score.to_string()).bind(r.revision.to_string()).bind(receipt).bind(evidence).execute(&mut **tx).await?;
    }
    let receipt =
        serde_json::to_vec(&(wire::digest(&raw), result, c.generation)).map_err(invalid)?;
    sqlx::query(
        "UPDATE game_t94_corrections SET receipt=$2 WHERE request_id=$1::uuid AND receipt IS NULL",
    )
    .bind(request_id)
    .bind(receipt)
    .execute(&mut **tx)
    .await?;
    Ok(true)
}

async fn correction_receipt(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
    raw: &[u8],
    result: &str,
    generation: u64,
) -> Result<()> {
    let receipt = serde_json::to_vec(&(wire::digest(raw), result, generation)).map_err(invalid)?;
    sqlx::query(
        "UPDATE game_t94_corrections SET receipt=$2 WHERE request_id=$1::uuid AND receipt IS NULL",
    )
    .bind(id)
    .bind(receipt)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
