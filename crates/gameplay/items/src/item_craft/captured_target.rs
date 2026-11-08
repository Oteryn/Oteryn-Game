//! Quantity capture for the source-qualified Tainted Souls transformation.

/// Crystal transforms the selected antler object, then removes one used herb.
/// A stackable target becomes one non-stackable talisman; capture its whole count.
pub const TAINTED_SOULS_RECIPE: &str = "oteryn:item-conversion.tainted-souls-antler-talisman";

pub fn captured_target_quantity(recipe: &str, quantity: u32) -> Option<u32> {
    if !(1..=100).contains(&quantity) {
        return None;
    }
    Some(if recipe == TAINTED_SOULS_RECIPE {
        quantity
    } else {
        1
    })
}

#[cfg(test)]
mod captured_target_tests {
    use super::*;

    #[test]
    fn tainted_souls_captures_the_entire_bounded_target_stack() {
        for quantity in [1, 37, 100] {
            assert_eq!(
                captured_target_quantity(TAINTED_SOULS_RECIPE, quantity),
                Some(quantity)
            );
        }
        for quantity in [0, 101, u32::MAX] {
            assert_eq!(
                captured_target_quantity(TAINTED_SOULS_RECIPE, quantity),
                None
            );
        }
    }

    #[test]
    fn ordinary_carried_recipes_still_consume_one_target_unit() {
        assert_eq!(
            captured_target_quantity("oteryn:item-conversion.flour", 37),
            Some(1)
        );
        assert_eq!(
            captured_target_quantity("oteryn:item-conversion.flour", 0),
            None
        );
    }
}
