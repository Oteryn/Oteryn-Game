//! Project the admitted buyer's unclaimed Store Inbox items.
use super::{ComposedFreshAdmission, gameplay_fence, uuid_bytes, wire};
use crate::gameplay_transport::connection::AdmittedSession;

pub(super) async fn project(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    after: Option<[u8; 16]>,
) -> Option<wire::StoreInboxPage> {
    owner.store.as_ref()?;
    let account = admitted.controller?.account_id;
    let fence = gameplay_fence(owner, admitted).await?;
    let character = *fence.character_id.as_bytes();
    let world = *admitted.world_id.as_bytes();
    let limit = i64::try_from(wire::MAX_INBOX_ENTRIES + 1).ok()?;
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
                   AND NOT EXISTS (
                     SELECT 1 FROM game_store_restore_conflicts f
                     WHERE f.delivery_operation_id = r.operation_id OR f.operation_id = r.operation_id
                   )
                   AND ($4::uuid IS NULL OR (r.fulfilment_unit_id, r.operation_id) >
                     (SELECT fulfilment_unit_id, operation_id FROM store_delivery_receipts
                      WHERE operation_id = $4 AND account_id = $1 AND character_id = $2 AND world_id = $3))
                 ORDER BY r.fulfilment_unit_id, r.operation_id LIMIT $5",
            )
            .bind(sqlx::types::Uuid::from_bytes(account))
            .bind(sqlx::types::Uuid::from_bytes(character))
            .bind(sqlx::types::Uuid::from_bytes(world))
            .bind(after.map(sqlx::types::Uuid::from_bytes))
            .bind(limit)
            .fetch_all(&mut **holder)
            .await?)
        }))
        .await
        .ok()?;
    let next = (rows.len() > wire::MAX_INBOX_ENTRIES)
        .then(|| *rows[wire::MAX_INBOX_ENTRIES - 1].0.as_bytes());
    let claims_require_channel = !matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }
    );
    let mut page = wire::StoreInboxPage {
        next,
        claims_require_channel,
        ..Default::default()
    };
    let lamp_key = crate::durability::house_hireling::HIRELING_LAMP_KEY;
    let mut has_lamp = false;
    for (_, entitlement, definition) in rows.into_iter().take(wire::MAX_INBOX_ENTRIES) {
        if definition == lamp_key {
            has_lamp = true;
            continue;
        }
        if let (Some(fulfilment_unit_id), Some(item_id)) = (
            uuid_bytes(&entitlement),
            definition.strip_prefix("oteryn:item.tibia.i").and_then(|id| id.parse().ok()),
        ) {
            page.entries.push(wire::StoreInboxEntry { fulfilment_unit_id, item_id });
        }
    }
    if has_lamp
        && claims_require_channel
        && admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
        )
        && let Some(actor) = admitted.runtime_actor
    {
        let runtime = owner.runtime.lock().await;
        if runtime.house_actor_state(actor, admitted.game_session_id).is_ok()
            && let Some(index) = super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation)
            && let Some(revision) = index.revision(lamp_key)
        {
            page.lamp_definition = index.definition_ref("Item", lamp_key, revision);
        }
    }
    Some(page)
}
