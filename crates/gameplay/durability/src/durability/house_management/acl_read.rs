use super::*;
use oteryn_gameplay_housing::house_management::{HOUSE_ACL_MEMBERS_MAX, HouseManagementList};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseAclView {
    pub revision: u64,
    pub members: Vec<[u8; 16]>,
}
impl DurabilityRoot {
    pub async fn read_house_acl(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
        list: HouseManagementList,
        content_digest: [u8; 32],
    ) -> Result<HouseAclView> {
        if house.world_id() != fence.runtime_scope.world_id() || content_digest == [0; 32] {
            return Err(HouseManagementError::Rejected);
        }
        let list_key = list.key().ok_or(HouseManagementError::Rejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseManagementError::Rejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(HouseManagementError::Rejected));
            }
            let world = house.world_id().as_bytes().to_vec();
            let key = house.house_key();
            // Preserve the writer's Guild -> property lock order. A shared property lock
            // keeps membership and its global revision from changing between these reads.
            sqlx::query("SELECT g.guild_id FROM game_guilds g JOIN game_house_properties p ON p.owner_guild_id=g.guild_id WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 FOR SHARE OF g")
                .bind(&world).bind(key).fetch_optional(&mut *tx).await?;
            let property = sqlx::query("SELECT p.state,p.content_digest,p.acl_revision::text,coalesce(game_house_effective_owner(p.world_id,p.house_key)=encode($3,'hex')::uuid,false) AS owned FROM game_house_properties p WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 FOR SHARE OF p")
                .bind(&world).bind(key).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(property) = property else { return Ok(Err(HouseManagementError::Rejected)) };
            if ![3_i16,4].contains(&property.try_get("state")?) || property.try_get::<Vec<u8>,_>("content_digest")? != content_digest {
                return Ok(Err(HouseManagementError::Rejected));
            }
            if !property.try_get::<bool,_>("owned")? {
                let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) WHERE role='SUBOWNER' AND NOT content_fenced)")
                    .bind(&world).bind(key).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if !allowed || list != HouseManagementList::Guest { return Ok(Err(HouseManagementError::Rejected)); }
            }
            let ids: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_key=$3 ORDER BY character_id LIMIT 201")
                .bind(&world).bind(key).bind(&list_key).fetch_all(&mut *tx).await?;
            if ids.len() > HOUSE_ACL_MEMBERS_MAX { return Ok(Err(HouseManagementError::Rejected)); }
            let Ok(members) = ids.into_iter().map(|id| id.try_into()).collect::<std::result::Result<Vec<[u8;16]>,_>>() else {
                return Ok(Err(HouseManagementError::Rejected));
            };
            Ok(Ok(HouseAclView { revision: numeric_u64(&property,"acl_revision")?, members }))
        })).await?
    }
}
