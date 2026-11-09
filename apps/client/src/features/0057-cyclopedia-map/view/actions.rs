use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_ui::cyclopedia_map::Section;
use tokio::sync::mpsc::UnboundedSender;

/// Refresh the active section through its existing capability and pending gates.
pub(super) fn refresh_section(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    match mailbox.properties.cyclopedia_map.section {
        Section::Map => super::super::input::submit(
            mailbox,
            commands,
            oteryn_client_ui::cyclopedia_map::Intent::Query,
        ),
        Section::Bestiary | Section::Charms => {
            crate::play::feature_0067_bestiary_charm_progression::input::submit(
                mailbox,
                commands,
                oteryn_client_ui::charm_progression::Intent::Query,
            )
        }
        Section::Bosses => crate::play::feature_0117_boss_encounter::input::enqueue(
            mailbox,
            commands,
            oteryn_client_ui::boss_encounter::Intent::Refresh,
        ),
        Section::Houses => crate::play::feature_0049_house::input::submit(mailbox, commands, 0),
        Section::Character => {
            let panel = &mut mailbox.cyclopedia_character;
            if let Some(command) = oteryn_client_ui::cyclopedia_character::request(
                panel,
                oteryn_client_ui::cyclopedia_character::Intent::Tab,
            ) && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = Some(tr!("The session has ended"));
            }
        }
        Section::Items => {
            use oteryn_client_ui::inspection::{Request, request};
            let panel = &mut mailbox.discovery.inspection;
            let Some(previous @ Request::Catalogue(_)) = panel.last_request else {
                panel.feedback = tr!("Inspect a catalogue definition to load it here").into();
                return;
            };
            if let Some(command) = request(panel, previous)
                && commands.send(PlayCommand::ServerMechanic(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = tr!("The session has ended").into();
            }
        }
        Section::Analytics => {}
    }
}

/// Catalogue inspection uses the existing public-template request, never player inspection.
pub(super) fn load_item_definition(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    use oteryn_client_ui::inspection::{Request, request};
    let panel = &mut mailbox.discovery.inspection;
    if panel.pending {
        return;
    }
    if !panel.selected {
        panel.feedback = tr!("Item catalogue unavailable in this session").into();
        return;
    }
    let Some(command) = panel
        .catalogue_definition
        .trim()
        .parse::<u32>()
        .ok()
        .and_then(|id| request(panel, Request::Catalogue(id)))
    else {
        panel.feedback = tr!("Enter a valid item definition reference").into();
        return;
    };
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        panel.pending = false;
        panel.feedback = tr!("The session has ended").into();
    }
}

/// Shared native navigation preserves each section's confirmed state.
pub(super) fn navigate_section(mailbox: &mut Mailbox, index: usize, values: &[String]) -> bool {
    let Some(section) = Section::ALL.get(index).copied() else {
        return false;
    };
    mailbox.properties.cyclopedia_map.navigate(section);
    if section == Section::Items {
        mailbox.discovery.inspection.search = values.get(1).cloned().unwrap_or_default();
        mailbox.discovery.inspection.category = values.get(2).cloned().unwrap_or_default();
    }
    true
}
