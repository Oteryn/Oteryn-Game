//! Paged named modifiers read under the existing Character privacy and current viewer fence.
use super::*;
use oteryn_protocol_oteryn::cyclopedia_character::{
    combat::CombatStats,
    offence::{Page, Query, Snapshot},
};
impl DurabilityRoot {
    pub async fn read_cyclopedia_offence(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        target: CharacterId,
        query: Query,
    ) -> Result<Page, CharacterAuthorityError> {
        if query.encode().is_err() || query.target.is_some_and(|id| id != *target.as_bytes()) {
            return Err(CharacterAuthorityError::Rejected);
        }
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
        let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
        if super::super::character_progression::assert_item_only_fence(&mut tx,&fence,&node).await?.is_err() {return Ok(Err(CharacterAuthorityError::Rejected));}
        let allowed:bool=sqlx::query_scalar(VISIBLE_SQL).bind(fence.character_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if !allowed {return Ok(Err(CharacterAuthorityError::Rejected));}
        let Some(row)=sqlx::query("SELECT offence,offence_augments,sample_revision,offence_augments_revision FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid").bind(target.as_bytes().as_slice()).fetch_optional(&mut *tx).await? else {return Ok(Err(CharacterAuthorityError::Rejected));};
        let bytes:Vec<u8>=row.try_get("offence")?;let Ok(stats)=CombatStats::decode(&bytes) else {return Ok(Err(CharacterAuthorityError::Rejected));};
        // Old binaries can update scalar columns without clearing a newer optional column.
        // Its explicit sample binding prevents an older modifier payload from looking current.
        let revision:i64=row.try_get("sample_revision")?;
        let modifier_revision:Option<i64>=row.try_get("offence_augments_revision")?;
        let bytes:Option<Vec<u8>>=if modifier_revision==Some(revision) {row.try_get("offence_augments")?}else{None};
        let facts=match bytes {None=>None,Some(bytes)=>match Snapshot::decode(&bytes) {Ok(value)=>Some(value),Err(_)=>return Ok(Err(CharacterAuthorityError::Rejected))}};
        let Ok(page)=Page::from_snapshot(*target.as_bytes(),query,stats,facts) else {return Ok(Err(CharacterAuthorityError::Rejected));};
        commit_semantic_transaction(tx,deadline).await?;Ok(Ok(page))
    })).await?
    }
}
