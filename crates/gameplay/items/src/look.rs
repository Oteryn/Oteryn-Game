//! Read-only ITEM-LOOK-1 text proposal; no target lookup, wire IDs or runtime activation.

use oteryn_gameplay_content::content::{
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use oteryn_gameplay_content::content::{ReferenceItemField as F, ReferenceItemPresentation};
    fn semantics(name: F<String>, description: F<String>) -> ReferenceItemSemantics {
        ReferenceItemSemantics {
            presentation: F::Known(ReferenceItemPresentation { name, description }),
            ..Default::default()
        }
    }

    #[test]
    fn formatter_uses_known_text_and_authoritative_count() {
        let facts = semantics(
            F::Known("cheese".into()),
            F::Known("A cave-aged cheese.".into()),
        );
        assert_eq!(
            item_text(&facts, NonZeroU32::new(2).unwrap()),
            Some("You see cheese.\nCount: 2.\nA cave-aged cheese.".into())
        );
        for unknown in [F::Unknown, F::Conflict, F::NotApplicable] {
            assert_eq!(
                item_text(
                    &semantics(F::Known("cheese".into()), unknown.clone()),
                    NonZeroU32::MIN
                ),
                Some("You see cheese.\nCount: 1.".into())
            );
            assert_eq!(
                item_text(&semantics(unknown, F::Unknown), NonZeroU32::MIN),
                None
            );
        }
        assert_eq!(
            item_text(&ReferenceItemSemantics::default(), NonZeroU32::MIN),
            None
        );
    }

    #[test]
    fn formatter_preserves_existing_content_byte_limits() {
        use oteryn_gameplay_content::content::{
            REFERENCE_ITEM_MAX_DESCRIPTION_BYTES as DESCRIPTION,
            REFERENCE_ITEM_MAX_NAME_BYTES as NAME,
        };
        let valid = semantics(
            F::Known("é".repeat(NAME / 2)),
            F::Known("é".repeat(DESCRIPTION / 2)),
        );
        assert!(item_text(&valid, NonZeroU32::MAX).is_some());
        let bad_name = semantics(F::Known("é".repeat(NAME / 2 + 1)), F::Unknown);
        let bad_description = semantics(
            F::Known("cheese".into()),
            F::Known("é".repeat(DESCRIPTION / 2 + 1)),
        );
        for invalid in [
            bad_name,
            bad_description,
            semantics(F::Known(" ".into()), F::Unknown),
        ] {
            assert_eq!(item_text(&invalid, NonZeroU32::MIN), None);
        }
    }
}
