use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_furniture_panel(&self) -> oteryn_client_ui::house_furniture_wrap::Panel {
        lock(&self.mailbox).properties.house_furniture_wrap.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    use oteryn_client_ui::house_furniture_wrap::Intent;
    open(
        link,
        Form {
            title: "House furniture",
            labels: &["Furniture number from the list"],
            buttons: &["Refresh", "Previous", "Next", "Wrap / place"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.house_furniture_wrap;
                (
                    panel.open,
                    panel.pending,
                    vec![panel.choice.clone()],
                    oteryn_client_ui::house_furniture_wrap::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house_furniture_wrap.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.house_furniture_wrap;
                if panel.pending {
                    return;
                }
                if let Some(value) = values
                    .first()
                    .filter(|value| value.len() <= 3 && value.chars().all(|ch| ch.is_ascii_digit()))
                {
                    panel.choice = value.clone();
                } else if action == 3 {
                    panel.feedback = tr!("Choose a furniture number from the list").into();
                    return;
                }
                let intent = match action {
                    0 => Intent::Refresh,
                    1 => Intent::Previous,
                    2 => Intent::Next,
                    3 => Intent::Apply,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}
