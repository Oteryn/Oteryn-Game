#[cfg(feature = "bevy-shell")]
#[path = "bound.rs"]
pub(crate) mod bound;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_move_out_panel(&self) -> oteryn_client_ui::house_move_out::Panel {
        lock(&self.mailbox).properties.house_move_out.clone()
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
            title: "House move-out",
            labels: &["House key", "Due time (Unix milliseconds)"],
            buttons: &["Query", "Notice", "Cancel"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.house_move_out;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::house_move_out::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house_move_out.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.house_move_out;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::house_move_out::Intent::Query,
                    1 => oteryn_client_ui::house_move_out::Intent::Notice,
                    2 => oteryn_client_ui::house_move_out::Intent::Cancel,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
