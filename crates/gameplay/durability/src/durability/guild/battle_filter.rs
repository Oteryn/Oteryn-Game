//! Viewer-authorized same-guild observations. Other Guild IDs and membership are not disclosed.
use super::*;
impl DurabilityRoot {
    pub async fn read_battle_guild_allies(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        targets: &[[u8; 16]],
    ) -> Result<Vec<bool>> {
        if targets.len() > oteryn_protocol_oteryn::battle_metadata::MAX_ENTRIES {
            return Err(GuildError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        let targets = targets.iter().map(|id| id.to_vec()).collect::<Vec<_>>();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let result=async {
                let member=read_current_guild_chat_data(&mut tx,&recovery,&node,&fence,None).await?;
                let Some(member)=member else{return Ok(vec![false;targets.len()]);};
                let rows=sqlx::query("SELECT EXISTS(SELECT 1 FROM game_guild_members m WHERE m.character_id=encode(t.character_bytes,'hex')::uuid AND m.guild_id=encode($2,'hex')::uuid) AS same FROM unnest($1::bytea[]) WITH ORDINALITY t(character_bytes,ord) ORDER BY ord LIMIT 16")
                    .bind(&targets).bind(member.guild_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
                rows.iter().map(|row|row.try_get("same").map_err(GuildError::from)).collect::<Result<Vec<bool>>>()
            }.await;
            match result {
                Ok(data)=>{commit_semantic_transaction(tx,deadline).await?;Ok(Ok(data))},
                Err(GuildError::Unavailable(error))=>{tx.rollback().await?;Err(error)},
                Err(error)=>{tx.rollback().await?;Ok(Err(error))},
            }
        })).await?
    }
}
