use super::stacking::{plan_stack, stack_maximum};
use crate::item_transfer::{GAMEITEM01_STACK_QUANTITY_MAX, ItemTransferRefusal};
use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;
use oteryn_test_support::property::for_each_case;

#[test]
fn property_stack_split_and_merge_conserve_count_and_clamp_to_room() {
    for_each_case("stack conservation", |rng, _| {
        let maximum = rng.below(101) as u32;
        let receiver = rng.below(u64::from(maximum) + 1) as u32;
        for quantity in [0, 1, maximum, rng.u64() as u32, u32::MAX] {
            for receiver in [0, receiver, maximum, maximum.saturating_add(1), u32::MAX] {
                let (moved, remaining) = plan_stack(quantity, maximum, receiver);
                assert_eq!(u64::from(moved) + u64::from(remaining), u64::from(quantity));
                if receiver <= maximum {
                    assert!(receiver + moved <= maximum);
                    assert!(remaining == 0 || receiver + moved == maximum);
                    let (merged, rest) = plan_stack(moved, maximum, receiver);
                    assert_eq!((merged, rest), (moved, 0));
                } else {
                    assert_eq!((moved, remaining), (0, quantity));
                }
            }
        }
        Ok(())
    });
}

#[test]
fn property_stack_declarations_reject_invalid_limits_instead_of_clamping() {
    for_each_case("stack declarations", |rng, _| {
        for maximum in [0, 1, 100, 101, u32::MAX, rng.u64() as u32] {
            let result = stack_maximum(ItemStackClass::Stackable {
                proven_maximum: Some(maximum),
            });
            if (1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&maximum) {
                assert_eq!(result, Ok((maximum, true)));
            } else {
                assert_eq!(result, Err(ItemTransferRefusal::UnsupportedStackMaximum));
            }
        }
        assert_eq!(stack_maximum(ItemStackClass::NonStackable), Ok((1, false)));
        assert_eq!(
            stack_maximum(ItemStackClass::Unknown),
            Err(ItemTransferRefusal::UnknownStackClass)
        );
        assert_eq!(
            stack_maximum(ItemStackClass::Stackable {
                proven_maximum: None
            }),
            Ok((100, true))
        );
        Ok(())
    });
}
