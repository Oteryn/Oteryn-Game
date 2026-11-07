//! Audited staff inspection projection.
use super::*;

impl DurabilityRoot {
    /// Audited inspection enrichment. Exact current GM principal, world and session
    /// are re-proved in the query; Platform identity/IP data is never projected.
    pub async fn staff_inspect_summary(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        principal: &str,
        target: &str,
    ) -> Result<String, DurabilityError> {
        if !rules::hex(target, 16) || !rules::atom(principal) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let principal = principal.to_owned();
        let target = target.to_owned();
        self.with_staff_tools_fence(authority,node,fence,move |tx,fence,environment,activation| Box::pin(async move {
            let summary: Option<String>=sqlx::query_scalar("SELECT format('Character: %s (%s); level: %s; experience: %s; vocation: %s; magic: %s; fist/club/sword/axe/distance/shield/fishing: %s/%s/%s/%s/%s/%s/%s',r.name,r.character_id,COALESCE(p.level::text,'unavailable'),COALESCE(p.total_experience::text,'unavailable'),COALESCE(b.vocation,'unavailable'),COALESCE(b.magic_level::text,'unavailable'),COALESCE(b.fist_level::text,'-'),COALESCE(b.club_level::text,'-'),COALESCE(b.sword_level::text,'-'),COALESCE(b.axe_level::text,'-'),COALESCE(b.distance_level::text,'-'),COALESCE(b.shielding_level::text,'-'),COALESCE(b.fishing_level::text,'-')) FROM game_character_roots r LEFT JOIN game_character_progression_state p USING(character_id) LEFT JOIN game_character_build_state b USING(character_id) WHERE r.character_id=$1::uuid AND r.world_id=encode($2,'hex')::uuid AND r.lifecycle=1 AND EXISTS(SELECT 1 FROM game_staff_character_groups g JOIN game_staff_control_heads h ON h.environment=g.environment AND h.activation_sha256=g.activation_sha256 WHERE g.environment=$3 AND g.character_id=encode($4,'hex')::uuid AND g.activation_sha256=$5 AND g.principal=$6 AND g.group_id>=4)")
                .bind(target).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(environment).bind(fence.character_id.as_bytes().as_slice()).bind(activation).bind(principal).fetch_optional(tx).await?;
            summary.ok_or(DurabilityError::Unavailable)
        })).await
    }
}
