//! Atomic presentation bindings over the retained character owner.
use crate::{combat_input::PlayCommand, play::{PlayLink, mailbox::lock}};
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel as Panel, Intent, request};
#[derive(Clone)]
pub(crate) enum CharacterAction {
    Toggle,
    LocalBadge(u8),
    Close,
    Request(Intent),
    Native(usize, [String; 3]),
    GalleryFilter(Option<oteryn_client_runtime::cyclopedia_character::gallery::Kind>, String),
    ItemFilter(Option<oteryn_client_runtime::cyclopedia_character::items::Location>, String),
}
impl PlayLink {
    pub(crate) fn cyclopedia_character_bound(&self, expected: &Panel, action: CharacterAction) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.cyclopedia_character_shutdown || mailbox.server_modal.is_active() {
            return false;
        }
        let panel = &mut mailbox.cyclopedia_character;
        if panel != expected || !panel.selected || self.commands.is_closed() {
            return false;
        }
        if matches!(action, CharacterAction::Toggle) {
            panel.open = !panel.open;
            return true;
        }
        if !panel.open { return false; }
        if matches!(action, CharacterAction::Close) {
            panel.open = false;
            return true;
        }
        if panel.pending || panel.blessings.pending { return false; }
        if let CharacterAction::LocalBadge(id) = action {
            if !panel.badges.as_ref().is_some_and(|ids| ids.contains(&id)) { return false; }
            if !panel.hidden_badges.remove(&id) { panel.hidden_badges.insert(id); }
            return true;
        }
        let mut next = panel.clone();
        let command = match action {
            CharacterAction::Request(intent) => {
                match intent {
                    Intent::Appearances | Intent::GalleryNext | Intent::GalleryPrevious => next.gallery.open = true,
                    Intent::Items | Intent::ItemsNext | Intent::ItemsPrevious => next.items.open = true,
                    Intent::Protection => next.protection.open = true,
                    _ => { next.gallery.open = false; next.items.open = false; next.protection.open = false; }
                }
                request(&mut next, intent)
            }
            CharacterAction::Native(id, values) => {
                next.gallery.open = false; next.items.open = false; next.protection.open = false;
                super::native::submit(&mut next, id, [&values[0], &values[1], &values[2]])
            }
            CharacterAction::GalleryFilter(kind, search) => {
                next.gallery.filter = kind; next.gallery.search = search;
                *panel = next; return true;
            }
            CharacterAction::ItemFilter(location, search) => {
                next.items.location = location; next.items.search = search;
                *panel = next; return true;
            }
            CharacterAction::LocalBadge(_) | CharacterAction::Close | CharacterAction::Toggle => return false,
        };
        if let Some(command) = command {
            if self.commands.send(PlayCommand::Feature(command)).is_err() { return false; }
        } else if next == *panel || next.pending { return false; }
        *panel = next;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::play_channel;
    #[test]
    fn cyclopedia_character_actions_are_bound_and_confirmed() -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, outbox) = play_channel();
        outbox.cyclopedia_character_initialize(true);
        let panel = link.cyclopedia_character_panel();
        assert!(link.cyclopedia_character_bound(&panel, CharacterAction::Toggle));
        let panel = link.cyclopedia_character_panel();
        assert!(link.cyclopedia_character_bound(&panel, CharacterAction::Request(Intent::Items)));
        let PlayCommand::Feature(command) = commands.try_recv()? else { return Err("missing feature request".into()); };
        assert_eq!(command.payload, oteryn_client_runtime::cyclopedia_character::items::request(0));
        assert!(link.cyclopedia_character_panel().items.page.is_none());
        assert!(!link.cyclopedia_character_bound(&panel, CharacterAction::Request(Intent::Items)));
        assert!(commands.try_recv().is_err());
        let page = oteryn_client_runtime::cyclopedia_character::items::Page { offset:0,total:0,rows:vec![] };
        outbox.cyclopedia_character_result(&oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id:1, status:oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
            server_sequence:1,payload:page.encode().map_err(|_| "encode")?,
        });
        assert_eq!(link.cyclopedia_character_panel().items.page, Some(page));
        Ok(())
    }
    #[test]
    fn cyclopedia_character_rejected_when_session_replaced_or_target_changed() {
        let (link, mut commands, outbox) = play_channel();
        outbox.cyclopedia_character_initialize(true);
        let panel = link.cyclopedia_character_panel();
        assert!(link.cyclopedia_character_bound(&panel, CharacterAction::Toggle));
        let panel = link.cyclopedia_character_panel();
        lock(&link.mailbox).cyclopedia_character.change_target(Some([7;16]));
        let changed = link.cyclopedia_character_panel();
        assert!(!link.cyclopedia_character_bound(&panel, CharacterAction::Request(Intent::Tab)));
        assert_eq!(link.cyclopedia_character_panel(),changed);
        assert!(commands.try_recv().is_err());
        let (replacement, _, _) = play_channel();
        assert!(!replacement.cyclopedia_character_bound(&panel, CharacterAction::Request(Intent::Tab)));
        drop(commands);
        assert!(!link.cyclopedia_character_bound(&changed, CharacterAction::Request(Intent::Tab)));
        assert_eq!(link.cyclopedia_character_panel(),changed);
    }
}
