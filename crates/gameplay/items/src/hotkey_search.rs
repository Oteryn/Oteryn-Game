//! BAGS-0 §8: deterministic breadth-first hotkey selection, including closed bags.

use std::collections::VecDeque;

/// `BAGS0-RL-07`: at most 509 carried items inspected by one hotkey search.
pub const HOTKEY_ITEMS_MAX: usize = 509;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyRefusal {
    NothingFound,
    BoundExceeded,
    /// ITEM-USE-0/DUR-03 do not yet admit consuming an equipped item. Never fall through.
    EquipmentSource,
}

/// Select the first matching unreserved stack in wire/display order. `children` returns direct
/// entries in display order regardless of open views. The complete input must fit the bound:
/// even an early match cannot turn an oversized tree into a truncated successful search.
pub fn first_unreserved<'a, T: Copy, D: PartialEq + 'a>(
    equipment: impl IntoIterator<Item = T>,
    main_entries: impl IntoIterator<Item = T>,
    children: impl Fn(T) -> Vec<T>,
    definition: impl Fn(T) -> &'a D,
    wanted: &D,
    reserved: impl Fn(T) -> bool,
) -> Result<T, HotkeyRefusal> {
    let mut queue = VecDeque::new();
    for (item, equipped) in equipment
        .into_iter()
        .map(|item| (item, true))
        .chain(main_entries.into_iter().map(|item| (item, false)))
    {
        if queue.len() == HOTKEY_ITEMS_MAX {
            return Err(HotkeyRefusal::BoundExceeded);
        }
        queue.push_back((item, equipped));
    }
    let mut inspected = 0_usize;
    let mut first = None;
    while let Some((item, equipped)) = queue.pop_front() {
        inspected += 1;
        if first.is_none() && definition(item) == wanted && !reserved(item) {
            first = Some(if equipped {
                Err(HotkeyRefusal::EquipmentSource)
            } else {
                Ok(item)
            });
        }
        if !equipped {
            let entries = children(item);
            if inspected
                .saturating_add(queue.len())
                .saturating_add(entries.len())
                > HOTKEY_ITEMS_MAX
            {
                return Err(HotkeyRefusal::BoundExceeded);
            }
            queue.extend(entries.into_iter().map(|entry| (entry, false)));
        }
    }
    first.unwrap_or(Err(HotkeyRefusal::NothingFound))
}

#[cfg(test)]
mod hotkey_search_tests;
