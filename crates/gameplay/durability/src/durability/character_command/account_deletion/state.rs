use super::*;
use wire::digest;

pub(super) async fn decide(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    input: &Owned,
    binding: &[u8],
    now: u64,
) -> Result<Option<Option<Stored>>, DurabilityError> {
    let installed: bool =
        sqlx::query_scalar("SELECT to_regclass('game_account_closure_jobs') IS NOT NULL")
            .fetch_one(&mut **tx)
            .await?;
    if !installed {
        return Ok(None);
    }
    let prior = sqlx::query("SELECT command,command_binding,receipt,result_code FROM game_account_closure_jobs WHERE operation_id=$1::uuid FOR UPDATE")
        .bind(r.operation_id).fetch_optional(&mut **tx).await?;
    if let Some(row) = &prior {
        if row.try_get::<String, _>("command")? != r.command
            || row.try_get::<Vec<u8>, _>("command_binding")? != binding
        {
            return Ok(Some(None));
        }
        if let Some(receipt) = row.try_get::<Option<Vec<u8>>, _>("receipt")? {
            return Ok(Some(Some(Stored {
                receipt,
                result_code: row.try_get("result_code")?,
            })));
        }
    }
    if !r.deletion.enabled {
        return Ok(None);
    }
    let raw = match input {
        Owned::Intent(raw) => raw,
        Owned::ReadConflict => {
            return common_rejection(tx, r, binding, OPERATION_CONFLICT, now).await;
        }
        Owned::Oversized => return common_rejection(tx, r, binding, INTENT_INVALID, now).await,
        Owned::Unsupported => return common_rejection(tx, r, binding, UNSUPPORTED, now).await,
    };
    let intent = match decode(raw, r.command, r.operation_id) {
        Ok(v) => v,
        // Invalid bodies have no trusted account binding: persist a common v2
        // rejection in the existing immutable command receipt store.
        Err(code) => {
            let result = common_rejection(tx, r, binding, code, now).await?;
            if let Some(Some(stored)) = &result {
                audit_payload(tx, r, raw, &stored.receipt, now).await?;
            }
            return Ok(result);
        }
    };
    if prior.is_none() {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:closure-backlog',0))")
            .execute(&mut **tx)
            .await?;
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_account_closure_jobs WHERE receipt IS NULL AND command=$1 LIMIT 128) q").bind(r.command).fetch_one(&mut **tx).await?;
        if count >= 128 {
            return Ok(None);
        }

        sqlx::query("INSERT INTO game_account_closure_jobs(operation_id,command,command_binding,intent,account_id,termination_id,lifecycle_generation,source_revision,recovery_generation,scheduled_for_source,created_at) VALUES($1::uuid,$2,$3,$4,$5::uuid,$6::uuid,$7::numeric,$8::numeric,$9::numeric,$10::bigint,$11)")
            .bind(r.operation_id).bind(r.command).bind(binding).bind(raw).bind(&intent.account_id).bind(&intent.termination_id)
            .bind(&intent.lifecycle_generation).bind(&intent.source_revision).bind(r.recovery_generation.to_string()).bind(&intent.scheduled_for).bind(now as i64).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING").bind(&intent.account_id).execute(&mut **tx).await?;
    sqlx::query(
        "SELECT account_id FROM game_character_account_guards WHERE account_id=$1::uuid FOR UPDATE",
    )
    .bind(&intent.account_id)
    .fetch_one(&mut **tx)
    .await?;
    let current = sqlx::query("SELECT *,close_operation_id::text AS close_key,resolution_operation_id::text AS resolution_key,termination_id::text AS termination_key,lifecycle_generation::text AS generation,close_generation::text AS original_generation FROM game_account_closures WHERE account_id=$1::uuid FOR UPDATE")
        .bind(&intent.account_id).fetch_optional(&mut **tx).await?;
    let generation = number(&intent.lifecycle_generation)?;
    if number(&intent.expected_recovery_generation)? != r.recovery_generation {
        return reject(tx, r, STALE_GENERATION, now).await;
    }
    let begun = current.as_ref().is_some_and(|c| {
        c.try_get::<String, _>("close_key").ok().as_deref() == Some(r.operation_id)
    });
    let begun = begun
        || current.as_ref().is_some_and(|c| {
            c.try_get::<Option<String>, _>("resolution_key")
                .ok()
                .flatten()
                .as_deref()
                == Some(r.operation_id)
        });
    if let Some(c) = &current {
        if intent.decision.is_none() && number(&c.try_get::<String, _>("generation")?)? > generation
        {
            return reject(tx, r, STALE_GENERATION, now).await;
        }
        if let Some(winner) = c.try_get::<Option<String>, _>("decision")?
            && intent.decision.as_ref().is_some_and(|d| d != &winner)
        {
            return reject(tx, r, RESOLUTION_CONFLICT, now).await;
        }
    }
    if !begun && number(&intent.expires_at_source)? <= now {
        return reject(tx, r, INTENT_EXPIRED, now).await;
    }
    if number(&intent.issued_at_source)? > now.saturating_add(MAX_FUTURE_SKEW_SECONDS) {
        return reject(tx, r, INTENT_INVALID, now).await;
    }
    // A technical retry cannot move the original business deadline. Keep this
    // binding across operation keys and recovery, separately from attempt TTL.
    let close_generation = if intent.decision.is_some() {
        generation.saturating_sub(1)
    } else {
        generation
    };
    let deadline_conflict:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_closure_jobs WHERE account_id=$1::uuid AND termination_id=$2::uuid AND lifecycle_generation=$3::numeric AND command='CloseAccount' AND operation_id<>$4::uuid AND result_code IS DISTINCT FROM 'CHAR_CMD_INTENT_INVALID' AND (scheduled_for_source<$5::bigint OR scheduled_for_source>$5::bigint))")
        .bind(&intent.account_id).bind(&intent.termination_id).bind(close_generation.to_string()).bind(r.operation_id).bind(&intent.scheduled_for).fetch_one(&mut **tx).await?;
    if deadline_conflict {
        return reject(tx, r, INTENT_INVALID, now).await;
    }
    if intent.decision.as_deref() != Some("CANCEL") && number(&intent.scheduled_for)? > now {
        return Ok(pending());
    }
    if !lifecycle_current(tx, r, &intent, now).await? {
        return Ok(pending());
    }
    let account = uuid_text(&intent.account_id)?;
    if account_security(tx, &account, now as i64).await? == Security::Unknown {
        return Ok(pending());
    }
    // Revision identity is shared across accounts for each variant, just as CC.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:closure-source:' || $1 || ':' || $2,0))")
        .bind(&intent.variant).bind(&intent.source_revision).execute(&mut **tx).await?;
    if sqlx::query("SELECT 1 FROM game_account_closure_jobs WHERE command=$1 AND source_revision=$2::numeric AND operation_id<>$3::uuid AND result_code IS DISTINCT FROM 'CHAR_CMD_OPERATION_CONFLICT' LIMIT 1")
        .bind(r.command).bind(&intent.source_revision).bind(r.operation_id).fetch_optional(&mut **tx).await?.is_some() { return reject(tx,r,OPERATION_CONFLICT,now).await; }
    if let Some(c) = &current {
        let term: String = c.try_get("termination_key")?;
        let old_gen = number(&c.try_get::<String, _>("generation")?)?;
        let state: String = c.try_get("state")?;
        if let Some(decision) = &intent.decision {
            if term != intent.termination_id
                || c.try_get::<String, _>("original_generation")?
                    .parse::<u64>()
                    .ok()
                    .and_then(|v| v.checked_add(1))
                    != Some(generation)
            {
                return reject(tx, r, STALE_GENERATION, now).await;
            }
            if c.try_get::<Option<String>, _>("decision")?
                .as_ref()
                .is_some_and(|v| v != decision)
            {
                return reject(tx, r, RESOLUTION_CONFLICT, now).await;
            }
            if let Some(previous) = c.try_get::<Option<String>, _>("resolution_key")?
                && previous != r.operation_id
            {
                let previous_floor:String=sqlx::query_scalar("SELECT recovery_generation::text FROM game_account_closure_jobs WHERE operation_id=$1::uuid").bind(previous).fetch_one(&mut **tx).await?;
                if previous_floor == r.recovery_generation.to_string() {
                    return reject(tx, r, OPERATION_CONFLICT, now).await;
                }
            }
            let reference_known:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_closure_jobs WHERE operation_id=$1::uuid)").bind(&intent.close_operation_id).fetch_one(&mut **tx).await?;
            if !reference_known
                && !(decision == "CANCEL"
                    && state == "CANCELLED"
                    && intent.close_operation_id.as_deref()
                        == Some(c.try_get::<String, _>("close_key")?.as_str()))
            {
                return Ok(pending());
            }
            let reference_ok:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_closure_jobs WHERE operation_id=$1::uuid AND command='CloseAccount' AND account_id=$2::uuid AND termination_id=$3::uuid AND lifecycle_generation=$4::numeric AND (result_code IS NULL OR result_code='CHAR_CMD_OK'))")
                .bind(&intent.close_operation_id).bind(&intent.account_id).bind(&intent.termination_id).bind(c.try_get::<String,_>("original_generation")?).fetch_one(&mut **tx).await?;
            if reference_known && !reference_ok {
                return reject(tx, r, RESOLUTION_CONFLICT, now).await;
            }
            let close_committed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_closure_jobs WHERE operation_id=$1::uuid AND result_code='CHAR_CMD_OK' AND receipt IS NOT NULL)").bind(&intent.close_operation_id).fetch_one(&mut **tx).await?;
            if decision == "FINALIZE" && !close_committed {
                return Ok(pending());
            }
            if decision == "FINALIZE"
                && !matches!(
                    state.as_str(),
                    "CLOSED_DETACHED" | "FINALIZING" | "FINALIZED"
                )
            {
                return Ok(pending());
            }
        } else if term != intent.termination_id
            || generation != old_gen
            || !matches!(state.as_str(), "CLOSING" | "CLOSED_DETACHED")
        {
            if state != "CANCELLED" || term == intent.termination_id || generation <= old_gen {
                return reject(tx, r, STALE_GENERATION, now).await;
            }
            begin_close(tx, r, &intent, now).await?;
            return Ok(pending());
        }
    } else if intent.command == "CloseAccount" {
        begin_close(tx, r, &intent, now).await?;
        return Ok(pending());
    } else if intent.decision.as_deref() == Some("FINALIZE") {
        return Ok(pending());
    } else {
        let Some(close_gen) = generation.checked_sub(1).filter(|g| *g > 0) else {
            return reject(tx, r, INTENT_INVALID, now).await;
        };
        sqlx::query("INSERT INTO game_account_closures(account_id,termination_id,close_operation_id,lifecycle_generation,close_generation,decision,state,recovery_generation,closed_at) VALUES($1::uuid,$2::uuid,$3::uuid,$4::numeric,$5::numeric,'CANCEL','CANCELLING',$6::numeric,$7)")
            .bind(&intent.account_id).bind(&intent.termination_id).bind(&intent.close_operation_id).bind(generation.to_string()).bind(close_gen.to_string()).bind(r.recovery_generation.to_string()).bind(now as i64).execute(&mut **tx).await?;
    }
    if let Some(decision) = &intent.decision {
        // A completed cancellation already restored admission. Recovery reissue
        // reconfirms that retained outcome; it cannot undo subsequent gameplay
        // or require the old actors to be offline again.
        if decision == "CANCEL"
            && current.as_ref().is_some_and(|c| {
                c.try_get::<String, _>("state").ok().as_deref() == Some("CANCELLED")
            })
        {
            return commit(tx, r, &intent, ClosureState::Cancelled, now).await;
        }
        sqlx::query("UPDATE game_account_closures SET decision=$2,lifecycle_generation=$3::numeric,resolution_operation_id=coalesce(resolution_operation_id,$4::uuid),name_release_after_seconds=coalesce(name_release_after_seconds,$5::numeric),state=CASE WHEN $2='CANCEL' THEN 'CANCELLING' ELSE 'FINALIZING' END WHERE account_id=$1::uuid AND state NOT IN ('CANCELLED','FINALIZED')")
            .bind(&intent.account_id).bind(decision).bind(generation.to_string()).bind(r.operation_id).bind(r.deletion.name_release_after_seconds.map(|v|v.to_string())).execute(&mut **tx).await?;
        if decision == "FINALIZE"
            && (!r.deletion.permit_retirement || account_domain_pending(tx, r, &intent).await?)
        {
            return Ok(pending());
        }
        if let Some(member)=sqlx::query("SELECT m.character_id::text,m.root_revision::text,m.step,m.custody_sha256 FROM game_account_closure_members m WHERE m.account_id=$1::uuid AND m.close_operation_id=(SELECT close_operation_id FROM game_account_closures WHERE account_id=$1::uuid) AND m.step IN (0,1) ORDER BY m.character_id LIMIT 1 FOR UPDATE")
            .bind(&intent.account_id).fetch_optional(&mut **tx).await? {
            let character:String=member.try_get("character_id")?;
            let root=sqlx::query("SELECT account_id::text,character_revision::text FROM game_character_roots WHERE character_id=$1::uuid FOR UPDATE").bind(&character).fetch_one(&mut **tx).await?;
            if root.try_get::<String,_>("account_id")?!=intent.account_id || (member.try_get::<i16,_>("step")?==1 && root.try_get::<String,_>("character_revision")?!=member.try_get::<String,_>("root_revision")?) { return Ok(pending()); }
            if !offline(tx,&character,&intent.account_id).await? {return Ok(pending());}
            if member.try_get::<i16,_>("step")?==1 {
                let actual:Vec<u8>=sqlx::query_scalar("SELECT game_account_closure_custody_hash($1::uuid)").bind(&character).fetch_one(&mut **tx).await?;
                if member.try_get::<Option<Vec<u8>>,_>("custody_sha256")?.as_deref()!=Some(actual.as_slice()) {return Ok(pending());}
            }
            if decision=="FINALIZE" {
                if r.deletion.physical_erasure_after_seconds.is_none() {return Ok(pending());}
                let Some(retention)=r.deletion.name_release_after_seconds else { return Ok(pending()); };
                let closed:i64=sqlx::query_scalar("SELECT closed_at FROM game_account_closures WHERE account_id=$1::uuid").bind(&intent.account_id).fetch_one(&mut **tx).await?;
                if now < (closed as u64).saturating_add(retention) { return Ok(pending()); }
                // Names cannot be reused ahead of the accepted public tombstone.
                if sqlx::query("SELECT 1 WHERE EXISTS(SELECT 1 FROM game_public_character_profile_outbox WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_public_name_handoffs WHERE character_id=$1::uuid)").bind(&character).fetch_optional(&mut **tx).await?.is_some() { return Ok(pending()); }
            }
            sqlx::query("UPDATE game_account_closure_members SET step=$3,verified_recovery_generation=$4::numeric,root_revision=CASE WHEN step=0 THEN $5::numeric ELSE root_revision END,custody_sha256=coalesce(custody_sha256,game_account_closure_custody_hash(character_id)) WHERE account_id=$1::uuid AND character_id=$2::uuid AND close_operation_id=(SELECT close_operation_id FROM game_account_closures WHERE account_id=$1::uuid)")
                .bind(&intent.account_id).bind(&character).bind(if decision=="CANCEL" {2i16}else{3i16}).bind(r.recovery_generation.to_string()).bind(root.try_get::<String,_>("character_revision")?).execute(&mut **tx).await?;
            if decision=="FINALIZE" {
                sqlx::query("SELECT game_account_closure_release_name($1::uuid)").bind(&character).execute(&mut **tx).await?;
            }
            return Ok(pending());
        }
        if intent.decision.as_deref() == Some("CANCEL")
            && let Some(member)=sqlx::query("SELECT m.character_id::text,m.root_revision::text,m.custody_sha256 FROM game_account_closure_members m JOIN game_account_closures c ON c.close_operation_id=m.close_operation_id AND c.account_id=m.account_id WHERE m.account_id=$1::uuid AND m.step=2 AND m.custody_sha256 IS NOT NULL AND m.verified_recovery_generation IS DISTINCT FROM $2::numeric ORDER BY m.character_id LIMIT 1 FOR UPDATE OF m")
                .bind(&intent.account_id).bind(r.recovery_generation.to_string()).fetch_optional(&mut **tx).await? {
                let character:String=member.try_get("character_id")?;
                let current=sqlx::query("SELECT character_revision::text,game_account_closure_custody_hash(character_id) AS custody FROM game_character_roots WHERE character_id=$1::uuid AND account_id=$2::uuid FOR UPDATE").bind(&character).bind(&intent.account_id).fetch_optional(&mut **tx).await?;
                let Some(current)=current else{return Ok(pending());};
                if current.try_get::<String,_>("character_revision")?!=member.try_get::<String,_>("root_revision")? || current.try_get::<Vec<u8>,_>("custody")?!=member.try_get::<Vec<u8>,_>("custody_sha256")? || !offline(tx,&character,&intent.account_id).await? {return Ok(pending());}
                sqlx::query("UPDATE game_account_closure_members m SET verified_recovery_generation=$3::numeric FROM game_account_closures c WHERE m.account_id=$1::uuid AND m.character_id=$2::uuid AND m.close_operation_id=c.close_operation_id AND c.account_id=m.account_id")
                    .bind(&intent.account_id).bind(character).bind(r.recovery_generation.to_string()).execute(&mut **tx).await?;
                return Ok(pending());
        }
        return commit(
            tx,
            r,
            &intent,
            if decision == "CANCEL" {
                ClosureState::Cancelled
            } else {
                ClosureState::Finalized
            },
            now,
        )
        .await;
    }
    let canonical_close: String = sqlx::query_scalar(
        "SELECT close_operation_id::text FROM game_account_closures WHERE account_id=$1::uuid",
    )
    .bind(&intent.account_id)
    .fetch_one(&mut **tx)
    .await?;

    let member=sqlx::query("SELECT character_id::text FROM game_account_closure_members WHERE account_id=$1::uuid AND close_operation_id=$2::uuid AND step=0 ORDER BY character_id LIMIT 1 FOR UPDATE")
        .bind(&intent.account_id).bind(&canonical_close).fetch_optional(&mut **tx).await?;
    if let Some(member) = member {
        let character: String = member.try_get("character_id")?;
        let root=sqlx::query("SELECT character_revision::text FROM game_character_roots WHERE character_id=$1::uuid AND account_id=$2::uuid FOR UPDATE").bind(&character).bind(&intent.account_id).fetch_optional(&mut **tx).await?;
        let Some(root) = root else {
            return Ok(pending());
        };
        if !offline(tx, &character, &intent.account_id).await? {
            return Ok(pending());
        }
        if let Some(code) = domains(tx, r, &intent, &character).await? {
            return reject(tx, r, code, now).await;
        }
        if !r.deletion.preserve_custody {
            return Ok(pending());
        }
        sqlx::query("UPDATE game_account_closure_members SET step=1,verified_recovery_generation=$4::numeric,root_revision=$3::numeric,custody_sha256=game_account_closure_custody_hash(character_id) WHERE close_operation_id=$1::uuid AND character_id=$2::uuid")
            .bind(&canonical_close).bind(&character).bind(root.try_get::<String,_>("character_revision")?).bind(r.recovery_generation.to_string()).execute(&mut **tx).await?;
        return Ok(pending());
    }
    if let Some(member)=sqlx::query("SELECT character_id::text,root_revision::text,custody_sha256 FROM game_account_closure_members WHERE close_operation_id=$1::uuid AND step=1 AND verified_recovery_generation IS DISTINCT FROM $2::numeric ORDER BY character_id LIMIT 1 FOR UPDATE")
        .bind(&canonical_close).bind(r.recovery_generation.to_string()).fetch_optional(&mut **tx).await? {
        let character:String=member.try_get("character_id")?;
        let root=sqlx::query("SELECT character_revision::text,game_account_closure_custody_hash(character_id) AS custody FROM game_character_roots WHERE character_id=$1::uuid AND account_id=$2::uuid FOR UPDATE")
            .bind(&character).bind(&intent.account_id).fetch_optional(&mut **tx).await?;
        let Some(root)=root else{return Ok(pending());};
        if root.try_get::<String,_>("character_revision")?!=member.try_get::<String,_>("root_revision")?
            || root.try_get::<Vec<u8>,_>("custody")?!=member.try_get::<Vec<u8>,_>("custody_sha256")?
            || !offline(tx,&character,&intent.account_id).await? {return Ok(pending());}
        if let Some(code)=domains(tx,r,&intent,&character).await? {return reject(tx,r,code,now).await;}
        sqlx::query("UPDATE game_account_closure_members SET verified_recovery_generation=$3::numeric WHERE close_operation_id=$1::uuid AND character_id=$2::uuid")
            .bind(&canonical_close).bind(&character).bind(r.recovery_generation.to_string()).execute(&mut **tx).await?;
        return Ok(pending());
    }
    // Account-level dependencies matter for empty portfolios too.
    if account_domain_pending(tx, r, &intent).await? {
        return Ok(pending());
    }
    commit(tx, r, &intent, ClosureState::ClosedDetached, now).await
}

