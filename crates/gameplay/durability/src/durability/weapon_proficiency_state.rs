//! Current-session fenced observations of the existing proficiency owner; no writes.
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::weapon_proficiency::ProficiencyModifiedPerk;
use sqlx::Row;
pub struct CommittedProficiencyState {
    pub revision: u64,
    pub slots: [Option<ProficiencyModifiedPerk>; 2],
}
impl DurabilityRoot {
    pub async fn read_weapon_proficiency_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        item: &str,
    ) -> Result<CommittedProficiencyState, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        let item = item.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterAuthorityError::Rejected));
            }
            // Serialize this observation against every Character writer; the root's revision
            // is NOT the proficiency command's track revision.
            sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let track = sqlx::query("SELECT committed_character_revision::text AS revision FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid AND item_key=$2")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&item).fetch_optional(&mut *tx).await?;
            let revision = track.as_ref().map(|row| super::character_progression::numeric_u64(row,"revision")).transpose()?.unwrap_or(0);
            let rows = sqlx::query("SELECT slot,level,shaping_key,shaping_revision,entry_index,rank,pending_offer,coalesce(pending_offer IS NULL OR (array_ndims(pending_offer)=1 AND array_lower(pending_offer,1)=1),false) AS canonical FROM game_character_proficiency_modifications WHERE character_id=encode($1,'hex')::uuid AND item_key=$2 ORDER BY slot LIMIT 2")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&item).fetch_all(&mut *tx).await?;
            let mut slots = [None,None];
            for row in rows {
                if !row.try_get::<bool,_>("canonical")? { return Err(DurabilityError::InvalidStoredState); }
                let small = |field: &str| -> Result<u8,DurabilityError> { u8::try_from(row.try_get::<i16,_>(field)?).map_err(|_|DurabilityError::InvalidStoredState) };
                let offer: Option<Vec<i16>> = row.try_get("pending_offer")?;
                let pending_offer = offer.map(|values| {
                    let values = values.into_iter().map(u8::try_from).collect::<Result<Vec<_>,_>>().map_err(|_|DurabilityError::InvalidStoredState)?;
                    values.try_into().map_err(|_|DurabilityError::InvalidStoredState)
                }).transpose()?;
                let perk = ProficiencyModifiedPerk { level:small("level")?,shaping_key:row.try_get("shaping_key")?,shaping_revision:row.try_get("shaping_revision")?,entry_index:small("entry_index")?,rank:small("rank")?,pending_offer };
                if revision == 0 || !perk.is_well_formed() { return Err(DurabilityError::InvalidStoredState); }
                let slot = small("slot")?.checked_sub(1).ok_or(DurabilityError::InvalidStoredState)?;
                let target = slots.get_mut(usize::from(slot)).ok_or(DurabilityError::InvalidStoredState)?;
                if target.is_some() { return Err(DurabilityError::InvalidStoredState); }
                *target = Some(perk);
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(CommittedProficiencyState { revision,slots }))
        })).await?
    }
}
