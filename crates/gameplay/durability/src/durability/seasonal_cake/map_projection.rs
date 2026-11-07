//! Current-Channel replay of the durable source tower projection.
use super::*;
use crate::durability::terrain_tool::{TerrainScope, TerrainState};
#[derive(Debug)]
pub struct CakeTerrainProjection {
    pub state: TerrainState,
    pub source: u16,
}
#[derive(Debug)]
pub struct CakeMapProjection {
    pub world: CakeWorldSnapshot,
    pub removed: Vec<(u16, u16)>,
    pub terrain: Vec<CakeTerrainProjection>,
}
impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn cake_map_projection(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        scope: TerrainScope,
        now_seconds: i64,
    ) -> Result<Option<CakeMapProjection>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let result=async{
                assert_channel_owner(&mut tx,&recovery,&node,world,channel,generation).await?;
                super::authority::arm(&mut tx,&node,world,channel,generation,now_seconds).await?;
                let Some(snapshot)=lock_world(&mut tx,world,now_seconds).await? else{return Ok(None);};
                sqlx::query("SELECT * FROM game_cake_restore_terrain(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6)")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(snapshot.event_start)
                    .bind(&scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch)
                    .execute(&mut *tx).await?;
                // Hydrate all retained source states, including an already committed
                // restoration whose response was lost before local projection.
                let rows=sqlx::query("SELECT DISTINCT ON(t.placement) t.placement,t.object_revision,t.state_item_id,r.appearance FROM game_terrain_tool_transforms t JOIN game_seasonal_cake_bite_receipts r ON r.world_id=t.world_id AND r.channel_id=t.channel_id AND r.placement=t.placement AND r.terrain_reset_epoch=t.reset_epoch AND r.terrain_map_revision=t.map_revision AND r.terrain_content_generation=t.content_generation WHERE t.world_id=encode($1,'hex')::uuid AND t.channel_id=encode($2,'hex')::uuid AND t.map_revision=$3 AND t.content_generation=$4 AND t.reset_epoch=$5 AND r.event_start=$6 AND t.state_item_id IN(0,12085,12086) ORDER BY t.placement,r.world_revision_after DESC LIMIT 1117")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).bind(snapshot.event_start).fetch_all(&mut *tx).await?;
                if rows.len()>1116{return Err(BossClockError::InvalidStoredState);}
                let terrain=rows.into_iter().map(|row|Ok(CakeTerrainProjection{state:TerrainState{placement:row.try_get("placement")?,revision:row.try_get("object_revision")?,item:u32::try_from(row.try_get::<i64,_>("state_item_id")?).map_err(|_|BossClockError::InvalidStoredState)?},source:u16::try_from(row.try_get::<i32,_>("appearance")?).map_err(|_|BossClockError::InvalidStoredState)?})).collect::<Result<Vec<_>>>()?;
                let rows=sqlx::query("SELECT x,y FROM game_seasonal_cake_tiles WHERE world_id=encode($1,'hex')::uuid AND event_start=$2 AND layer=$3 AND removed ORDER BY x,y LIMIT 1117")
                    .bind(world.as_bytes().as_slice()).bind(snapshot.event_start).bind(i16::from(snapshot.state.layer)).fetch_all(&mut *tx).await?;
                if rows.len()>1116{return Err(BossClockError::InvalidStoredState);}
                let removed=rows.into_iter().map(|row|Ok((u16::try_from(row.try_get::<i32,_>("x")?).map_err(|_|BossClockError::InvalidStoredState)?,u16::try_from(row.try_get::<i32,_>("y")?).map_err(|_|BossClockError::InvalidStoredState)?))).collect::<Result<Vec<_>>>()?;
                Ok(Some(CakeMapProjection{world:snapshot,removed,terrain}))
            }.await;
            if result.is_ok(){commit_semantic_transaction(tx,deadline).await?;}
            Ok(result)
        })).await?
    }
}