fn number(v: &str) -> Result<u64, DurabilityError> {
    counter(v).ok_or(DurabilityError::InvalidStoredState)
}

async fn lifecycle_current(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    i: &Intent,
    now: u64,
) -> Result<bool, DurabilityError> {
    if r.deletion.lifecycle_source == LifecycleSource::Pending {
        return Ok(false);
    }
    Ok(sqlx::query("SELECT 1 FROM game_account_closure_lifecycle WHERE usable AND (phase='CANCEL' OR financial_obligations_clear) AND account_id=$1::uuid AND termination_id=$2::uuid AND lifecycle_generation=$3::numeric AND recovery_generation=$4::numeric AND phase=$5 AND observed_at<=$6 AND valid_until>$6 FOR SHARE")
        .bind(&i.account_id).bind(&i.termination_id).bind(&i.lifecycle_generation).bind(r.recovery_generation.to_string()).bind(i.decision.as_deref().unwrap_or("CLOSE")).bind(now as i64).fetch_optional(&mut **tx).await?.is_some())
}

async fn begin_close(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    i: &Intent,
    now: u64,
) -> Result<(), DurabilityError> {
    let roots:Vec<(String,String)>=sqlx::query_as("SELECT character_id::text,character_revision::text FROM game_character_roots WHERE account_id=$1::uuid ORDER BY character_id LIMIT 65 FOR UPDATE")
        .bind(&i.account_id).fetch_all(&mut **tx).await?;
    if roots.len() > 64 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let guilds:Vec<String>=sqlx::query_scalar("SELECT guild_id::text FROM (SELECT m.guild_id FROM game_guild_members m JOIN game_character_roots r USING(character_id) WHERE r.account_id=$1::uuid UNION SELECT v.guild_id FROM game_guild_invitations v JOIN game_character_roots r ON r.character_id IN(v.character_id,v.inviter) WHERE r.account_id=$1::uuid UNION SELECT guild_id FROM game_account_guild_leadership WHERE account_id=$1::uuid) g ORDER BY guild_id LIMIT 1025")
        .bind(&i.account_id).fetch_all(&mut **tx).await?;
    if guilds.len() > 1024 {
        return Ok(());
    }
    for guild in guilds {
        sqlx::query("SELECT guild_id FROM game_guilds WHERE guild_id=$1::uuid FOR UPDATE")
            .bind(guild)
            .fetch_optional(&mut **tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_account_closures(account_id,termination_id,close_operation_id,lifecycle_generation,close_generation,state,recovery_generation,closed_at) VALUES($1::uuid,$2::uuid,$3::uuid,$4::numeric,$4::numeric,'CLOSING',$5::numeric,$6) ON CONFLICT(account_id) DO UPDATE SET termination_id=EXCLUDED.termination_id,close_operation_id=EXCLUDED.close_operation_id,lifecycle_generation=EXCLUDED.lifecycle_generation,close_generation=EXCLUDED.close_generation,decision=NULL,name_release_after_seconds=NULL,state='CLOSING',manifest=convert_to('[]','UTF8'),manifest_count=0,recovery_generation=EXCLUDED.recovery_generation,closed_at=EXCLUDED.closed_at")
        .bind(&i.account_id).bind(&i.termination_id).bind(r.operation_id).bind(&i.lifecycle_generation).bind(r.recovery_generation.to_string()).bind(now as i64).execute(&mut **tx).await?;
    let original: Vec<_> = roots
        .iter()
        .map(|(character, _)| ManifestMember {
            character_id: character.clone(),
            disposition: "PRESERVED_PRIVATE",
        })
        .collect();
    let manifest = serde_json::to_vec(&original).map_err(|_| DurabilityError::Unavailable)?;
    sqlx::query("UPDATE game_account_closures SET manifest=$2,manifest_count=$3,resolution_operation_id=NULL WHERE account_id=$1::uuid")
        .bind(&i.account_id).bind(manifest).bind(roots.len() as i16).execute(&mut **tx).await?;
    for (character, revision) in roots {
        sqlx::query("INSERT INTO game_account_closure_members(close_operation_id,character_id,account_id,root_revision) VALUES($1::uuid,$2::uuid,$3::uuid,$4::numeric)")
            .bind(r.operation_id).bind(character).bind(&i.account_id).bind(revision).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn offline(tx: &mut Tx<'_>, character: &str, account: &str) -> Result<bool, DurabilityError> {
    let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND session_state IN(1,2)) OR EXISTS(SELECT 1 FROM game_durability_admission_character_guards WHERE character_id=$1::uuid AND holder_game_session_id IS NOT NULL) OR EXISTS(SELECT 1 FROM game_durability_admission_account_guards WHERE account_id=$2::uuid AND presence_character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=$1::uuid AND s.character_lease_generation=(SELECT max(s2.character_lease_generation) FROM game_durability_reconnect_sessions s2 WHERE s2.character_id=s.character_id) AND NOT EXISTS(SELECT 1 FROM game_account_closure_offline a WHERE a.game_session_id=s.game_session_id AND a.character_id=s.character_id AND a.lease_generation=s.character_lease_generation))")
        .bind(character).bind(account).fetch_one(&mut **tx).await?;
    Ok(!blocked)
}

async fn domains(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    i: &Intent,
    c: &str,
) -> Result<Option<&'static str>, DurabilityError> {
    let house:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_properties WHERE owner_account_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=$1::uuid AND state IN(1,2))")
        .bind(&i.account_id).fetch_one(&mut **tx).await?;
    let guild:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE character_id=$1::uuid OR inviter=$1::uuid) OR EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE account_id=$2::uuid)")
        .bind(c).bind(&i.account_id).fetch_one(&mut **tx).await?;
    for (policy, held) in [(r.deletion.house, house), (r.deletion.guild, guild)] {
        if policy == OwnerPolicy::Refuse && held {
            return Ok(Some(OBLIGATION));
        }
        if policy == OwnerPolicy::Pending || held {
            return Err(DurabilityError::Unavailable);
        }
    }
    if account_domain_pending(tx, r, i).await? {
        return Err(DurabilityError::Unavailable);
    }
    Ok(None)
}

