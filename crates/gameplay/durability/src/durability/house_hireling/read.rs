use super::super::character_authority::{ReconciledCharacterAuthority,assert_recovery_fence};
use super::super::db::{begin_semantic_transaction,commit_semantic_transaction,lock_admission_relations};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError,DurabilityRoot};
use super::{HirelingSnapshot,fence::assert_inventory_fence,model::{MAX_HIRELINGS,decode,snapshot_query}};
use sqlx::{Postgres, Row, Transaction};

/// The caller holds the current named House fence and property/access locks in
/// this same transaction. Reuse the native bounded projection through movement
/// publication, so a committed placement cannot be hidden by a delayed refresh.
pub(crate) async fn read_house_hirelings_in_transaction(
    tx: &mut Transaction<'_, Postgres>, fence: &CurrentCharacterItemFence,
) -> Result<Vec<HirelingSnapshot>, DurabilityError> {
    let rows=snapshot_query!(" FROM game_house_hirelings h \
        JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($1,'hex')::uuid \
        WHERE h.active AND h.world_id=s.world_id AND h.house_key=s.runtime_scope_house_key \
        AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id ORDER BY h.hireling_id LIMIT 11")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if rows.len()>MAX_HIRELINGS{return Err(DurabilityError::InvalidStoredState)}
    rows.iter().map(decode).collect()
}

impl DurabilityRoot {
    /// A bounded current projection for any admitted occupant of this named House.
    pub async fn read_house_hirelings(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        fence:CurrentCharacterItemFence,origin_channel:[u8;16],
    )->Result<Vec<HirelingSnapshot>,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if !super::fence::assert_house_inventory_fence(&mut tx,&node,&fence,origin_channel).await? {return Err(DurabilityError::InvalidStoredState)}
            let result=read_house_hirelings_in_transaction(&mut tx, &fence).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }

    pub async fn read_owned_hirelings(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        fence:CurrentCharacterItemFence,origin_channel:[u8;16],
    )->Result<Vec<HirelingSnapshot>,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_inventory_fence(&mut tx,&node,&fence,origin_channel).await? {return Err(DurabilityError::InvalidStoredState)}
            let rows=snapshot_query!(" FROM game_house_hirelings h \
                WHERE owner_character_id=encode($1,'hex')::uuid ORDER BY hireling_id LIMIT 11")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            if rows.len()>MAX_HIRELINGS {return Err(DurabilityError::InvalidStoredState)}
            let result=rows.iter().map(decode).collect::<Result<Vec<_>,_>>()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }

    /// The viewer may be an admitted guest. Only the Hireling owner's entitlements apply.
    pub async fn load_house_hireling_service(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        fence:CurrentCharacterItemFence,origin_channel:[u8;16],id:[u8;16],
    )->Result<Option<HirelingSnapshot>,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if !super::fence::assert_house_inventory_fence(&mut tx,&node,&fence,origin_channel).await? {
                return Err(DurabilityError::InvalidStoredState)
            }
            let row=snapshot_query!(" FROM game_house_hirelings h \
                JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($2,'hex')::uuid \
                WHERE h.hireling_id=encode($1,'hex')::uuid AND h.active AND h.world_id=s.world_id \
                AND h.house_key=s.runtime_scope_house_key \
                AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id")
                .bind(id.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let result=row.as_ref().map(decode).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }

    /// Stable binding lookup, followed by the same live House service checks at the consumer.
    pub async fn hireling_lamp_identity(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,fence:CurrentCharacterItemFence,lamp:[u8;16],
    )->Result<Option<[u8;16]>,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let row=sqlx::query("SELECT uuid_send(hireling_id) AS id FROM game_house_hirelings \
                WHERE lamp_item_instance_id=encode($1,'hex')::uuid AND owner_character_id=encode($2,'hex')::uuid")
                .bind(lamp.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let id=row.map(|r|r.try_get::<Vec<u8>,_>("id")?.try_into().map_err(|_|DurabilityError::InvalidStoredState)).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(id)
        })).await
    }
}
