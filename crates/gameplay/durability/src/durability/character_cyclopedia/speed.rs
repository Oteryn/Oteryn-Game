//! Derive the bonus from fenced, persisted completion; never retain a drifting count.
use super::*;
use std::collections::BTreeMap;
impl DurabilityRoot {
    pub async fn read_cyclopedia_speed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        subareas: &[(u32, u32)],
        current_area: Option<u32>,
    ) -> Result<u16, CharacterProgressionError> {
        if subareas.is_empty() || subareas.len() > 512 {
            return Ok(0);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let subareas = subareas.to_vec();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                return Ok(Err(error));
            }
            let row = sqlx::query("SELECT catalogue_digest,progress::text FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let mut areas = BTreeMap::<u32, bool>::new();
            if let Some(row) = row {
                if !BasicArea::reads_content_digest(&row.try_get::<Vec<u8>,_>("catalogue_digest")?) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let text: String = row.try_get("progress")?;
                if text.len() > 262144 { return Err(DurabilityError::InvalidStoredState); }
                let progress = storage::CanonicalProgress::decode(&text)?;
                for (area, subarea) in &subareas {
                    let done = progress.subareas.get(subarea).map_or(progress.legacy.discovered, |p| p.discovered);
                    *areas.entry(*area).or_insert(true) &= done;
                }
            }
            let rules = crate::domain::cyclopedia::DiscoverySpeed::embedded().ok_or(DurabilityError::InvalidStoredState)?;
            let completed = areas.values().filter(|complete| **complete).count() as u32;
            let total = subareas.iter().map(|(area,_)| *area).collect::<std::collections::BTreeSet<_>>().len() as u32;
            let eligible = current_area.and_then(|area| areas.get(&area)).copied().unwrap_or(false);
            let bonus = rules.bonus(completed, total, eligible);
            tx.commit(deadline).await?;
            Ok(Ok(bonus))
        })).await?
    }
}