async fn account_domain_pending(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    i: &Intent,
) -> Result<bool, DurabilityError> {
    if [
        r.deletion.house,
        r.deletion.guild,
        r.deletion.market,
        r.deletion.bank,
    ]
    .contains(&OwnerPolicy::Pending)
    {
        return Ok(true);
    }
    let held:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_market_offers WHERE account_id=$1::uuid AND (state=1 OR matching_pending OR escrow_gold>0 OR held_credit_gold>0)) OR EXISTS(SELECT 1 FROM game_item_market_escrow_locations l JOIN game_market_offers o USING(offer_id) WHERE o.account_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_character_inbox_counters c JOIN game_character_roots r USING(character_id) WHERE r.account_id=$1::uuid AND c.market_reserved>0) OR EXISTS(SELECT 1 FROM game_account_bank_balances WHERE account_id=$1::uuid AND balance<>0) OR EXISTS(SELECT 1 FROM game_guild_disband_claims WHERE account_id=$1::uuid AND amount>0) OR EXISTS(SELECT 1 FROM game_house_properties WHERE owner_account_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_house_bids WHERE account_id=$1::uuid AND state IN(1,2)) OR EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE account_id=$1::uuid)")
        .bind(&i.account_id).fetch_one(&mut **tx).await?;
    Ok(held)
}

