use super::*;
use crate::foundation::{ChannelId, WorldId};
#[derive(Clone, Debug)]
pub struct HorseStationWorld {
    pub generation: i64,
    pub started: i64,
    pub expires: i64,
    pub ended: bool,
    pub wave: i16,
    pub claimed: bool,
}
#[derive(Clone, Copy)]
pub struct HorseStationChannel {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
}
impl DurabilityRoot {
    pub async fn refresh_horse_station(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: HorseStationChannel,
    ) -> Result<HorseStationWorld, DurabilityError> {
        self.horse_station_work(authority, node, scope, None)
            .await
            .map(|(world, _)| world)
    }
    pub async fn catch_horse_station(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: HorseStationChannel,
        occurrence: [u8; 32],
    ) -> Result<bool, DurabilityError> {
        self.horse_station_work(authority, node, scope, Some(occurrence))
            .await
            .map(|(_, ended)| ended)
    }
    async fn horse_station_work(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: HorseStationChannel,
        occurrence: Option<[u8; 32]>,
    ) -> Result<(HorseStationWorld, bool), DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let fact = node.fact();
            sqlx::query("SELECT game_horse_station_arm_node(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6)")
               .bind(scope.world.as_bytes().as_slice()).bind(scope.channel.as_bytes().as_slice()).bind(scope.generation.to_string())
               .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret()).execute(&mut **tx).await?;
            let ended = if let Some(occurrence) = occurrence {
                sqlx::query_scalar("SELECT game_horse_station_catch($1)")
                    .bind(occurrence.as_slice())
                    .fetch_one(&mut **tx)
                    .await?
            } else {
                false
            };
            // Catch changes the private phase, so only the regular cadence claims waves.
            let row = if occurrence.is_none() {
                sqlx::query("SELECT * FROM game_horse_station_refresh_claim()")
                    .fetch_one(&mut **tx)
                    .await?
            } else {
                sqlx::query("SELECT generation,started_seconds,expires_seconds,ended,-1::smallint AS wave,FALSE AS claimed FROM game_horse_station_worlds WHERE world_id=encode($1,'hex')::uuid")
            .bind(scope.world.as_bytes().as_slice()).fetch_one(&mut **tx).await?
            };
            let world = HorseStationWorld {
                generation: row.try_get("generation")?,
                started: row.try_get("started_seconds")?,
                expires: row.try_get("expires_seconds")?,
                ended: row.try_get("ended")?,
                wave: row.try_get("wave")?,
                claimed: row.try_get("claimed")?,
            };
            tx.commit(deadline).await?;
            Ok((world, ended))
        })).await
    }
}
