//! Pure source Item classification from the qualified Content policy.

/// WORLD-INTERACTION-0 §8.3: the source's normal/safe barrier distinction
/// overrides the generic Item profile in every collision consumer.
pub fn source_barrier_collision(
    policy: oteryn_gameplay_content::content::native_gameplay::QualifiedItemPolicy<'_>,
) -> Option<(bool, bool)> {
    match policy
        .record()
        .production_binding
        .external_id
        .parse::<u32>()
        .ok()?
    {
        2128 => Some((true, true)),
        2130 => Some((true, false)),
        10181 => Some((false, true)),
        10182 => Some((false, false)),
        _ => None,
    }
}

pub fn source_magic_field(
    policy: oteryn_gameplay_content::content::native_gameplay::QualifiedItemPolicy<'_>,
) -> Result<bool, &'static str> {
    match &policy.record().semantics.classification {
        oteryn_gameplay_content::content::ReferenceItemField::Known(c) => match c.item_type {
            oteryn_gameplay_content::content::ReferenceItemField::Known(t) => {
                Ok(t == oteryn_gameplay_content::content::ReferenceItemType::MagicField)
            }
            oteryn_gameplay_content::content::ReferenceItemField::NotApplicable => Ok(false),
            _ => Err("unknown source Item classification"),
        },
        _ => Err("unknown source Item classification"),
    }
}
