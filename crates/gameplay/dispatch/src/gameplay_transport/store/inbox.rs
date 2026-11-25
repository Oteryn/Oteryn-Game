//! Project the admitted buyer's unclaimed Store Inbox items.
use super::{ComposedFreshAdmission, gameplay_fence, uuid_bytes, wire};
use crate::gameplay_transport::connection::AdmittedSession;

pub(super) async fn project(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    query: wire::StoreInboxQuery,
) -> Option<wire::StoreInboxPage> {
    owner.store.as_ref()?;
    let account = admitted.controller?.account_id;
    let fence = gameplay_fence(owner, admitted).await?;
    let character = *fence.character_id.as_bytes();
    let world = *admitted.world_id.as_bytes();
    let page_size = wire::MAX_INBOX_ENTRIES;
    let fetch_limit = i64::try_from(page_size + 1).ok()?;
    let rows: Vec<(sqlx::types::Uuid, String, String)> = owner.root.try_issue_semantic_pass().ok()?
        .run(move |holder, _deadline| Box::pin(async move {
            Ok(sqlx::query_as(
                "SELECT r.operation_id, r.fulfilment_unit_id, i.definition_production_key
                 FROM store_delivery_receipts r
                 JOIN game_character_roots c ON c.character_id = r.character_id
                 JOIN game_item_character_inbox_locations x
                   ON x.item_instance_id = r.item_instance_id
                   AND x.character_id = r.character_id AND x.world_id = r.world_id
                 JOIN game_item_instances i ON i.item_instance_id = x.item_instance_id
                 WHERE r.account_id = $1 AND r.character_id = $2 AND r.world_id = $3
                   AND r.item_instance_id IS NOT NULL AND c.account_id = r.account_id AND i.lifecycle = 1
                   AND ($4::uuid IS NULL OR (r.fulfilment_unit_id, r.operation_id) > (
                     SELECT p.fulfilment_unit_id, p.operation_id FROM store_delivery_receipts p
                     WHERE p.operation_id = $4 AND p.account_id = $1 AND p.character_id = $2
                       AND p.world_id = $3 AND p.item_instance_id IS NOT NULL
                   ))
                   AND NOT EXISTS (
                     SELECT 1 FROM game_store_restore_conflicts f
                     WHERE f.delivery_operation_id = r.operation_id OR f.operation_id = r.operation_id
                   )
                 ORDER BY r.fulfilment_unit_id, r.operation_id LIMIT $5",
            )
            .bind(sqlx::types::Uuid::from_bytes(account))
            .bind(sqlx::types::Uuid::from_bytes(character))
            .bind(sqlx::types::Uuid::from_bytes(world))
            .bind(query.after.map(sqlx::types::Uuid::from_bytes))
            .bind(fetch_limit)
            .fetch_all(&mut **holder)
            .await?)
        }))
        .await
        .ok()?;
    // The receipt survives a claim, so its position still works if the item leaves the Inbox.
    let next_cursor = (rows.len() > page_size).then(|| *rows[page_size - 1].0.as_bytes());
    Some(wire::StoreInboxPage {
        entries: rows
            .into_iter()
            .take(page_size)
            .filter_map(|(_, entitlement, definition)| {
                Some(wire::StoreInboxEntry {
                    fulfilment_unit_id: uuid_bytes(&entitlement)?,
                    item_id: definition
                        .strip_prefix("oteryn:item.tibia.i")?
                        .parse()
                        .ok()?,
                })
            })
            .collect(),
        next_cursor,
    })
}