async fn reject(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    code: &str,
    now: u64,
) -> Result<Option<Option<Stored>>, DurabilityError> {
    let receipt = encode_receipt(
        r.source_authority,
        r.command,
        r.operation_id,
        code,
        None,
        r.recovery_generation,
        now,
    )?;
    sqlx::query("UPDATE game_account_closure_jobs SET receipt=$2,result_code=$3,decided_at=$4 WHERE operation_id=$1::uuid AND receipt IS NULL")
        .bind(r.operation_id).bind(&receipt).bind(code).bind(now as i64).execute(&mut **tx).await?;
    audit(tx, r, &receipt, now).await?;
    Ok(Some(Some(Stored {
        receipt,
        result_code: code.into(),
    })))
}

#[derive(Serialize)]
struct ManifestMember {
    character_id: String,
    disposition: &'static str,
}
async fn commit(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    i: &Intent,
    state: ClosureState,
    now: u64,
) -> Result<Option<Option<Stored>>, DurabilityError> {
    let c=sqlx::query("SELECT close_operation_id::text AS close_key,manifest,manifest_count FROM game_account_closures WHERE account_id=$1::uuid FOR UPDATE").bind(&i.account_id).fetch_one(&mut **tx).await?;
    let expected_step = match state {
        ClosureState::ClosedDetached => 1i16,
        ClosureState::Cancelled => 2i16,
        ClosureState::Finalized => 3i16,
    };
    let complete:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_account_closure_members WHERE close_operation_id=$1::uuid AND step<>$2) AND (SELECT count(*) FROM game_account_closure_members WHERE close_operation_id=$1::uuid)=$3")
        .bind(c.try_get::<String,_>("close_key")?).bind(expected_step).bind(i64::from(c.try_get::<i16,_>("manifest_count")?)).fetch_one(&mut **tx).await?;
    if !complete {
        return Ok(pending());
    }
    let (manifest, count) = if state == ClosureState::ClosedDetached {
        let complete:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_account_closure_members WHERE close_operation_id=$1::uuid AND step<>1) AND NOT EXISTS(SELECT 1 FROM game_character_roots r WHERE account_id=$2::uuid AND NOT EXISTS(SELECT 1 FROM game_account_closure_members m WHERE m.close_operation_id=$1::uuid AND m.character_id=r.character_id)) AND (SELECT count(*) FROM game_account_closure_members WHERE close_operation_id=$1::uuid)=$3")
            .bind(c.try_get::<String,_>("close_key")?).bind(&i.account_id).bind(i64::from(c.try_get::<i16,_>("manifest_count")?)).fetch_one(&mut **tx).await?;
        if !complete {
            return Ok(pending());
        }

        let members:Vec<(String,String)>=sqlx::query_as("SELECT character_id::text,root_revision::text FROM game_account_closure_members WHERE close_operation_id=$1::uuid ORDER BY character_id LIMIT 65")
            .bind(c.try_get::<String,_>("close_key")?).fetch_all(&mut **tx).await?;
        if members.len() > 64 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let count = members.len() as i16;
        let manifest: Vec<_> = members
            .into_iter()
            .map(|(character_id, _)| ManifestMember {
                character_id,
                disposition: "PRESERVED_PRIVATE",
            })
            .collect();
        (
            serde_json::to_vec(&manifest).map_err(|_| DurabilityError::Unavailable)?,
            count,
        )
    } else {
        (c.try_get("manifest")?, c.try_get("manifest_count")?)
    };
    let state_text = match state {
        ClosureState::ClosedDetached => "CLOSED_DETACHED",
        ClosureState::Cancelled => "CANCELLED",
        ClosureState::Finalized => "FINALIZED",
    };
    sqlx::query("UPDATE game_account_closures SET state=$2,manifest=$3,manifest_count=$4 WHERE account_id=$1::uuid").bind(&i.account_id).bind(state_text).bind(&manifest).bind(count).execute(&mut **tx).await?;
    // A complete empty (or restored) LCFA revision must exist for zero-character subjects too.
    sqlx::query("SELECT game_account_closure_touch_portfolio($1::uuid)")
        .bind(&i.account_id)
        .execute(&mut **tx)
        .await?;
    let pair=sqlx::query("SELECT projection_epoch,projection_revision FROM game_character_account_projection_epoch,game_character_account_projections WHERE epoch_scope=1 AND account_id=$1::uuid").bind(&i.account_id).fetch_one(&mut **tx).await?;
    let mut data = ResultData {
        account_id: i.account_id.clone(),
        termination_id: i.termination_id.clone(),
        lifecycle_generation: i.lifecycle_generation.clone(),
        closure_state: state,
        detached_characters: count.to_string(),
        manifest_sha256: digest(&manifest),
        projection_epoch: pair.try_get::<i64, _>("projection_epoch")?.to_string(),
        projection_revision: pair.try_get::<i64, _>("projection_revision")?.to_string(),
        proof_sha256: None,
    };
    data.seal()?;
    let receipt = encode_receipt(
        r.source_authority,
        r.command,
        r.operation_id,
        OK,
        Some(&data),
        r.recovery_generation,
        now,
    )?;
    sqlx::query("UPDATE game_account_closure_jobs SET receipt=$2,result_code=$3,decided_at=$4 WHERE operation_id=$1::uuid AND receipt IS NULL")
        .bind(r.operation_id).bind(&receipt).bind(OK).bind(now as i64).execute(&mut **tx).await?;
    audit(tx, r, &receipt, now).await?;
    if state == ClosureState::Finalized {
        let not_before = r
            .deletion
            .physical_erasure_after_seconds
            .and_then(|seconds| now.checked_add(seconds))
            .and_then(|v| i64::try_from(v).ok());
        sqlx::query("INSERT INTO game_account_closure_erasure_queue(account_id,operation_id,not_before,authoritative_proof_retention_seconds) VALUES($1::uuid,$2::uuid,$3,$4::numeric) ON CONFLICT(account_id) DO NOTHING")
            .bind(&i.account_id).bind(r.operation_id).bind(not_before).bind(r.deletion.proof_retention_seconds.map(|v|v.to_string())).execute(&mut **tx).await?;
    }
    publish::enqueue(tx, r, &data, &receipt, now).await?;
    Ok(Some(Some(Stored {
        receipt,
        result_code: OK.into(),
    })))
}

