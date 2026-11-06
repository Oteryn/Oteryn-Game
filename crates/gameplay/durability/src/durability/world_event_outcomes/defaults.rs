//! Conservative functional defaults approved by owner 2026-11-06.
use super::*;
use crate::foundation::WorldId;
use wire::{Challenge, ProducerDefaults};
impl DurabilityRoot {
    /// Interprets sources only after the separately fenced bootstrap/reconciliation.
    /// A restored store still needs explicit reconciliation and its nonrollback fence.
    pub async fn t94_start_world(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: WorldId,
        defaults: &ProducerDefaults,
        source: &str,
    ) -> Result<Disposition> {
        if !oteryn_gameplay_native_admission_source::character_commands::source_authority_valid(
            source,
        ) || defaults.season_advance_seconds < 3
            || defaults.season_advance_seconds > 86400
            || defaults.season_seconds < 60
            || defaults.season_seconds > 31536000
            || defaults.event_window_seconds == 0
            || defaults.event_window_seconds > 86400
            || defaults.freeze_delay_seconds > 31536000
            || defaults.correction_seconds > 7776000
            || defaults.max_score == 0
            || defaults.eligible_guilds.len() > 1000
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        // Eligible/scoring rules require an explicit SeasonBinding, rather than
        // changing the immutable no-eligible-guild default season after it starts.
        let recovery = authority.record_for(self).map_err(invalid)?;
        let defaults = defaults.clone();
        let source = source.to_owned();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
   let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
   let Some(c)=control(&mut tx,recovery.recovery_generation).await?else{return Ok(Disposition::Pending)};
   if c.authority!=source{return Err(DurabilityError::Unavailable)}
   let world_text=native::uuid_text(world.as_bytes())?;
   let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut *tx).await?;
   // Catch native resets committed before the publisher's boot. No invented
   // historical settlement: these are permanently ineligible VOID records.
   let epochs:Vec<String>=sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_event_epochs e WHERE world_id=$1::uuid AND NOT EXISTS(SELECT 1 FROM game_t94_native_receipts n WHERE n.world_id=e.world_id AND n.kind='WORLD_CHANGE' AND n.source_cut='reset/'||e.reset_epoch::text) ORDER BY reset_epoch LIMIT 32")
    .bind(&world_text).fetch_all(&mut *tx).await?;
   for epoch in &epochs{
    let epoch=wire::decimal(epoch,false).map_err(invalid)?;
    native::capture_reset(&mut tx,world,epoch).await?;
    if !defaults.fury_gate_realization_available{native::capture_gate_unavailable(&mut tx,world,epoch).await?;}
   }
   let mut gate_backlog=false;
   if !defaults.fury_gate_realization_available {
    let missing:Vec<String>=sqlx::query_scalar("SELECT e.reset_epoch::text FROM game_world_event_epochs e WHERE e.world_id=$1::uuid AND NOT EXISTS(SELECT 1 FROM game_t94_native_receipts n WHERE n.world_id=e.world_id AND n.kind='DAILY_WORLD_GATE' AND n.source_key='fury_gates' AND n.source_cut='reset/'||e.reset_epoch::text) ORDER BY e.reset_epoch LIMIT 32").bind(&world_text).fetch_all(&mut *tx).await?;
    gate_backlog=missing.len()==32;
    for epoch in missing {native::capture_gate_unavailable(&mut tx,world,wire::decimal(&epoch,false).map_err(invalid)?).await?;}
   }
   let native=sqlx::query("SELECT DISTINCT ON(n.kind,n.occurrence_id) n.kind,n.source_key,n.occurrence_id::text,n.observed_at,convert_from(n.receipt,'UTF8')::jsonb->>'content_sha256' AS content_sha256,(convert_from(n.receipt,'UTF8')::jsonb->>'native_at')::bigint AS native_at FROM game_t94_native_receipts n WHERE n.world_id=$1::uuid AND n.kind IN('RAID','BOSS','DAILY_WORLD_GATE','WORLD_CHANGE') AND octet_length(n.receipt)>0 AND NOT EXISTS(SELECT 1 FROM game_t94_bindings b WHERE b.family='world_event_outcome' AND b.world_id=n.world_id AND convert_from(b.binding,'UTF8')::jsonb->>'occurrence_id'=n.occurrence_id::text AND convert_from(b.binding,'UTF8')::jsonb->'question'->>'kind'=n.kind) ORDER BY n.kind,n.occurrence_id,n.delivery_order LIMIT 32")
    .bind(&world_text).fetch_all(&mut *tx).await?;
   let native_backlog=native.len()==32;
   for row in native {
    let occurrence:String=row.try_get("occurrence_id")?;let at=row.try_get::<i64,_>("native_at")? as u64;
    let end=at.checked_add(defaults.event_window_seconds).ok_or(DurabilityError::InvalidStoredState)?;
    let kind:String=row.try_get("kind")?;let key:String=row.try_get("source_key")?;
    let question=EventRecord{kind:kind.clone(),source_key:key,occurrence_id:occurrence.clone(),ruleset_revision:"t94:no-advance-history-v1".into(),content_sha256:row.try_get("content_sha256")?,scheduled_start:at.to_string(),scheduled_end:end.to_string(),lock_at:at.to_string(),options:match kind.as_str(){"RAID"=>vec!["completed".into(),"timed-out".into()],"BOSS"=>vec!["killed".into(),"not-killed".into()],_=>vec!["ineligible".into(),"realized".into()]},state:"SCHEDULED".into(),outcome:Vec::new(),decided_at:Vec::new(),correction_of:Vec::new(),reason_code:"NONE".into(),evidence_sha256:Vec::new()};
    let channels=if matches!(kind.as_str(),"RAID"|"BOSS"){
        sqlx::query_scalar::<_,String>("SELECT channel_id::text FROM game_world_raid_targets WHERE firing_id=$1::uuid ORDER BY channel_id LIMIT 1025").bind(&occurrence).fetch_all(&mut *tx).await?
    }else{Vec::new()};
    if channels.len()>1024{return Err(DurabilityError::InvalidStoredState)}
    let binding=EventBinding{channel_policy:Default::default(),occurrence_id:occurrence,withdraw_display:false,question_key:format!("t94:no-advance-history-v1/{}",kind.to_ascii_lowercase()),question,channels,advance_approved:false,outcomes:BTreeMap::new(),void_reason:Some(if kind=="DAILY_WORLD_GATE"&&!defaults.fury_gate_realization_available{"SOURCE_UNAVAILABLE"}else{"INVALID_SOURCE"}.into()),corrections_closed:true,correction_deadline:Some(at.checked_add(defaults.correction_seconds).ok_or(DurabilityError::InvalidStoredState)?)};
    sqlx::query("INSERT INTO game_t94_bindings(family,world_id,subject_id,approved,binding) VALUES('world_event_outcome',$1::uuid,uuidv7(),true,$2)").bind(&world_text).bind(serde_json::to_vec(&binding).map_err(invalid)?).execute(&mut *tx).await?;
   }
   sqlx::query("INSERT INTO game_t94_world_policies(world_id,first_start,season_seconds,defaults_sha256,fury_gate_realization_available) VALUES($1::uuid,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
    .bind(&world_text).bind(i64::try_from((now as u64).checked_add(defaults.season_advance_seconds).ok_or(DurabilityError::InvalidStoredState)?).map_err(invalid)?).bind(i64::try_from(defaults.season_seconds).map_err(invalid)?).bind(Sha256::digest(serde_json::to_vec(&defaults).map_err(invalid)?).as_slice()).bind(defaults.fury_gate_realization_available).execute(&mut *tx).await?;
   let (first,duration,defaults_hash):(i64,i64,Vec<u8>)=sqlx::query_as("SELECT first_start,season_seconds,defaults_sha256 FROM game_t94_world_policies WHERE world_id=$1::uuid").bind(&world_text).fetch_one(&mut *tx).await?;
   if defaults_hash!=Sha256::digest(serde_json::to_vec(&defaults).map_err(invalid)?).as_slice()||duration as u64!=defaults.season_seconds{return Err(DurabilityError::InvalidStoredState)}
   let first=first as u64;
   let latest:Option<i64>=sqlx::query_scalar("SELECT max(starts_at) FROM game_t94_default_seasons WHERE world_id=$1::uuid").bind(&world_text).fetch_one(&mut *tx).await?;
   let starts=match latest{None=>first,Some(last)=>{let last=u64::try_from(last).map_err(invalid)?;let next=last.checked_add(defaults.season_seconds).ok_or(DurabilityError::InvalidStoredState)?;if next<=now as u64{next}else{last}}};
   let calendar_backlog=starts.checked_add(defaults.season_seconds).ok_or(DurabilityError::InvalidStoredState)?<=now as u64;
   let ends=starts.checked_add(defaults.season_seconds).ok_or(DurabilityError::InvalidStoredState)?;
   let freeze=ends.checked_add(defaults.freeze_delay_seconds).ok_or(DurabilityError::InvalidStoredState)?;
   let (season,challenge):(String,String)=sqlx::query_as("INSERT INTO game_t94_default_seasons(world_id,starts_at) VALUES($1::uuid,$2) ON CONFLICT(world_id,starts_at) DO UPDATE SET starts_at=excluded.starts_at RETURNING season_id::text,challenge_id::text")
    .bind(&world_text).bind(i64::try_from(starts).map_err(invalid)?).fetch_one(&mut *tx).await?;
   let template=SeasonRecord{ruleset_revision:"t94:zero-score-season-v1".into(),starts_at:starts.to_string(),ends_at:ends.to_string(),freeze_at:freeze.to_string(),state:"OPEN".into(),challenges:vec![Challenge{challenge_id:challenge,source_key:"world:gathering".into(),scoring_revision:"t94:zero-score-season-v1".into(),max_score:defaults.max_score.to_string()}],standings:Vec::new(),computed_at:now.to_string(),source_complete_through:now.to_string(),correction_of:Vec::new()};
   let binding=SeasonBinding{attribution:Default::default(),withdraw_display:false,season:template,rules:BTreeMap::new(),eligible_guilds:defaults.eligible_guilds.clone(),tombstone_name:defaults.tombstone_name.clone(),corrections_closed:true,correction_deadline:Some(freeze),complete_through:now.saturating_sub(1) as u64};
   sqlx::query("INSERT INTO game_t94_bindings(family,world_id,subject_id,approved,binding) VALUES('guild_season',$1::uuid,$2::uuid,true,$3) ON CONFLICT(family,world_id,subject_id) DO UPDATE SET binding=convert_to(jsonb_set(convert_from(game_t94_bindings.binding,'UTF8')::jsonb,'{complete_through}',to_jsonb($4::bigint))::text,'UTF8')")
    .bind(&world_text).bind(season).bind(serde_json::to_vec(&binding).map_err(invalid)?).bind(now.saturating_sub(1)).execute(&mut *tx).await?;
   sqlx::query("UPDATE game_t94_bindings SET binding=convert_to(jsonb_set(convert_from(binding,'UTF8')::jsonb,'{complete_through}',to_jsonb($1::bigint))::text,'UTF8') WHERE family='guild_season' AND approved")
    .bind(now.saturating_sub(1)).execute(&mut *tx).await?;
   // Empty eligibility is a proven small, complete source cut, never a score
   // inferred from a public highscore. Pending publications still fence watermarks.
   if epochs.len()<32&&!gate_backlog&&!native_backlog&&!calendar_backlog {
    sqlx::query("UPDATE game_t94_control SET complete_through=GREATEST(complete_through,$1::bigint) WHERE singleton")
     .bind(now.saturating_sub(1)).execute(&mut *tx).await?;
   }
   commit_semantic_transaction(tx,deadline).await?;Ok(Disposition::Applied)
  })).await
    }
}
