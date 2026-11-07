//! Existing Look30 text, after the real named-House carried-item proof.
//! Deliberately closed to23398; no origin Channel authority grants a House view.
use super::*;
use crate::durability::consume_transform::HouseActionContext;

pub(super) async fn look(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: Option<CurrentCharacterItemFence>,
    views: &SessionItemView,
    target: ItemHandle,
) -> Option<ItemLookResult> {
    let fence = fence?;
    let context = {
        let runtime = admission.runtime.lock().await;
        HouseActionContext::from_native_owner(&runtime, actor, session, fence).ok()?
    };
    Some(
        match observe(admission, actor, session, fence, &context, views, target).await {
            Ok(text) => ItemLookResult::Text(text),
            Err(refusal) => ItemLookResult::Refused(refusal),
        },
    )
}
async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
    context: &HouseActionContext,
    views: &SessionItemView,
    target: ItemHandle,
) -> Result<String, ItemLookRefusal> {
    let unavailable = ItemLookRefusal::NotAvailable;
    let Some(ItemKey::Instance(instance)) = views.resolve(target) else {
        return Err(unavailable);
    };
    let inventory = admission
        .root
        .read_hireling_use_inventory(
            admission.character,
            admission.holder,
            fence,
            *admission.channel_id.as_bytes(),
        )
        .await
        .map_err(|_| unavailable)?;
    let source = inventory
        .backpack
        .iter()
        .flat_map(|b| b.entries.iter().map(|e| &e.item))
        .chain(inventory.nested.iter().map(|(_, e)| &e.item))
        .find(|i| {
            i.item_instance_id == instance
                && i.quantity == 1
                && i.definition.production_key == "oteryn:item.tibia.i23398"
        })
        .ok_or(unavailable)?;
    let description = decoration_kit::decoration_kit_description(admission, instance).await?;
    if !admission
        .root
        .house_action_current(admission.holder, context)
        .await
        .map_err(|_| unavailable)?
    {
        return Err(unavailable);
    }
    let runtime = admission.runtime.lock().await;
    let current = HouseActionContext::from_native_owner(&runtime, actor, session, fence)
        .map_err(|_| unavailable)?;
    if current.house() != context.house() {
        return Err(unavailable);
    }
    let index = pinned_index(&runtime, admission.active_generation).ok_or(unavailable)?;
    let reference = index
        .definition_ref(
            &source.definition.family,
            &source.definition.production_key,
            &source.definition.revision_ref,
        )
        .ok_or(unavailable)?;
    let native = admission
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or(unavailable)?;
    let policy = native
        .item_policy(
            &source.definition.production_key,
            &source.definition.revision_ref,
        )
        .ok_or(unavailable)?;
    let observed = ObservedLookItem {
        item: ViewItem {
            key: ItemKey::Instance(instance),
            item_definition_ref: reference,
            count: NonZeroU32::MIN,
            sub_type: 0,
        },
        definition: &source.definition,
        look_distance: -1,
    };
    let ItemLookOutcome::Text(mut text) = examine_qualified_item(policy, observed) else {
        return Err(unavailable);
    };
    if let Some(description) = description {
        text.push('\n');
        text.push_str(&description)
    }
    Ok(bounded_text(text))
}
