//! Read-only ITEM-LOOK-1 text proposal; no target lookup, wire IDs or runtime activation.

use crate::content::{
    REFERENCE_ITEM_MAX_DESCRIPTION_BYTES, REFERENCE_ITEM_MAX_NAME_BYTES, ReferenceItemField,
    ReferenceItemSemantics,
};
use std::num::NonZeroU32;

/// Only the qualified active-generation record may supply `semantics` in production.
/// Identity, visibility and admission are the caller's responsibility. Unknown and
/// conflicting descriptions are omitted; a missing name cannot produce examine text.
/// Article, pluralisation, weight and readable-instance text await owning contracts.
pub fn item_text(semantics: &ReferenceItemSemantics, count: NonZeroU32) -> Option<String> {
    let ReferenceItemField::Known(presentation) = &semantics.presentation else {
        return None;
    };
    let ReferenceItemField::Known(name) = &presentation.name else {
        return None;
    };
    if name.trim().is_empty() || name.len() > REFERENCE_ITEM_MAX_NAME_BYTES {
        return None;
    }
    let mut text = format!("You see {name}.\nCount: {count}.");
    if let ReferenceItemField::Known(description) = &presentation.description {
        if description.len() > REFERENCE_ITEM_MAX_DESCRIPTION_BYTES {
            return None;
        }
        if !description.is_empty() {
            text.push('\n');
            text.push_str(description);
        }
    }
    Some(text)
}
