//! BED-REGEN-1 admission participant. One fenced transaction updates the ordered
//! projection, records its immutable outcome and consumes the sleeper/revision.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{
    LastPositionKey, LastPositionWrite, LastVitals, load_last_position, upsert_last_position,
};
use super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;

impl DurabilityRoot {
    /// Settle before first-entry installs an actor. Same-session replay is a read
    /// of durable history; no new sleep can be consumed by an old admission retry.
    pub async fn settle_bed_regen(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<(), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            let args = || (fence.game_session_id.as_bytes().as_slice(), fence.character_id.as_bytes().as_slice(), fence.connection_generation.get().to_string(), fence.character_lease_generation.to_string());
            let (session, character, generation, lease) = args();
            let sleeper = sqlx::query("SELECT * FROM game_bed_regen_lock(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric)")
                .bind(session).bind(character).bind(&generation).bind(&lease).fetch_optional(&mut *tx).await?;
            let Some(sleeper) = sleeper else {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(());
            };
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_pending_arrivals WHERE character_id=encode($1,'hex')::uuid)")
                .bind(character).fetch_one(&mut *tx).await?;
            let stored = load_last_position(&mut tx, fence.character_id).await?;
            let previous: u64 = sleeper.try_get::<String, _>("lease")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            if !pending && sleeper.try_get::<Option<bool>, _>("eligible")? == Some(true)
                && let Some(mut stored) = stored
                // A legacy projection from another lease cannot fund this sleep.
                && stored.key.character_lease_generation == previous
                && stored.record.world_id == fence.runtime_scope.world_id()
                && let Some(mut rest) = stored.record.rest
            {
                let build = sqlx::query("SELECT coalesce(b.vocation,'none') AS vocation,p.level AS level FROM game_character_roots r LEFT JOIN game_character_build_state b USING(character_id) LEFT JOIN game_character_progression_state p USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
                    .bind(character).fetch_one(&mut *tx).await?;
                let vocation: String = build.try_get("vocation")?;
                let level = match build.try_get::<Option<i64>, _>("level")? {
                    Some(level) => u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
                    None if vocation == "none" => 1,
                    None => return Err(DurabilityError::InvalidStoredState),
                };
                let (hp, mana, soul) = oteryn_gameplay_characters::vitals::maxima(&vocation,level,
                    oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,oteryn_protocol_oteryn::actor_spell::MAX_SOUL)
                    .ok_or(DurabilityError::InvalidStoredState)?;
                let slept: i64 = sleeper.try_get("slept_at")?;
                let wake: i64 = sleeper.try_get("wake_at")?;
                let end = sleeper.try_get::<Option<i64>, _>("freed_at")?.unwrap_or(wake).min(wake);
                let seconds = u64::try_from((i128::from(end)-i128::from(slept)).max(0)/1000).map_err(|_| DurabilityError::InvalidStoredState)?;
                let before = stored.record.vitals.map_or([hp,mana,rest[0]], |v| [v.health(),v.mana(),rest[0]]);
                let (after,food) = oteryn_gameplay_housing::house_bed_regen::settle(before,[hp,mana,soul],rest[1],seconds).ok_or(DurabilityError::InvalidStoredState)?;
                rest[0]=after[2]; rest[1]=food;
                stored.record.vitals=LastVitals::new(after[0],after[1]);
                stored.record.rest=Some(rest);
                let key = LastPositionKey { character_lease_generation:fence.character_lease_generation,write_sequence:1 };
                if upsert_last_position(&mut tx,fence.character_id.as_bytes(),key,&stored.record).await? != LastPositionWrite::Written {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            sqlx::query("SELECT game_bed_regen_finish(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric)")
                .bind(session).bind(character).bind(generation).bind(lease).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(())
        })).await
    }
}
