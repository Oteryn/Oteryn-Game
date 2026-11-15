#[cfg(windows)]
#[path = "graphic_window.rs"]
mod graphic_window;
#[cfg(any(windows, test))]
#[path = "preview.rs"]
mod preview;
#[cfg(windows)]
#[path = "preview_assets.rs"]
mod preview_assets;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_podium_panel(&self) -> oteryn_client_ui::house_podium::Panel {
        lock(&self.mailbox).properties.house_podium.clone()
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
            title: "House podium",
            labels: &[
                "Podium item UUID",
                "Display: 1 outfit, 2 mount, 3 creature",
                "Direction: 0 north, 1 east, 2 south, 3 west",
                "Display content key",
            ],
            buttons: button_labels(&link.house_podium_panel()),
            snapshot: |mailbox| {
                let panel = &mailbox.properties.house_podium;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::house_podium::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.house_podium.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.house_podium;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                if (3..=5).contains(&action) {
                    super::input::edit(panel, 1, (action - 2).to_string());
                    return;
                }
                if action == 6 {
                    let direction = panel.fields[2]
                        .parse::<u8>()
                        .ok()
                        .filter(|v| *v < 4)
                        .unwrap_or(0);
                    super::input::edit(panel, 2, ((direction + 1) % 4).to_string());
                    return;
                }
                if (8..=10).contains(&action) {
                    use oteryn_client_ui::house_podium_flags::{Part, toggle};
                    toggle(
                        panel,
                        [Part::Platform, Part::Outfit, Part::Mount][action - 8],
                    );
                    return;
                }
                let intent = match action {
                    0 => oteryn_client_ui::house_podium::Intent::Refresh,
                    1 => oteryn_client_ui::house_podium::Intent::Projection,
                    2 => oteryn_client_ui::house_podium::Intent::Set,
                    7 => {
                        panel.preview_requested = true;
                        oteryn_client_ui::house_podium::Intent::Preview
                    }
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
    graphic_window::start(link);
}

#[cfg(any(windows, test))]
fn button_labels(panel: &oteryn_client_ui::house_podium::Panel) -> &'static [&'static str] {
    const BUTTONS: &[&str] = &[
        "Refresh",
        "Projection",
        "Set",
        "Outfit",
        "Mount",
        "Creature",
        "Turn clockwise",
        "Preview artwork",
        "Toggle platform",
        "Toggle outfit",
        "Toggle mount",
    ];
    if panel.flagged {
        BUTTONS
    } else {
        &BUTTONS[..8]
    }
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    #[test]
    fn podium_toggle_controls_follow_capability_selection() {
        let (link, _, outbox) = crate::play::play_channel();
        for flagged in [false, true, false] {
            lock(&outbox.0).properties.house_podium.flagged = flagged;
            let panel = link.house_podium_panel();
            let labels = button_labels(&panel);
            assert_eq!(labels.len(), if flagged { 11 } else { 8 });
            assert_eq!(labels.contains(&"Toggle platform"), flagged);
            assert_eq!(labels.contains(&"Toggle outfit"), flagged);
            assert_eq!(labels.contains(&"Toggle mount"), flagged);
            assert!(labels.contains(&"Preview artwork"));
            use oteryn_client_ui::house_podium_flags::Part;
            for part in [Part::Platform, Part::Outfit, Part::Mount] {
                assert_eq!(link.house_podium_toggle(part), flagged);
            }
        }
    }
}
