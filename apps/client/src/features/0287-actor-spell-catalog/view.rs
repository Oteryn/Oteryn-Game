use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn spell_catalog_panel(&self) -> oteryn_client_ui::spell_catalog::Panel {
        lock(&self.mailbox).spell_catalog.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    crate::play::feature_0115_mail::view::native::open(link, form(false));
}
#[cfg(windows)]
pub(in crate::play) fn open_archive(link: &PlayLink) {
    crate::play::feature_0115_mail::view::native::open(link, form(true));
}
#[cfg(windows)]
fn form(archive: bool) -> crate::play::feature_0115_mail::view::native::Form {
    use crate::play::feature_0115_mail::view::native::Form;
    let mut form = Form {
        title: "Spell catalogue",
        labels: &[
            "Search name or words",
            "Maximum level (blank: all)",
            "Cooldown group (blank: all)",
            "Learned only (0 / 1)",
        ],
        buttons: &[
            "Search",
            "Previous page",
            "Next page",
            "Select previous",
            "Select next",
            "Assign slot 1",
            "Assign slot 2",
            "Assign slot 3",
            "Assign slot 4",
        ],
        snapshot,
        set_open: |mailbox, value| mailbox.spell_catalog.open = value,

        submit: |mailbox, _, action, values| {
            if values.len() != 4 || !matches!(values[3].as_str(), "0" | "1") {
                mailbox.spell_catalog.filter_error = true;
                return;
            }
            if !oteryn_client_ui::spell_catalog::protocol::set_filters(
                &mut mailbox.spell_catalog,
                &values[1],
                &values[2],
                values[3] == "1",
            ) {
                return;
            }
            if let Some(filter) = values.first() {
                oteryn_client_ui::spell_catalog::protocol::navigate(
                    &mut mailbox.spell_catalog,
                    filter,
                    action,
                );
                match action {
                    3 => oteryn_client_ui::spell_catalog::protocol::select_relative(
                        &mut mailbox.spell_catalog,
                        false,
                    ),
                    4 => oteryn_client_ui::spell_catalog::protocol::select_relative(
                        &mut mailbox.spell_catalog,
                        true,
                    ),
                    5..=8 => {
                        super::input::assign(mailbox, action - 5);
                    }
                    _ => {}
                }
            }
        },
    };
    if archive {
        form.title = "Cyclopedia: Magical Archive";
        form.snapshot = |mailbox| {
            let mut result = snapshot(mailbox);
            result.0 = mailbox.spell_catalog.archive_open;
            result.3 = format!("Magical Archive\r\n{}", result.3);
            result
        };
        form.set_open = |mailbox, value| mailbox.spell_catalog.archive_open = value;
    }
    form
}

#[cfg(windows)]
fn snapshot(mailbox: &crate::play::mailbox::Mailbox) -> (bool, bool, Vec<String>, String) {
    let panel = &mailbox.spell_catalog;
    (
        panel.open,
        false,
        vec![
            panel.filter.clone(),
            panel
                .max_level
                .map_or_else(String::new, |level| level.to_string()),
            panel
                .group
                .map_or_else(String::new, |group| group.to_string()),
            u8::from(panel.learned_only).to_string(),
        ],
        format!(
            "{}\r\n{}",
            oteryn_client_ui::spell_catalog::detail_text(panel, &mailbox.spell_bar.bar),
            oteryn_client_ui::spell_catalog::panel_text(panel)
        ),
    )
}
