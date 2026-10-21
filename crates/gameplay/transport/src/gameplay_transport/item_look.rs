//! Unwired ITEM-LOOK-1 read-only preparation. This is not a production handler.
//!
//! A future owner caller must refresh the session's views and observe the target in
//! the same current authority/visibility boundary before calling this helper. Never
//! create an observation from client facts or a cached view alone. The helper checks
//! binding; it does not acquire locks or independently establish fresh world visibility.

use super::item_ref_admission::pinned_index;
use super::item_view::{SessionItemView, ViewItem};
use crate::content::{ActiveGeneration, native_gameplay::QualifiedItemPolicy};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::ChannelRuntimeV1;
use oteryn_protocol_oteryn::item_view::ItemHandle;

// The examine text is mounted here, not in `interaction/mod.rs`: the test targets that recompile
// `interaction/mod.rs` carry no `crate::content` (the `item_view::corpse_open` pattern).
#[path = "look.rs"]
mod text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemLookOutcome {
    Text(String),
    /// The same refusal for hidden, missing, stale, mismatched or unadmitted targets.
    Unavailable,
}

/// Fresh facts from the item owner; `item.key`, count and definition must belong to
/// the same observation. No instance identities or definition keys go on the wire.
pub struct ObservedLookItem<'a> {
    pub item: ViewItem,
    pub definition: &'a TypedDefinitionRef,
}

pub fn examine_observed_item(
    runtime: &ChannelRuntimeV1,
    generation: Option<&ActiveGeneration>,
    views: &SessionItemView,
    target: ItemHandle,
    observed: Option<ObservedLookItem<'_>>,
) -> ItemLookOutcome {
    // A surviving content pin does not override locally known owner retirement.
    // This denial is not proof of current DB/session authority or fresh visibility.
    if runtime.owner_fence().is_err() {
        return ItemLookOutcome::Unavailable;
    }
    let Some(key) = views.resolve(target) else {
        return ItemLookOutcome::Unavailable;
    };
    let Some(observed) = observed.filter(|observed| observed.item.key == key) else {
        return ItemLookOutcome::Unavailable;
    };
    let Some(generation) = generation else {
        return ItemLookOutcome::Unavailable;
    };
    let pin = runtime.content_pin();
    if pin.world_id() != generation.identity().world_id()
        || pin.activation_sequence() != generation.activation_sequence()
    {
        return ItemLookOutcome::Unavailable;
    }
    let Some(index) = pinned_index(runtime, Some(generation)) else {
        return ItemLookOutcome::Unavailable;
    };
    let definition = observed.definition;
    if index.definition_ref(
        &definition.family,
        &definition.production_key,
        &definition.revision_ref,
    ) != Some(observed.item.item_definition_ref)
    {
        return ItemLookOutcome::Unavailable;
    }
    let Some(policy) = generation.native_gameplay().and_then(|native| {
        native.item_policy(&definition.production_key, &definition.revision_ref)
    }) else {
        return ItemLookOutcome::Unavailable;
    };
    examine_qualified_item(policy, observed)
}

fn examine_qualified_item(
    policy: QualifiedItemPolicy<'_>,
    observed: ObservedLookItem<'_>,
) -> ItemLookOutcome {
    let record = policy.record();
    let expected = observed.definition;
    let actual = &record.production_definition;
    if !record.admission.materializable
        || actual.family != expected.family
        || actual.production_key != expected.production_key
        || actual.revision_ref != expected.revision_ref
    {
        return ItemLookOutcome::Unavailable;
    }
    text::item_text(&record.semantics, observed.item.count)
        .map(ItemLookOutcome::Text)
        .unwrap_or(ItemLookOutcome::Unavailable)
}

#[cfg(test)]
#[path = "item_look_tests.rs"]
mod tests;
