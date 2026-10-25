//! MARKET0 production Content identities mapped onto the owned durable Market model.
//! This conversion preserves typed definition identity, not marketability or admission.
#![allow(
    dead_code,
    reason = "private Market owner awaits atomic CP integration"
)]

/// Definition identity supplied by the admitted content producer.
pub trait MarketDefinitionIdentity {
    fn is_item_family(&self) -> bool;
    fn production_key(&self) -> &str;
    fn revision_ref(&self) -> &str;
}

pub fn definition_for_storage<T, E>(
    value: &impl MarketDefinitionIdentity,
    store: impl FnOnce(super::item_definition::TypedDefinitionRef) -> Result<T, E>,
    invalid: impl FnOnce() -> E,
) -> Result<T, E> {
    if !value.is_item_family() {
        return Err(invalid());
    }
    store(crate::domain::item_definition::TypedDefinitionRef {
        family: "Item".to_owned(),
        production_key: value.production_key().to_owned(),
        revision_ref: value.revision_ref().to_owned(),
    })
}
