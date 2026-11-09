//! Publish only the pinned source Primal recipe under current native authority.
use super::*;
impl ActivityHandoffOwner {
    pub async fn activate_primal_boss_catalogue(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        active: &crate::content::ActiveGeneration,
        template: &crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<bool> {
        if template.boss_key() != "oteryn:boss.the_primal_menace"
            || template.creature_key() != "oteryn:creature.the_primal_menace"
            || template.source_path()
                != "data-global/scripts/actions/bosses_levers/the_primal_manace.lua"
            || template.source_sha256()
                != "59768c964e9e6319810e3c0619a1e322b8a8a2c3f03232b021667ae595927703"
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
                let accepted:bool=sqlx::query_scalar("SELECT game_primal_boss_catalogue_activate(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),encode($4,'hex')::uuid,$5::text::numeric(20,0),$6,$7::text::numeric(20,0),$8,$9,$10)")
                    .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(node.sql_possession_secret())
                    .bind(activation.to_string()).bind(server.as_slice()).bind(client.as_slice()).bind(revision).fetch_one(&mut *tx).await?;
                commit_semantic_transaction(tx,deadline).await?;
                Ok(accepted)
            })
        }).await
    }
}
