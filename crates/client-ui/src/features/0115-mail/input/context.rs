use crate::world_interaction::menu::{ContextAction, Menu, context_entries};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Existing(ContextAction),
    Property(Action),
}
pub fn combined(
    world: Option<&Menu>,
    people: [bool; 3],
    handle: Option<u64>,
    selected: [bool; 4],
) -> Vec<(&'static str, Entry)> {
    let mut combined: Vec<_> = context_entries(world, people[0], people[1], people[2])
        .into_iter()
        .map(|(label, action)| (label, Entry::Existing(action)))
        .collect();
    combined.extend(
        entries(handle, selected[0], selected[1], selected[2], selected[3])
            .into_iter()
            .map(|(label, action)| (label, Entry::Property(action))),
    );
    combined
}
/// Property actions join the existing world/NPC/player popup for the clicked handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Letter(u64),
    Mailbox(u64),
    Offline(u64),
    Weapon(u64),
    Dummy(u64),
    Podium,
    GuildBank,
}
pub fn entries(
    handle: Option<u64>,
    mail: bool,
    training: bool,
    podium: bool,
    bank: bool,
) -> Vec<(&'static str, Action)> {
    let mut entries = Vec::new();
    if let Some(handle) = handle.filter(|handle| *handle != 0) {
        if mail {
            entries.extend([
                ("Write or post this item…", Action::Letter(handle)),
                ("Select as mailbox…", Action::Mailbox(handle)),
            ]);
        }
        if training {
            entries.extend([
                ("Select offline training statue…", Action::Offline(handle)),
                ("Select exercise weapon…", Action::Weapon(handle)),
                ("Select exercise dummy…", Action::Dummy(handle)),
            ]);
        }
    }
    if podium {
        entries.push(("House podium editor…", Action::Podium));
    }
    if bank {
        entries.push(("Guild bank…", Action::GuildBank));
    }
    entries
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_menu_preserves_world_npc_trade_party_actions_and_appends_properties_once() {
        use oteryn_client_runtime::{inventory::ItemHandle, world_interaction::Target};
        let menu = Menu::new(Target::Item(ItemHandle::MIN), true, false).with_targeting(true);
        let existing = context_entries(Some(&menu), true, true, true);
        let all = combined(Some(&menu), [true; 3], Some(7), [true; 4]);
        assert_eq!(all.len(), existing.len() + 7);
        for (expected, actual) in existing.iter().zip(&all) {
            assert_eq!((expected.0, Entry::Existing(expected.1)), *actual);
        }
        assert_eq!(combined(None, [false; 3], None, [false; 4]), vec![]);
    }
    #[test]
    fn property_menu_is_capability_and_handle_gated() {
        assert!(entries(Some(7), false, false, false, false).is_empty());
        assert!(entries(None, true, true, false, false).is_empty());
        assert!(entries(Some(0), true, true, false, false).is_empty());
        let menu = entries(Some(7), true, true, true, true);
        assert_eq!(menu.len(), 7);
        assert_eq!(menu[0].1, Action::Letter(7));
        assert_eq!(menu[4].1, Action::Dummy(7));
    }
}
