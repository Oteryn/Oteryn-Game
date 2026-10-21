//! Pure source Item classification from the qualified Content policy.

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
