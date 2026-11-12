use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guild_war_panel(&self) -> oteryn_client_ui::guild_war::Panel {
        lock(&self.mailbox).properties.guild_war.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Guild wars",
            labels: &[
                "Opponent guild UUID (declare) or war UUID (accept)",
                "Frag limit (declare)",
                "Duration in days (declare)",
            ],
            buttons: &["Declare", "Accept"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.guild_war;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::guild_war::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.guild_war.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.guild_war;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::guild_war::Intent::Declare,
                    1 => oteryn_client_ui::guild_war::Intent::Accept,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
