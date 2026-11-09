use crate::play::mailbox::Mailbox;
use oteryn_client_ui::{analyser::Tool, cyclopedia_map::Section};

/// Direct tool buttons follow the existing map controls in the Cyclopedia host.
pub(super) fn select(mailbox: &mut Mailbox, action: usize) -> bool {
    let Some(tool) = action
        .checked_sub(26)
        .and_then(|index| Tool::CYCLOPEDIA.get(index))
    else {
        return false;
    };
    mailbox.analyser.tool = *tool;
    mailbox
        .properties
        .cyclopedia_map
        .navigate(Section::Analytics);
    true
}

pub(super) fn text(mailbox: &Mailbox) -> String {
    let mut text = format!("Selected tool: {}\r\n", mailbox.analyser.tool.title());
    text.push_str(&oteryn_client_ui::analyser::panel_text(&mailbox.analyser));
    text
}
