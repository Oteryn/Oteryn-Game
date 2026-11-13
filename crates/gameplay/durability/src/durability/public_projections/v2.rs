//! Bounded V2 delivery passes under independently reconciled Character authority.
use super::*;
use crate::native_admission_source::public_projections::v2::{Body, Family, Facts, Snapshot};

impl DurabilityRoot {
    pub async fn next_public_v2(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, family: Family,
        held: &[String],
    ) -> Result<Option<Snapshot>> {
        if !family.produced() {
            return Ok(None);
        }
        let recovery=authority.record_for(self)?;
        let held=held.to_vec();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if family == Family::WorldDailyKills {
                sqlx::query("SELECT game_statistics_kill_maintenance()").execute(&mut *tx).await?;
            }
            if family == Family::CharacterDeaths {
                sqlx::query("SELECT game_public_death_recheck_due()").execute(&mut *tx).await?;
            }
            if family == Family::CharacterOnlineSessions {
                sqlx::query("SELECT game_public_online_maintain(32)").execute(&mut *tx).await?;
            }
            let row=sqlx::query("WITH head AS (SELECT o.world_id,o.subject,min(o.created_ms) AS oldest FROM game_public_v2_outbox o JOIN game_public_v2_heads h ON h.family=o.family AND h.world_id=o.world_id AND h.subject=o.subject WHERE o.family=$1 AND game_public_v2_ready(h.family,h.world_id,h.subject) AND NOT (o.world_id || ':' || o.subject = ANY($2::text[])) GROUP BY o.world_id,o.subject ORDER BY oldest,o.world_id,o.subject LIMIT 1) SELECT h.subject,h.world_id,h.revision,h.observed_ms,h.payload::text AS payload,e.projection_epoch FROM head q JOIN game_public_v2_heads h ON h.family=$1 AND h.world_id=q.world_id AND h.subject=q.subject CROSS JOIN game_public_projection_epoch e WHERE e.epoch_scope=1")
                .bind(family.wire()).bind(held).fetch_optional(&mut *tx).await?;
            let snapshot=match row {
                None=>None,
                Some(row)=>{
                    let payload:String=row.try_get("payload")?;
                    let invalid=|e| sqlx::Error::Decode(Box::new(e));
                    let body=match family {
                        Family::CharacterDeaths=>Body::Deaths(serde_json::from_str(&payload).map_err(invalid)?),
                        Family::Guild=>Body::Guild(serde_json::from_str(&payload).map_err(invalid)?),
                        Family::House=>Body::House(serde_json::from_str(&payload).map_err(invalid)?),
                        Family::CharacterOnlineSessions=>Body::OnlineSessions(serde_json::from_str(&payload).map_err(invalid)?),
                        Family::WorldDailyKills=>Body::DailyKills(serde_json::from_str(&payload).map_err(invalid)?),
                        Family::CharacterSkills | Family::CharacterAchievements
                        | Family::CharacterEquipment | Family::CharacterOutfit => return Ok(Ok(None)),
                    };
                    Some(Snapshot { subject:row.try_get("subject")?, world_id:row.try_get("world_id")?,
                        projection_epoch:positive(row.try_get("projection_epoch")?).map_err(|_|super::super::DurabilityError::InvalidStoredState)?,
                        projection_revision:positive(row.try_get("revision")?).map_err(|_|super::super::DurabilityError::InvalidStoredState)?,
                        source_observed_at:row.try_get::<i64,_>("observed_ms")?/1000,body })
                }
            };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(snapshot))
        })).await?
    }
    pub async fn ack_public_v2(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, snapshot:&Snapshot,
    )->Result<()> {
        let recovery=authority.record_for(self)?;
        let family=snapshot.family();let subject=snapshot.subject.clone();let world=snapshot.world_id.clone();
        let epoch=wide(snapshot.projection_epoch)?;let revision=wide(snapshot.projection_revision)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if family == Family::WorldDailyKills {
                sqlx::query("SELECT game_statistics_kill_maintenance()").execute(&mut *tx).await?;
            }
            if family == Family::CharacterDeaths {
                sqlx::query("SELECT game_public_death_recheck_due()").execute(&mut *tx).await?;
            }
            if family == Family::CharacterOnlineSessions {
                sqlx::query("SELECT game_public_online_maintain(32)").execute(&mut *tx).await?;
            }
            sqlx::query("DELETE FROM game_public_v2_outbox WHERE family=$1 AND world_id=$2 AND subject=$3 AND epoch<=$4 AND revision<=$5")
                .bind(family.wire()).bind(&world).bind(&subject).bind(epoch).bind(revision).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_public_v2_name_handoffs WHERE family=$1 AND world_id=$2 AND subject=$3 AND epoch<=$4 AND revision<=$5")
                .bind(family.wire()).bind(&world).bind(&subject).bind(epoch).bind(revision).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(()))
        })).await?
    }
    pub async fn public_v2_facts(
        &self,authority:&ReconciledCharacterAuthority<'_, '_>,family:Family,
    )->Result<Facts> {
        if !family.produced() {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery=authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if family == Family::WorldDailyKills {
                sqlx::query("SELECT game_statistics_kill_maintenance()").execute(&mut *tx).await?;
            }
            if family == Family::CharacterDeaths {
                sqlx::query("SELECT game_public_death_recheck_due()").execute(&mut *tx).await?;
            }
            if family == Family::CharacterOnlineSessions {
                sqlx::query("SELECT game_public_online_maintain(32)").execute(&mut *tx).await?;
            }
            let row=sqlx::query("SELECT e.projection_epoch,floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms,least((SELECT min(created_ms) FROM game_public_v2_outbox WHERE family=$1),game_public_v2_source_pending($1)) AS oldest FROM game_public_projection_epoch e WHERE epoch_scope=1")
                .bind(family.wire()).fetch_one(&mut *tx).await?;
            let now:i64=row.try_get("now_ms")?;let oldest:Option<i64>=row.try_get("oldest")?;
            let facts=Facts { projection_epoch:positive(row.try_get("projection_epoch")?).map_err(|_|super::super::DurabilityError::InvalidStoredState)?,
                oldest_undelivered_ms:oldest, now_ms:now };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(facts))
        })).await?
    }
}
