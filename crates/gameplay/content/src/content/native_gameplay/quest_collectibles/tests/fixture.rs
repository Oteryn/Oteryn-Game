//! Unit-only fallback policy; the production packet may legitimately be empty.
use super::*;

pub(super) fn reviewed_entry() -> Result<Entry, ContentError> {
    let document: ItemProfilesDocument = serde_json::from_slice(include_bytes!(
        "../../../../../../../../content/items/definitions/spell-native-profiles.json"
    ))
    .map_err(|_| invalid("collectible fixture provider"))?;
    let mut profile = document
        .records
        .into_iter()
        .find(|r| r.production_definition.production_key == "oteryn:item.tibia.i44179")
        .ok_or_else(|| invalid("collectible fixture canonical item"))?;
    profile.production_binding.source_key = "oteryn:source.oteryn".into();
    profile.production_binding.source_revision = "quest-collectibles-v1".into();
    profile.production_binding.identity_namespace = "oteryn/quest_collectible_id".into();
    profile.production_binding_qualification = None;
    profile.semantics.container = F::NotApplicable;
    profile.semantics.charges = F::NotApplicable;
    profile.semantics.temporal = F::NotApplicable;
    profile.semantics.equipment = F::Unknown;
    profile.semantics.weapon = F::Unknown;
    profile.semantics.fluid = F::Unknown;
    profile.semantics.use_transform = F::Unknown;
    profile.semantics.imbuement = F::Unknown;
    Ok(Entry {
        profile,
        witnesses: vec![
            "fixture:served-client".into(),
            "fixture:canonical-item".into(),
            "fixture:bounded-private-namespace".into(),
        ],
        assumption: "Unit-only fallback; live canonical admission retires the production entry."
            .into(),
    })
}
