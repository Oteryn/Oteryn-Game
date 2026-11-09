//! Read-only Forge projection; paid commands remain exclusively in item_forge.
use super::{
    ComposedFreshAdmission,
    connection::AdmittedSession,
    item_view::{ItemKey, SessionItemView},
};
use crate::durability::character_forge_dust::{
    ForgeDustLimitError, ForgePreviewItem, ForgePreviewSelection, ForgeResolvedItems,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_items::forge::ForgeOperation;
use oteryn_protocol_oteryn::{forge_preview as wire, item_forge::Operation};
#[derive(Debug)]
pub enum Outcome {
    Payload(Vec<u8>),
    Rejected,
    Unavailable,
}
fn operation(selection: wire::Selection) -> Option<ForgeOperation> {
    let wire::Selection::Pair {
        operation,
        boost,
        protect,
        ..
    } = selection
    else {
        return None;
    };
    Some(match operation {
        Operation::Fusion => ForgeOperation::Fusion { boost, protect },
        Operation::ConvergenceFusion => ForgeOperation::ConvergenceFusion,
        Operation::Transfer => ForgeOperation::Transfer,
        Operation::ConvergenceTransfer => ForgeOperation::ConvergenceTransfer,
        _ => return None,
    })
}
fn shown(item: &ForgePreviewItem, views: &SessionItemView) -> Option<wire::Item> {
    let handle = views.table().handle(&ItemKey::Instance(item.instance))?;
    // Use the existing accepted world appearance binding; never treat compact refs as sprites.
    let appearance = u32::from(crate::map::view::appearance_id(
        &item.definition.production_key,
        None,
    ));
    Some(wire::Item {
        handle,
        definition_ref: item.definition_ref,
        appearance,
        name: item.name.clone(),
        classification: item.classification,
        tier: item.tier,
        maximum_tier: item.maximum_tier,
        container: item.container,
    })
}
async fn current(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession) -> bool {
    let runtime = owner.runtime.lock().await;
    super::battle_instinct_status::actor_is_current(&runtime, admitted)
        && super::item_ref_admission::pinned_index(&runtime, owner.active_generation).is_some()
}
pub(super) async fn read(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: wire::Request,
    views: Option<&SessionItemView>,
) -> Outcome {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_FORGE_PREVIEW_V1)
        || !current(owner, admitted).await
    {
        return Outcome::Rejected;
    }
    let (Some(fence), Ok(command), Some(views), Some(native)) = (
        admitted.item_fence,
        CommandId::new(command_id),
        views,
        owner
            .active_generation
            .and_then(|active| active.native_gameplay()),
    ) else {
        return Outcome::Rejected;
    };
    let selection = match request.selection {
        wire::Selection::Browse { .. } => None,
        pair @ wire::Selection::Pair { first, second, .. } => {
            let (Some(ItemKey::Instance(first)), Some(ItemKey::Instance(second))) =
                (views.resolve(first), views.resolve(second))
            else {
                return Outcome::Rejected;
            };
            let Some(operation) = operation(pair) else {
                return Outcome::Rejected;
            };
            Some(ForgePreviewSelection {
                items: ForgeResolvedItems { first, second },
                operation,
            })
        }
    };
    let snapshot = match owner
        .root
        .read_forge_preview(
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(admitted.game_session_id, command),
            native,
            selection,
        )
        .await
    {
        Ok(value) => value,
        Err(ForgeDustLimitError::Unavailable(_)) => return Outcome::Unavailable,
        Err(_) => return Outcome::Rejected,
    };
    if !current(owner, admitted).await {
        return Outcome::Rejected;
    }
    let payload = match request.selection {
        wire::Selection::Browse { after } => {
            let mut items = snapshot
                .items
                .iter()
                .filter_map(|item| shown(item, views))
                .filter(|item| item.handle.get() > after)
                .collect::<Vec<_>>();
            items.sort_by_key(|item| item.handle);
            let more = items.len() > wire::PAGE_SIZE;
            items.truncate(wire::PAGE_SIZE);
            let next = if more {
                items.last().map(|item| item.handle.get())
            } else {
                None
            };
            wire::Payload::Page(wire::Page {
                after,
                next,
                items,
                resources: snapshot.resources,
            })
        }
        pair @ wire::Selection::Pair { first, second, .. } => {
            let first = snapshot
                .items
                .iter()
                .filter_map(|item| shown(item, views))
                .find(|item| item.handle == first);
            let second = snapshot
                .items
                .iter()
                .filter_map(|item| shown(item, views))
                .find(|item| item.handle == second);
            match (first, second, snapshot.quote) {
                (Some(first), Some(second), Some(quote)) => wire::Payload::Quote(wire::Quote {
                    selection: pair,
                    first,
                    second,
                    resources: snapshot.resources,
                    cost: quote.cost,
                    target_tier: quote.target_tier,
                    success_percent: quote.success_percent,
                    protection_percent: quote.protection_percent,
                }),
                _ => wire::Payload::Refused,
            }
        }
    };
    wire::Reply {
        nonce: request.nonce,
        payload,
    }
    .encode()
    .map_or(Outcome::Unavailable, Outcome::Payload)
}
macro_rules! authority_methods {
    () => {
        fn forge_preview_available(&self) -> bool {
            false
        }
        fn forge_preview(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command: u64,
            _request: oteryn_protocol_oteryn::forge_preview::Request,
            _views: Option<&$crate::gameplay_transport::item_view::SessionItemView>,
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::forge_preview::Outcome> {
            async { $crate::gameplay_transport::forge_preview::Outcome::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn forge_preview_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
        async fn forge_preview(
            &self,
            admitted: connection::AdmittedSession,
            command: u64,
            request: oteryn_protocol_oteryn::forge_preview::Request,
            views: Option<&item_view::SessionItemView>,
        ) -> forge_preview::Outcome {
            forge_preview::read(self, admitted, command, request, views).await
        }
    };
}
pub(super) use composed_methods;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forge_preview_operation_mapping_preserves_core_flags() -> Result<(), &'static str> {
        let first = oteryn_protocol_oteryn::item_view::ItemHandle::new(1).ok_or("first")?;
        let second = oteryn_protocol_oteryn::item_view::ItemHandle::new(2).ok_or("second")?;
        assert_eq!(
            operation(wire::Selection::Pair {
                operation: Operation::Fusion,
                first,
                second,
                boost: true,
                protect: true
            }),
            Some(ForgeOperation::Fusion {
                boost: true,
                protect: true
            })
        );
        assert_eq!(
            operation(wire::Selection::Pair {
                operation: Operation::ConvergenceTransfer,
                first,
                second,
                boost: false,
                protect: false
            }),
            Some(ForgeOperation::ConvergenceTransfer)
        );
        Ok(())
    }
}
