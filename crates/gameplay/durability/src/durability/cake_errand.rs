//! Crystal201189f movements_cake_base_isle.lua and movements_extraction_spot.lua.
//! The movement owner selects the immutable canonical map source; this owner
//! re-proves the current Character/Channel/Content/epoch before personal writes.
use super::*;

#[derive(Clone, Debug)]
pub struct CakeErrandStep {
    pub actor: [u8; 16],
    pub movement_revision: u64,
    pub position: [u8; 12],
    pub source_item: u16,
    pub bundle: [u8; 32],
    pub scope: TerrainScope,
}

impl DurabilityRoot {
    pub async fn record_world_cake_step(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: CakeErrandStep,
    ) -> Result<String, DurabilityError> {
        if !matches!(request.source_item, 8886 | 12090..=12095) {
            return Err(DurabilityError::InvalidStoredState);
        }
        permit.check_scope(fence.runtime_scope)?;
        let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, None, request.scope.content_generation).await.map_err(|_| DurabilityError::Unavailable)?;
            let current: bool = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric) AND EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric AND e.target_digest=$3 AND e.map_revision=$4)")
                .bind(world.as_bytes().as_slice()).bind(request.scope.reset_epoch.to_string())
                .bind(request.bundle.as_slice()).bind(&request.scope.map_revision).fetch_one(&mut **tx).await?;
            if !current { return Err(DurabilityError::Unavailable); }
            let Some(year) = latest_year(&mut tx, world).await? else {
                tx.commit(deadline).await?;
                return Ok("inactive".into());
            };
            let result: String = sqlx::query_scalar("SELECT game_cake_errand_step(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7,$8)")
                .bind(world.as_bytes().as_slice()).bind(year).bind(fence.character_id.as_bytes().as_slice())
                .bind(request.actor.as_slice()).bind(request.movement_revision.to_string()).bind(request.scope.reset_epoch.to_string())
                .bind(request.position.as_slice()).bind(i32::from(request.source_item)).fetch_one(&mut **tx).await?;
            if matches!(result.as_str(), "stale" | "invalid") { return Err(DurabilityError::Unavailable); }
            tx.commit(deadline).await?;
            Ok(result)
        })).await
    }
}

#[derive(Clone, Debug)]
pub struct CakeBrutusStatus {
    pub year: i32,
    pub cities: u8,
    pub started: bool,
    pub item_claimed: bool,
    pub cupcake_claimed: bool,
}
impl DurabilityRoot {
    pub async fn cake_brutus_status(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        content: [u8; 32],
        start: bool,
    ) -> Result<Option<CakeBrutusStatus>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, None, content).await
                .map_err(|_| DurabilityError::Unavailable)?;
            let row = sqlx::query("SELECT * FROM game_cake_brutus_status(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)")
                .bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(start)
                .fetch_optional(&mut **tx).await?;
            let status = row.map(|row| Ok::<_,DurabilityError>(CakeBrutusStatus {
                year: row.try_get("event_year")?,
                cities: u8::try_from(row.try_get::<i16,_>("cities")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                started: row.try_get("was_started")?, item_claimed: row.try_get("item_claimed")?,
                cupcake_claimed: row.try_get("cupcake_claimed")?,
            })).transpose()?;
            tx.commit(deadline).await?;
            Ok(status)
        })).await
    }
}
