//! Existing reference effect descriptions, bound to the real currently carried Forge instance.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
pub(super) fn effects(
    items: &[ForgePreviewItem],
    views: &SessionItemView,
    native: &NativeGameplayState,
    handle: oteryn_protocol_oteryn::item_view::ItemHandle,
) -> Option<wire::Effects> {
    let ItemKey::Instance(instance) = views.resolve(handle)? else {
        return None;
    };
    let item = items
        .iter()
        .find(|i| i.instance == instance && !i.container)?;
    let policy = native.item_policy(
        &item.definition.production_key,
        &item.definition.revision_ref,
    )?;
    let tiers = (0..=item.maximum_tier)
        .map(|tier| super::super::inspection::tier_description(policy.record(), tier))
        .collect();
    Some(wire::Effects {
        item: shown(item, views)?,
        tiers,
    })
}
