//! Pure map-authored item pickup eligibility.

/// Why an entry stays in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ineligible {
    /// The ordinal names no top-level entry of the tile.
    NotTopLevel,
    /// The entry's item is not pickupable.
    NotPickupable,
    House,
    Action,
    Unique,
    Door,
    Depot,
    Teleport,
    /// The bundle dropped the entry's zero-destination teleport (ADR-0021 §4.5).
    DroppedTeleport,
    /// The entry holds items one level deeper.
    Contents,
    Text,
    Description,
    Charges,
    /// The entry's count is zero or above the stack quantity maximum.
    Quantity,
}

/// Decoded top-level bundle entry facts, with Content pickupability supplied by the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickupFacts {
    pub pickupable: bool,
    pub house: bool,
    pub action: bool,
    pub unique: bool,
    pub door: bool,
    pub depot: bool,
    pub teleport: bool,
    pub dropped_teleport: bool,
    pub contents: bool,
    pub text: bool,
    pub description: bool,
    pub charges: bool,
    pub quantity: u32,
}

/// Whether a top-level entry's decoded facts permit pickup of its whole quantity.
pub fn eligibility(facts: PickupFacts) -> Result<(), Ineligible> {
    let refused = [
        (!facts.pickupable, Ineligible::NotPickupable),
        (facts.house, Ineligible::House),
        (facts.action, Ineligible::Action),
        (facts.unique, Ineligible::Unique),
        (facts.door, Ineligible::Door),
        (facts.depot, Ineligible::Depot),
        (facts.teleport, Ineligible::Teleport),
        (facts.dropped_teleport, Ineligible::DroppedTeleport),
        (facts.contents, Ineligible::Contents),
        (facts.text, Ineligible::Text),
        (facts.description, Ineligible::Description),
        (facts.charges, Ineligible::Charges),
    ];
    if let Some((_, reason)) = refused.into_iter().find(|(refused, _)| *refused) {
        return Err(reason);
    }
    if !(1..=crate::item_transfer::GAMEITEM01_STACK_QUANTITY_MAX).contains(&facts.quantity) {
        return Err(Ineligible::Quantity);
    }
    Ok(())
}
