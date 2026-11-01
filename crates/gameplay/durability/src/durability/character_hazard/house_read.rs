//! Read the existing Hazard admission selection under actual named-House custody.
//! Initialization and client selection remain the original Channel owner.
use super::*;
use crate::durability::db::begin_type2_transaction;
use crate::durability::house_hireling::fence::assert_house_inventory_fence;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ChannelId;

impl DurabilityRoot {
    pub async fn read_house_hazard_selection(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        origin_channel: ChannelId,
    ) -> Result<Option<(u16, u16)>, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_house_inventory_fence(&mut tx, &node, &fence, *origin_channel.as_bytes()).await? {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            }
            let row: Option<(i16, i16)> = sqlx::query_as(
                "SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid FOR SHARE",
            ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let selection = match row {
                Some((unlocked, selected)) if (1..=12).contains(&unlocked)
                    && (1..=unlocked).contains(&selected) => Some((unlocked as u16, selected as u16)),
                Some(_) => return Err(DurabilityError::InvalidStoredState),
                None => None,
            };
            tx.commit(deadline).await?;
            Ok(Ok(selection))
        })).await?
    }
}
