// Child of existing item_look owner. Metadata is derived from immutable source
// receipts; current item availability stays with the caller's native observation.
use super::{ComposedFreshAdmission, ItemLookRefusal};

pub(super) async fn decoration_kit_description(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    item: [u8; 16],
) -> Result<Option<String>, ItemLookRefusal> {
    use crate::content::ReferenceItemField as F;
    let unavailable = ItemLookRefusal::NotAvailable;
    if let Some(source) = admission
        .root
        .furniture_kit_description_source(admission.character, item)
        .await
        .map_err(|_| unavailable)?
    {
        let target = &source.target;
        let native = admission
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(unavailable)?;
        let policy = native
            .item_policy(&target.production_key, &target.revision_ref)
            .ok_or(unavailable)?;
        let F::Known(presentation) = &policy.record().semantics.presentation else {
            return Err(unavailable);
        };
        let F::Known(name) = &presentation.name else {
            return Err(unavailable);
        };
        if name.trim().is_empty() {
            return Err(unavailable);
        }
        // Crystal201189 src/game/game.cpp4900 (Canary's Store branding differs).
        let subject = match source.kind {
            crate::durability::house_wrap::FurnitureKitDescriptionKind::Wrapped => "it",
            crate::durability::house_wrap::FurnitureKitDescriptionKind::Purchased => "this item",
        };
        return Ok(Some(format!(
            "Unwrap {subject} in your own house to create a <{name}>."
        )));
    }
    admission
        .root
        .royal_furniture_kit_description(admission.character, item)
        .await
        .map_err(|_| unavailable)
}
