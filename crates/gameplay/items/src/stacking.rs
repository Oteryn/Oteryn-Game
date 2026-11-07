//! Shared stack limits and merge planning.

use crate::item_transfer::{GAMEITEM01_STACK_QUANTITY_MAX, ItemTransferRefusal};
use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;

/// D82: the stack maximum of a definition, never above 100.
pub fn stack_maximum(
    stack: ItemStackClass,
) -> std::result::Result<(u32, bool), ItemTransferRefusal> {
    match stack {
        ItemStackClass::NonStackable => Ok((1, false)),
        ItemStackClass::Stackable {
            proven_maximum: None,
        } => Ok((GAMEITEM01_STACK_QUANTITY_MAX, true)),
        ItemStackClass::Stackable {
            proven_maximum: Some(maximum),
        } if (1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&maximum) => Ok((maximum, true)),
        ItemStackClass::Stackable { .. } => Err(ItemTransferRefusal::UnsupportedStackMaximum),
        ItemStackClass::Unknown => Err(ItemTransferRefusal::UnknownStackClass),
    }
}

/// Plan one top-up and the quantity still needing placement or another receiver.
/// Owners validate definition, stack class and live quantities before calling; an
/// already full receiver has no room. Splitting a fresh stack uses a receiver of zero.
pub fn plan_stack(quantity: u32, maximum: u32, receiver_quantity: u32) -> (u32, u32) {
    let moved = quantity.min(maximum.saturating_sub(receiver_quantity));
    (moved, quantity - moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topups_and_splits_conserve_the_whole_quantity() {
        for maximum in [1, 25, 100] {
            for before in 0..=maximum {
                for quantity in [0, 1, 25, 100, 50_000, u32::MAX] {
                    let (moved, remaining) = plan_stack(quantity, maximum, before);
                    assert_eq!(u64::from(moved) + u64::from(remaining), u64::from(quantity));
                    assert!(before + moved <= maximum);
                    assert!(remaining == 0 || before + moved == maximum);
                }
            }
        }
        assert_eq!(plan_stack(7, 100, 101), (0, 7));
    }
}
