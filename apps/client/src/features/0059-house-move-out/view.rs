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
    crate::play::feature_0115_mail::view::native::open(link, form());
}
#[cfg(any(windows, test, feature = "bevy-shell"))]
pub(in crate::play) fn form() -> crate::play::feature_0115_mail::view::forms::Form {
    use crate::play::feature_0115_mail::view::forms::Form;
    Form {
        title: "House move-out",
        labels: &["House key", "Due time (Unix milliseconds)"],
        buttons: &[
            "Query",
            "Review notice",
            "Cancel notice",
            "Confirm reviewed notice",
            "Discard review",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.properties.house_move_out;
            let mut text = oteryn_client_ui::house_move_out::panel_text(panel);
            if let Some(fields) = &panel.review {
                let date = fields[1]
                    .parse()
                    .map(oteryn_client_ui::house::format_ownership_date)
                    .unwrap_or_default();
                text.push_str(&format!(
                    "\n\nConfirm move-out for {} on {date}?",
                    fields[0]
                ));
            }
            (panel.open, panel.pending, panel.fields.to_vec(), text)
        },
        set_open: |mailbox, value| {
            let panel = &mut mailbox.properties.house_move_out;
            panel.open = value;
            if !value {
                panel.review = None;
            }
        },
        submit: |mailbox, commands, action, values| {
            let panel = &mut mailbox.properties.house_move_out;
            if panel.pending {
                return;
            }
            if action == 3 {
                if let Some(fields) = panel.review.take() {
                    panel.fields = fields;
                    super::input::submit(
                        mailbox,
                        commands,
                        oteryn_client_ui::house_move_out::Intent::Notice,
                    );
                }
                return;
            }
            if action == 4 {
                panel.review = None;
                return;
            }
            panel.review = None;
            for (index, value) in values.into_iter().enumerate() {
                super::input::edit(panel, index, value);
            }
            let intent = match action {
                0 => oteryn_client_ui::house_move_out::Intent::Query,
                1 => oteryn_client_ui::house_move_out::Intent::Notice,
                2 => oteryn_client_ui::house_move_out::Intent::Cancel,
                _ => return,
            };
            if action == 1 {
                match super::protocol::command(panel, intent) {
                    Ok(_) => panel.review = Some(panel.fields.clone()),
                    Err(error) => panel.feedback = error,
                }
                return;
            }
            super::input::submit(mailbox, commands, intent);
        },
    }
}
