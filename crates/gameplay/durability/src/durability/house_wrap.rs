//! HOUSE-ITEM-WIRE-1: atomic fenced furniture conversion and custody.
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::house_wrap::{Refusal, Reply, Request};
use sqlx::Row;

mod purchased_kit;
pub(crate) use purchased_kit::record_purchased_kit;

pub struct HouseWrapCommand {
    pub house: HouseId,
    pub command: u64,
    pub request: Request,
}

impl DurabilityRoot {
    /// The callback drafts the deterministic conversion from the actual stored definition.
    /// SQL rechecks its pair, revision, custody, session and edit ACL under the write locks.
    pub async fn wrap_house_furniture<F, C>(
        &self,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        input: HouseWrapCommand,
        conversion: F,
        capacity: C,
    ) -> Result<Reply, DurabilityError>
    where
        F: FnOnce(&str) -> Option<String> + Send + 'static,
        C: FnOnce(&str) -> Option<u8> + Send + 'static,
    {
        let HouseWrapCommand {
            house,
            command,
            request,
        } = input;
        if fence.runtime_scope != house.runtime_scope()? {
            return Ok(Reply::Refused(Refusal::NotInHouse));
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? {
                return Ok(Reply::Refused(Refusal::Stale));
            }
            let key: Option<String> = sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
                .bind(request.item.as_slice()).bind(house.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let after = key.as_deref().and_then(conversion).unwrap_or_default();
            let backpack: Option<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id,world_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_revision_ref='definition-r1'")
                .bind(fence.character_id.as_bytes().as_slice()).bind(house.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let capacity = backpack.as_deref().and_then(capacity).map(i32::from);
            let fact = node.fact();
            let result = sqlx::query("SELECT game_house_furniture_wrap(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,$10::text::numeric,encode($11,'hex')::uuid,$12::text::numeric,$13::text::numeric,$14::smallint,$15,$16,$17,$18,$19)::text")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
                .bind(fence.scope_ownership_generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(house.world_id().as_bytes().as_slice()).bind(house.house_key())
                .bind(command.to_string()).bind(request.item.as_slice()).bind(request.expected_revision.to_string())
                .bind(request.acl_revision.to_string()).bind(request.action as i16)
                .bind(request.position.iter().flat_map(|v| v.to_be_bytes()).collect::<Vec<_>>())
                .bind(request.encode()).bind(after).bind(capacity).bind(backpack).fetch_one(&mut *tx).await;
            let row = match result {
                Ok(row) => row,
                Err(error) => return match error.as_database_error().map(|e| e.message()) {
                    Some("PODIUM_STALE_SESSION" | "WRAP_STALE") => Ok(Reply::Refused(Refusal::Stale)),
                    Some("WRAP_NO_ACCESS") => Ok(Reply::Refused(Refusal::NoAccess)),
                    Some("WRAP_NOT_WRAPABLE") => Ok(Reply::Refused(Refusal::NotWrapable)),
                    Some("WRAP_BLOCKED") => Ok(Reply::Refused(Refusal::Blocked)),
                    Some("WRAP_INVENTORY_FULL") => Ok(Reply::Refused(Refusal::InventoryFull)),
                    _ => Err(error.into()),
                },
            };
            let revision = row.try_get::<String,_>(0)?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Reply::Committed(revision))
        })).await
    }
}
