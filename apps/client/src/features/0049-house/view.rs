use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_panel(&self) -> super::state::Panel {
        lock(&self.mailbox).properties.house.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Houses",
            labels: &[
                "World UUID",
                "Town key",
                "House row (1 onwards)",
                "Bid / notice Unix time",
                "Member UUIDs (space/newline separated)",
                "Door: x y floor (uses current house ACL revision)",
                "Type CONFIRM for changes",
            ],
            buttons: super::input::BUTTONS,
            snapshot: |mailbox| {
                let p = &mailbox.properties.house;
                (
                    p.model.open,
                    p.model.pending.is_some(),
                    p.fields.to_vec(),
                    text(&p.model),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house.model.open = value,
            submit: |mailbox, commands, selection, fields| {
                if mailbox.properties.house.model.pending.is_some() {
                    return;
                }
                for (slot, value) in mailbox.properties.house.fields.iter_mut().zip(fields) {
                    *slot = value;
                }
                super::input::submit(mailbox, commands, selection);
            },
        },
    );
}

#[cfg(windows)]
fn text(panel: &oteryn_client_ui::house::Panel) -> String {
    let mut text = oteryn_client_ui::house::panel_text(panel);
    text.push_str("\r\nSelect a row for changes:");
    for (index, row) in panel.page.rows.iter().enumerate() {
        text.push_str(&format!(
            "\r\n{}: {} (owned: {})",
            index + 1,
            row.house,
            row.owned
        ));
    }
    text
}
