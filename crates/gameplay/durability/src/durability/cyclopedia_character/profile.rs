//! Identity and profile facts share one transaction and the existing VIP visibility fence.
use super::*;
use oteryn_protocol_oteryn::cyclopedia_character::{Badge, profile::Profile};
impl DurabilityRoot {
    pub async fn read_cyclopedia_profile(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        viewer_fence: CurrentCharacterItemFence,
        target: CharacterId,
    ) -> Result<Profile, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::super::character_progression::assert_item_only_fence(&mut tx,&viewer_fence,&node).await?.is_err() { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let allowed:bool=sqlx::query_scalar(VISIBLE_SQL).bind(viewer_fence.character_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !allowed { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let Some(name)=sqlx::query_scalar::<_,String>("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(target.as_bytes().as_slice()).fetch_optional(&mut *tx).await? else { return Ok(Err(CharacterAuthorityError::Rejected)); };
            let ids:Vec<i16>=sqlx::query_scalar("SELECT badge FROM game_account_badges WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) ORDER BY badge LIMIT 33").bind(target.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let mut badges=Vec::new();
            for id in ids { let Some(badge)=u8::try_from(id).ok().and_then(Badge::from_id) else { return Ok(Err(CharacterAuthorityError::Rejected)); }; badges.push(badge); }
            let rows=sqlx::query(INSPECTION_SQL).bind(target.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let equipment=rows.iter().map(|row| Ok(EquippedItem {slot:narrow(i64::from(row.try_get::<i16,_>("slot")?))?,definition:row.try_get("definition_production_key")?,quantity:narrow(row.try_get("quantity")?)?})).collect::<Result<Vec<_>,sqlx::Error>>()?;
            let profile=Profile {character:*target.as_bytes(),name,badges,equipment};
            if profile.encode().is_err() { return Ok(Err(CharacterAuthorityError::Rejected)); }
            commit_semantic_transaction(tx,deadline).await?; Ok(Ok(profile))
        })).await?
    }
}
