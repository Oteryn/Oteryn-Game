//! Closed documented-template defaults; Item admission and stack maxima remain unchanged.
use super::{
    ProjectReferenceRecord, ProjectV2Draft,
    item_stack_false_promotion::{StackFalsePromotion, apply_rows},
    world_project_sha256,
};

pub const ITEM_STACK_DEFAULT_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-stack-default-promotion-v1.json"
);
pub const ITEM_STACK_DEFAULT_PACKET_SHA256: &str =
    "8c970ed74a22124b3081b1bb2c2768100e910b0f82dde39663fd1d531add4fa3";

pub fn apply_item_stack_default_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<StackFalsePromotion, String> {
    if world_project_sha256(ITEM_STACK_DEFAULT_PACKET) != ITEM_STACK_DEFAULT_PACKET_SHA256 {
        return Err("documented stack default packet digest drift".into());
    }
    let items = draft.core.records.iter_mut().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            stack_class,
            semantics,
            ..
        } => Some((identity.key.as_str(), *stack_class, semantics)),
        _ => None,
    });
    apply_rows(items, ITEM_STACK_DEFAULT_PACKET, 1487)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::{
        ItemStackDocument, ReferenceItemField, ReferenceItemSemantics, ReferenceItemStack,
    };

    #[test]
    fn derived_default_preserves_siblings_and_replays_idempotently() {
        let mut item = ReferenceItemSemantics {
            stack: ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Unknown,
                stack_max: ReferenceItemField::Known(7),
            }),
            ..ReferenceItemSemantics::default()
        };
        let bytes = br#"{"schema":"OTERYN_ITEM_STACK_FALSE_PROMOTION/v1","source_policy":"DERIVED_DOCUMENTED_TEMPLATE_DEFAULT","counts":{"promotions":1},"promotions":[{"item_key":"item","stackable":false}]}"#;
        for changed in [1, 0] {
            assert_eq!(
                apply_rows(
                    std::iter::once(("item", ItemStackDocument::Unknown, &mut item)),
                    bytes,
                    1
                ),
                Ok(StackFalsePromotion { fields: 1, changed })
            );
        }
        assert_eq!(
            item.stack,
            ReferenceItemField::Known(ReferenceItemStack {
                stackable: ReferenceItemField::Known(false),
                stack_max: ReferenceItemField::Known(7)
            })
        );
    }

    #[test]
    fn derived_default_rejects_non_false_atomically() {
        let mut item = ReferenceItemSemantics::default();
        let bytes = br#"{"schema":"OTERYN_ITEM_STACK_FALSE_PROMOTION/v1","counts":{"promotions":2},"promotions":[{"item_key":"item","stackable":false},{"item_key":"second","stackable":true}]}"#;
        let before = item.clone();
        assert!(
            apply_rows(
                std::iter::once(("item", ItemStackDocument::Unknown, &mut item)),
                bytes,
                2
            )
            .is_err()
        );
        assert_eq!(item, before);
    }
}
