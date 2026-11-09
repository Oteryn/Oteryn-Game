use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cast_control_panel(&self) -> oteryn_client_ui::cast_control::Panel {
        lock(&self.mailbox).cast_control.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Cast settings",
            labels: &[
                "Delay seconds (0–300)",
                "Viewer limit (1–50)",
                "Password required (0/1, saved in account settings)",
                "Saved access revision",
            ],
            buttons: &[
                "Start / apply",
                "Stop",
                "Read consent",
                "Allow shown",
                "Hide me",
                "Refresh cast",
            ],
            snapshot: |m| {
                let p = &m.cast_control;
                (
                    p.open,
                    p.pending,
                    vec![
                        p.delay.to_string(),
                        p.cap.to_string(),
                        u8::from(p.password).to_string(),
                        p.access_revision.to_string(),
                    ],
                    oteryn_client_ui::cast_consent::panel_text(p),
                )
            },
            set_open: |m, v| m.cast_control.open = v,
            submit: |m, commands, action, values| {
                if action != 0 {
                    super::input::submit(m, commands, action);
                    return;
                }
                let parsed = (|| {
                    Some((
                        values.first()?.parse().ok()?,
                        values.get(1)?.parse().ok()?,
                        match values.get(2)?.as_str() {
                            "0" => false,
                            "1" => true,
                            _ => return None,
                        },
                        values.get(3)?.parse().ok()?,
                    ))
                })();
                if let Some((delay, cap, password, access_revision)) = parsed {
                    m.cast_control.delay = delay;
                    m.cast_control.cap = cap;
                    m.cast_control.password = password;
                    m.cast_control.access_revision = access_revision;
                    super::input::submit(m, commands, action);
                } else {
                    m.cast_control.feedback = tr!("Enter valid saved cast settings").into();
                }
            },
        },
    );
}
