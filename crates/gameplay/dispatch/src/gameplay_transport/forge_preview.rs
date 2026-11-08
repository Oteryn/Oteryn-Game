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
    if request.selection == wire::Selection::Resource(Operation::DustLimit) {
        return limit_review(owner, admitted, command_id, request.nonce).await;
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
        wire::Selection::Browse { .. } | wire::Selection::Resource(_) => None,
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
        wire::Selection::Resource(operation) => resource_quote(snapshot.resources, operation)
            .map_or(wire::Payload::Refused, wire::Payload::Resource),
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
// Raising the limit has never required a backpack. Reuse command668's current balance owner,
// and explicitly omit unrelated inventory counts rather than fabricating zero balances.
async fn limit_review(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    nonce: u64,
) -> Outcome {
    let (Some(fence), Ok(command)) = (admitted.item_fence, CommandId::new(command)) else {
        return Outcome::Rejected;
    };
    let value = match owner
        .root
        .read_current_forge_dust(
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(admitted.game_session_id, command),
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
    let dust = value
        .get("current_dust")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());
    let limit = value
        .get("dust_limit")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());
    let (Some(dust), Some(dust_limit)) = (dust, limit) else {
        return Outcome::Unavailable;
    };
    let payload = resource_quote(
        wire::Resources {
            dust,
            dust_limit,
            ..Default::default()
        },
        Operation::DustLimit,
    )
    .map_or(wire::Payload::Refused, |mut quote| {
        quote.full_resources = false;
        wire::Payload::Resource(quote)
    });
    wire::Reply { nonce, payload }
        .encode()
        .map_or(Outcome::Unavailable, Outcome::Payload)
}
/// Pure projection of the exact current conversion/limit rules; never writes or reserves.
pub(super) fn resource_quote(
    before: wire::Resources,
    operation: Operation,
) -> Option<wire::ResourceQuote> {
    let mut quote = wire::ResourceQuote {
        operation,
        full_resources: true,
        before,
        after_dust: before.dust,
        after_limit: before.dust_limit,
        after_slivers: before.slivers,
        after_cores: before.cores,
    };
    match operation {
        Operation::DustToSlivers | Operation::SliversToCores => {
            let resources = crate::domain::forge::ForgeDustResources {
                dust: u64::from(before.dust),
                slivers: before.slivers,
                cores: before.cores,
            };
            let after = if operation == Operation::DustToSlivers {
                resources.dust_to_slivers()
            } else {
                resources.slivers_to_core()
            }
            .ok()?;
            quote.after_dust = u32::try_from(after.dust).ok()?;
            quote.after_slivers = after.slivers;
            quote.after_cores = after.cores;
        }
        Operation::DustLimit => {
            use crate::domain::forge_dust::{DustLimit, ForgeDustBalance};
            let before = ForgeDustBalance::new(
                u16::try_from(before.dust).ok()?,
                DustLimit::new(u16::try_from(before.dust_limit).ok()?).ok()?,
            )
            .ok()?;
            let plan = before.raise_limit().ok()?;
            quote.after_dust = u32::from(plan.after.balance());
            quote.after_limit = u32::from(plan.after.limit().get());
        }
        _ => return None,
    }
    Some(quote)
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
#[cfg(test)]
mod resource_tests {
    use super::*;
    #[test]
    fn forge_resource_quote_uses_actual_conversion_and_limit_rules() -> Result<(), &'static str> {
        let before = wire::Resources {
            dust: 100,
            dust_limit: 100,
            slivers: 50,
            cores: 2,
            ..Default::default()
        };
        let dust = resource_quote(before, Operation::DustToSlivers).ok_or("dust")?;
        assert_eq!(
            (dust.after_dust, dust.after_slivers, dust.after_cores),
            (40, 53, 2)
        );
        let cores = resource_quote(before, Operation::SliversToCores).ok_or("core")?;
        assert_eq!(
            (cores.after_dust, cores.after_slivers, cores.after_cores),
            (100, 0, 3)
        );
        let limit = resource_quote(before, Operation::DustLimit).ok_or("limit")?;
        assert_eq!((limit.after_dust, limit.after_limit), (75, 120));
        assert!(
            resource_quote(
                wire::Resources { dust: 0, ..before },
                Operation::DustToSlivers
            )
            .is_none()
        );
        assert!(
            resource_quote(
                wire::Resources {
                    slivers: 49,
                    ..before
                },
                Operation::SliversToCores
            )
            .is_none()
        );
        assert!(
            resource_quote(
                wire::Resources {
                    dust: 4600,
                    dust_limit: 4600,
                    ..before
                },
                Operation::DustLimit
            )
            .is_none()
        );
        Ok(())
    }
}
