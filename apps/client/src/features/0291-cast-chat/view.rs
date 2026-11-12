use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cast_chat_panel(&self) -> oteryn_client_ui::cast_chat::Panel {
        lock(&self.mailbox).cast_chat.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Live cast chat",
            labels: &[
                "Reply to cast room",
                "Viewer handle (32 hex digits, from the live lines)",
            ],
            buttons: &["Reply", "Mute", "Unmute", "Kick", "Ban"],
            snapshot: |m| {
                (
                    m.cast_chat.open,
                    m.cast_chat.pending,
                    vec![String::new(), String::new()],
                    oteryn_client_ui::cast_chat::panel_text(&m.cast_chat),
                )
            },
            set_open: |m, value| m.cast_chat.open = value,
            submit: |m, commands, action, values| {
                if let Ok(action) = u8::try_from(action) {
                    super::input::submit(
                        m,
                        commands,
                        values.first().map_or("", String::as_str),
                        values.get(1).map_or("", String::as_str),
                        action,
                    );
                }
            },
        },
    );
}
