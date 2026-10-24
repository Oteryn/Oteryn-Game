use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Item {
    id: usize,
    definition: u8,
}

fn search(
    slots: &[Item],
    main: &[Item],
    nested: &[(usize, Vec<Item>)],
    reserved: &[usize],
) -> Result<Item, HotkeyRefusal> {
    first_unreserved(
        slots,
        main,
        |item| {
            nested
                .iter()
                .find(|(id, _)| *id == item.id)
                .map(|(_, items)| items.iter().collect())
                .unwrap_or_default()
        },
        |item| &item.definition,
        &1,
        |item| reserved.contains(&item.id),
    )
    .copied()
}

#[test]
fn order_is_slots_then_direct_entries_then_each_level_in_display_order() {
    let item = |id, definition| Item { id, definition };
    let main = [item(1, 0), item(2, 0), item(3, 1)];
    let nested = [
        (1, vec![item(4, 0)]),
        (2, vec![item(5, 1)]),
        (4, vec![item(6, 1)]),
    ];
    assert_eq!(
        search(&[item(7, 1)], &main, &nested, &[]),
        Err(HotkeyRefusal::EquipmentSource)
    );
    assert_eq!(search(&[], &main, &nested, &[]), Ok(main[2]));
    assert_eq!(search(&[], &main[..2], &nested, &[]), Ok(item(5, 1)));
    assert_eq!(search(&[], &main[..2], &nested, &[5]), Ok(item(6, 1)));
}

#[test]
fn closed_bags_are_searched_and_reserved_stacks_are_skipped() {
    let main = [Item {
        id: 1,
        definition: 0,
    }];
    let nested = [(
        1,
        vec![
            Item {
                id: 2,
                definition: 1,
            },
            Item {
                id: 3,
                definition: 1,
            },
        ],
    )];
    assert_eq!(
        search(&[], &main, &nested, &[2]),
        Ok(Item {
            id: 3,
            definition: 1
        })
    );
    assert_eq!(
        search(&[], &main, &nested, &[2, 3]),
        Err(HotkeyRefusal::NothingFound)
    );
    assert_eq!(
        search(
            &[Item {
                id: 4,
                definition: 1
            }],
            &main,
            &nested,
            &[4, 2]
        ),
        Ok(Item {
            id: 3,
            definition: 1
        })
    );
}

#[test]
fn searches_509_and_refuses_510_even_with_an_early_match() {
    let items: Vec<_> = (0..510)
        .map(|id| Item {
            id,
            definition: u8::from(id == 508),
        })
        .collect();
    assert_eq!(search(&[], &items[..509], &[], &[]), Ok(items[508]));
    assert_eq!(
        search(&[], &items, &[], &[]),
        Err(HotkeyRefusal::BoundExceeded)
    );
    let mut early = items.clone();
    early[0].definition = 1;
    assert_eq!(
        search(&[], &early, &[], &[]),
        Err(HotkeyRefusal::BoundExceeded)
    );
    let nested = [(0, items[1..].to_vec())];
    assert_eq!(
        search(&[], &items[..1], &nested, &[]),
        Err(HotkeyRefusal::BoundExceeded)
    );
}

#[test]
fn the_same_input_always_selects_the_same_stack() {
    let main = [
        Item {
            id: 1,
            definition: 0,
        },
        Item {
            id: 2,
            definition: 0,
        },
    ];
    let nested = [
        (
            1,
            vec![Item {
                id: 3,
                definition: 1,
            }],
        ),
        (
            2,
            vec![Item {
                id: 4,
                definition: 1,
            }],
        ),
    ];
    for _ in 0..10 {
        assert_eq!(
            search(&[], &main, &nested, &[]),
            Ok(Item {
                id: 3,
                definition: 1
            })
        );
    }
}