#[derive(Clone, PartialEq, prost::Message)]
struct AccountClosureDecisionV1 {
    #[prost(bytes = "vec", tag = "1")]
    intent_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    receipt: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    receipt_sha256: Vec<u8>,
    #[prost(uint64, tag = "4")]
    recovery_generation: u64,
}
async fn audit(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    receipt: &[u8],
    now: u64,
) -> Result<(), DurabilityError> {
    let raw: Vec<u8> = sqlx::query_scalar(
        "SELECT intent FROM game_account_closure_jobs WHERE operation_id=$1::uuid",
    )
    .bind(r.operation_id)
    .fetch_one(&mut **tx)
    .await?;
    audit_payload(tx, r, &raw, receipt, now).await
}
async fn audit_payload(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    raw: &[u8],
    receipt: &[u8],
    now: u64,
) -> Result<(), DurabilityError> {
    use crate::durability::item_mint_audit::{
        CausationRefV1, CausationV1, EventEnvelopeV1, TransactionEventRefV1,
    };
    use prost::Message;
    let payload = AccountClosureDecisionV1 {
        intent_sha256: Sha256::digest(raw).to_vec(),
        receipt: receipt.to_vec(),
        receipt_sha256: Sha256::digest(receipt).to_vec(),
        recovery_generation: r.recovery_generation,
    }
    .encode_to_vec();
    let ids=sqlx::query("SELECT uuid_send(game_character_uuid_v7()) AS event,uuid_send(game_character_uuid_v7()) AS transaction").fetch_one(&mut **tx).await?;
    let event: Vec<u8> = ids.try_get("event")?;
    let transaction: Vec<u8> = ids.try_get("transaction")?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: event.clone(),
        event_type_id: EVENT_TYPE as u32,
        event_schema_revision: 1,
        durability_class: 2,
        privacy_class: 3,
        retention_profile_id: AUDIT_PROFILE.into(),
        occurred_at_unix_ms: (now as i64) * 1000,
        operation_id: Some(r.operation.to_vec()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: transaction.clone(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(CausationRefV1 {
            cause: Some(CausationV1::OperationId(r.operation.to_vec())),
        }),
        server_build_id: SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload: payload.clone(),
        ..EventEnvelopeV1::default()
    }
    .encode_to_vec();
    if envelope.len() > 4096 {
        return Err(DurabilityError::Unavailable);
    }
    sqlx::query("INSERT INTO game_account_closure_audit_outbox(event_id,transaction_id,operation_id,envelope,payload,payload_sha256,occurred_at,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::uuid,$4,$5,sha256($5),$6,$6+7776000000) ON CONFLICT(operation_id) DO NOTHING")
        .bind(event).bind(transaction).bind(r.operation_id).bind(envelope).bind(payload).bind((now as i64)*1000).execute(&mut **tx).await?;
    Ok(())
}

async fn common_rejection(
    tx: &mut Tx<'_>,
    r: &Request<'_>,
    binding: &[u8],
    code: &str,
    now: u64,
) -> Result<Option<Option<Stored>>, DurabilityError> {
    let receipt = encode_receipt(
        r.source_authority,
        r.command,
        r.operation_id,
        code,
        None,
        r.recovery_generation,
        now,
    )?;
    sqlx::query("INSERT INTO game_character_command_receipts(operation_id,command,command_binding,outcome,result_code,recovery_generation,decided_at,receipt) VALUES($1::uuid,$2,$3,2,$4,$5::numeric,$6,$7)")
        .bind(r.operation_id).bind(r.command).bind(binding).bind(code).bind(r.recovery_generation.to_string()).bind(now as i64).bind(&receipt).execute(&mut **tx).await?;
    Ok(Some(Some(Stored {
        receipt,
        result_code: code.into(),
    })))
}
