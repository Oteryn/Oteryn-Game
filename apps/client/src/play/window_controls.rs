//! Shared window buttons use the same panel owners and commands as physical shortcuts.
use super::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_input_actions::NormalizedInputEvent;

fn open_slot<'a>(mailbox: &'a mut Mailbox, id: &str) -> Option<&'a mut bool> {
    Some(match id {
        "0199-cyclopedia-character-open" => &mut mailbox.cyclopedia_character.open,
        "0287-actor-spell-catalog-open" => &mut mailbox.spell_catalog.open,
        "local-spell-action-bar-open" => &mut mailbox.spell_bar.open,
        "0279-skill-progress-open" => &mut mailbox.skill_progress.open,
        "0276-condition-status-open" => &mut mailbox.condition_status.open,
        "0277-rest-status-open" => &mut mailbox.rest_status.open,
        "0016-quest-log-open" => &mut mailbox.quest.open,
        "0007-chat-keybindings" => &mut mailbox.chat.open,
        _ => return None,
    })
}

impl PlayLink {
    #[must_use]
    pub fn window_open(&self, id: &str) -> bool {
        open_slot(&mut lock(&self.mailbox), id).is_some_and(|open| *open)
    }

    pub fn window_toggle(&self, id: &str, events: &[NormalizedInputEvent]) -> bool {
        {
            let mut mailbox = lock(&self.mailbox);
            let Some(open) = open_slot(&mut mailbox, id) else {
                return false;
            };
            if *open {
                *open = false;
                return true;
            }
        }
        match id {
            "0199-cyclopedia-character-open" => self.cyclopedia_character_input(events),
            "0287-actor-spell-catalog-open" => self.spell_catalog_input(events),
            "local-spell-action-bar-open" => self.spell_bar_input(events),
            "0279-skill-progress-open" => self.skill_progress_input(events),
            "0276-condition-status-open" => self.condition_status_input(events),
            "0277-rest-status-open" => self.rest_status_input(events),
            "0016-quest-log-open" => self.quest_input(events),
            "0007-chat-keybindings" => self.chat_input(events),
            _ => false,
        }
    }
}

#[must_use]
pub fn window_icon(id: &str) -> &'static str {
    match id {
        "0199-cyclopedia-character-open" => "▣",
        "0287-actor-spell-catalog-open" => "✦",
        "local-spell-action-bar-open" => "▤",
        "0279-skill-progress-open" => "↗",
        "0276-condition-status-open" => "✚",
        "0277-rest-status-open" => "☾",
        "0016-quest-log-open" => "☑",
        "0007-chat-keybindings" => "☏",
        _ => "",
    }
}
