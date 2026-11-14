#[cfg(windows)]
#[path = "view/map_window.rs"]
mod map_window;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_panel(&self) -> super::state::Panel {
        lock(&self.mailbox).properties.house.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    lock(&link.mailbox).properties.house.ownership.context=std::sync::Arc::downgrade(&link.mailbox);
    open(
        link,
        Form {
            title: "Houses",
            labels: &[
                "World UUID",
                "Town key",
                "House row (1 onwards)",
                "Bid amount (gold)",
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
                if matches!(selection,8|9|14) {
                    let weak=mailbox.properties.house.ownership.context.clone();
                    ownership_window::open_from_mailbox(mailbox,weak,commands);
                } else if selection == 10 {
                    let panel = &mut mailbox.properties.house;
                    if let Some(index) = panel.fields[2]
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .and_then(|n| n.checked_sub(1))
                    {
                        panel.model.index = index;
                    }
                    let houses = oteryn_client_ui::house::map_houses(&panel.model);
                    if houses.is_empty() {
                        panel.model.feedback =
                            "Query houses and select a row to view its floor".into();
                    } else {
                        map_window::open(mailbox, commands, houses);
                    }
                } else {
                    super::input::submit(mailbox, commands, selection);
                }
            },
        },
    );
}

#[cfg(windows)]
fn text(panel: &oteryn_client_ui::house::Panel) -> String {
    let mut text = oteryn_client_ui::house::panel_text(panel);
    text.push_str(tr!("\r\nSelect a row for changes:"));
    for (index, row) in panel.page.rows.iter().enumerate() {
        text.push_str(&lfmt!(
            "\r\n{}: {} (owned: {})",
            index + 1,
            row.house,
            row.owned
        ));
    }
    text
}

#[cfg(windows)]
#[path = "view/ownership_window.rs"]
pub(super) mod ownership_window;
