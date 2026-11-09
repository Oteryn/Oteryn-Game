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

/// The native empty large amphora uses count zero for its empty fluid state.
/// Its ground push reuses every ordinary source-binding refusal.
pub fn empty_amphora_push_quantity(key: &str, facts: PickupFacts) -> Result<u32, Ineligible> {
    if key != "oteryn:item.tibia.i2904" {
        return Err(Ineligible::NotPickupable);
    }
    if facts.quantity != 0 {
        return Err(Ineligible::Quantity);
    }
    eligibility(PickupFacts {
        pickupable: true,
        quantity: 1,
        ..facts
    })?;
    Ok(1)
}

#[cfg(test)]
mod amphora_tests {
    use super::*;
    #[test]
    fn empty_amphora_push_preserves_bindings_and_refuses_filled_sources() {
        let empty = PickupFacts {
            pickupable: false,
            house: false,
            action: false,
            unique: false,
            door: false,
            depot: false,
            teleport: false,
            dropped_teleport: false,
            contents: false,
            text: false,
            description: false,
            charges: false,
            quantity: 0,
        };
        let key = "oteryn:item.tibia.i2904";
        assert_eq!(eligibility(empty), Err(Ineligible::NotPickupable));
        assert_eq!(empty_amphora_push_quantity(key, empty), Ok(1));
        assert_eq!(
            empty_amphora_push_quantity("oteryn:item.tibia.i2471", empty),
            Err(Ineligible::NotPickupable)
        );
        assert_eq!(
            empty_amphora_push_quantity(
                key,
                PickupFacts {
                    quantity: 1,
                    ..empty
                }
            ),
            Err(Ineligible::Quantity)
        );
        assert_eq!(
            empty_amphora_push_quantity(
                key,
                PickupFacts {
                    house: true,
                    ..empty
                }
            ),
            Err(Ineligible::House)
        );
        assert_eq!(
            empty_amphora_push_quantity(
                key,
                PickupFacts {
                    action: true,
                    ..empty
                }
            ),
            Err(Ineligible::Action)
        );
        assert_eq!(
            empty_amphora_push_quantity(
                key,
                PickupFacts {
                    contents: true,
                    ..empty
                }
            ),
            Err(Ineligible::Contents)
        );
    }
}
