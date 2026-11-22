use super::{Callback, Position, Selector};

/// Crystal movements_prohibit_entry.lua, action 50000. The separate gate
/// permits passage inside the season even when the personal stage is complete.
pub(super) fn tinder_box_entry(summit: &Callback) -> Callback {
    let mut entry = summit.clone();
    entry.key = "oteryn:quest-callback/tinder-box-summit-entry".into();
    entry.selector = Selector {
        action: Some(50000),
        appearance: None,
        native: None,
        ordinal: None,
        position: None,
        position_aliases: Vec::new(),
        unique: None,
    };
    entry.premium_required = false;
    entry.storage.clear();
    entry.return_on_failure = true;
    entry.failure_teleport = Some(Position {
        x: 32063,
        y: 31091,
        floor: -7,
    });
    entry.failure_message =
        Some("You cannot pass here outside the period from April 1st to May 1st.".into());
    entry.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/tinder_box_quest_chyllfroest/movements_prohibit_entry.lua".into()];
    entry
}
