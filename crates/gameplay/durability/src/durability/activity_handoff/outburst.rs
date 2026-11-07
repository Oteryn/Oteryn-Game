//! Publish only five pinned Rotten Blood recipes under current native authority.
use super::*;
impl ActivityHandoffOwner {
    pub async fn activate_outburst_boss_catalogue(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        active: &crate::content::ActiveGeneration,
        template: &crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<bool> {
        if template.boss_key() != "oteryn:boss.outburst"
            || template.creature_key() != "oteryn:creature.outburst"
            || template.source_path()
                != "data-global/scripts/quests/heart_of_destruction/actions_outburst_lever.lua"
            || template.source_sha256()
                != "89acba0c46b779140af5fcff854f9e9c37bec19af05937c8112de219fcc454f7"
            || template.issue_runtime_pin(active).is_err()
        {
            return Ok(false);
        }
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(false);
        };
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        let fact = node.fact();
        let boss = template.boss_key().to_owned();
        let revision = template.content_revision().to_owned();
        let server = active.identity().server_artifact_digest();
        let client = active.identity().client_artifact_digest();
        let activation = active.activation_sequence();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline| {
            Box::pin(async move {
                let mut tx=begin_semantic_transaction(holder,deadline).await?;
                assert_recovery_fence(&mut tx,&recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if retained_participant(&mut tx,&root,fence).await?.is_none()
                    || assert_item_only_fence(&mut tx,&fence,&node).await?.is_err() {return Ok(false);}
                let accepted:bool=sqlx::query_scalar("SELECT game_outburst_boss_catalogue_activate($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),encode($5,'hex')::uuid,$6::text::numeric(20,0),$7,$8::text::numeric(20,0),$9,$10,$11)")
                    .bind(boss).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
                    .bind(activation.to_string()).bind(server.as_slice()).bind(client.as_slice()).bind(revision).fetch_one(&mut *tx).await?;
                commit_semantic_transaction(tx,deadline).await?;
                Ok(accepted)
            })
        }).await
    }
}
