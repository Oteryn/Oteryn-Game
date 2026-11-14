#[cfg(windows)]
#[path = "graphic_window.rs"]
mod graphic_window;
#[cfg(any(windows, test, feature = "bevy-shell"))]
#[path = "preview.rs"]
mod preview;
#[cfg(any(windows, feature = "bevy-shell"))]
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

#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HousePodiumControl {
    Close,
    Display(u8),
    Turn,
    Toggle(oteryn_client_ui::house_podium_flags::Part),
    Request(oteryn_client_ui::house_podium::Intent),
}

#[cfg(feature = "bevy-shell")]
pub(crate) struct HousePodiumArtwork(preview_assets::Assets);

#[cfg(feature = "bevy-shell")]
impl HousePodiumArtwork {
    pub(crate) fn load() -> Result<Self, String> {
        let root = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .ok_or("Set OTERYN_ASSET_DIR to the existing pinned artwork root")?;
        preview_assets::Assets::load(std::path::Path::new(&root)).map(Self)
    }
    pub(crate) fn render(
        &mut self,
        artwork: oteryn_session::property_services::house_podium::preview::Artwork,
    ) -> Result<Vec<Vec<u8>>, String> {
        self.0.render(artwork)
    }
}

/// Dispatch against the exact retained projection captured by the presenter.
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn house_podium_draft_bound(
        &self,
        expected: &oteryn_client_ui::house_podium::Panel,
        fields: [String; 4],
        control: HousePodiumControl,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.server_modal.is_active() {
            return false;
        }
        let panel = &mut mailbox.properties.house_podium;
        if panel != expected
            || !panel.open
            || self.commands.is_closed()
            || fields.iter().any(|v| v.len() > 7996 || v.contains('\0'))
        {
            return false;
        }
        if control == HousePodiumControl::Close {
            panel.open = false;
            return true;
        }
        if !panel.selected || panel.pending {
            return false;
        }
        let mut draft = panel.clone();
        for (index, value) in fields.into_iter().enumerate() {
            super::input::edit(&mut draft, index, value);
        }
        if let HousePodiumControl::Request(intent) = control {
            let Ok(command) = super::protocol::command(&draft, intent) else {
                return false;
            };
            if !oteryn_client_ui::house_podium::request(&mut draft, command.kind) {
                return false;
            }
            if self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
            {
                return false;
            }
            *panel = draft;
            return true;
        }
        match control {
            HousePodiumControl::Close => draft.open = false,
            HousePodiumControl::Display(kind) => {
                if !(1..=3).contains(&kind) {
                    return false;
                }
                super::input::edit(&mut draft, 1, kind.to_string());
            }
            HousePodiumControl::Turn => {
                let direction = draft.fields[2]
                    .parse::<u8>()
                    .ok()
                    .filter(|v| *v < 4)
                    .unwrap_or(0);
                super::input::edit(&mut draft, 2, ((direction + 1) % 4).to_string());
            }
            HousePodiumControl::Toggle(part) => {
                if !oteryn_client_ui::house_podium_flags::toggle(&mut draft, part) {
                    return false;
                }
            }
            HousePodiumControl::Request(_) => return false,
        }
        *panel = draft;
        true
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

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    use super::*;
    use oteryn_client_ui::house_podium::Intent;
    fn seed(link: &PlayLink) {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.properties.house_podium;
        panel.selected = true;
        panel.open = true;
        panel.fields[0] = "00112233-4455-6677-8899-aabbccddeeff".into();
    }
    #[test]
    fn house_podium_actions_are_bound_and_confirmed() -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, _) = crate::play::play_channel();
        seed(&link);
        let expected = link.house_podium_panel();
        let mut invalid = expected.fields.clone();
        invalid[0] = "invalid UUID".into();
        assert!(!link.house_podium_draft_bound(
            &expected,
            invalid,
            HousePodiumControl::Request(Intent::Refresh)
        ));
        assert_eq!(link.house_podium_panel(), expected);
        assert!(commands.try_recv().is_err());
        assert!(link.house_podium_draft_bound(
            &expected,
            expected.fields.clone(),
            HousePodiumControl::Request(Intent::Refresh)
        ));
        assert!(matches!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::Feature(_)
        ));
        assert!(!link.house_podium_draft_bound(
            &expected,
            expected.fields.clone(),
            HousePodiumControl::Request(Intent::Refresh)
        ));
        assert!(commands.try_recv().is_err());
        assert_eq!(link.house_podium_panel().summary, expected.summary);
        assert_eq!(link.house_podium_panel().revision, expected.revision);
        Ok(())
    }
    #[test]
    fn house_podium_all_requests_use_existing_commands() -> Result<(), Box<dyn std::error::Error>> {
        for intent in [
            Intent::Refresh,
            Intent::Projection,
            Intent::Preview,
            Intent::Set,
        ] {
            let (link, mut commands, _) = crate::play::play_channel();
            seed(&link);
            {
                let mut mailbox = lock(&link.mailbox);
                let panel = &mut mailbox.properties.house_podium;
                panel.loaded_item = panel.fields[0].clone();
                panel.revision = Some(7);
                panel.fields[1] = "1".into();
                panel.fields[2] = "2".into();
                panel.fields[3] = "outfit.test".into();
            }
            let before = link.house_podium_panel();
            let expected = super::super::protocol::command(&before, intent)?;
            assert!(link.house_podium_draft_bound(
                &before,
                before.fields.clone(),
                HousePodiumControl::Request(intent)
            ));
            let crate::combat_input::PlayCommand::Feature(actual) = commands.try_recv()? else {
                return Err("wrong dispatcher".into());
            };
            assert_eq!(actual.capability, expected.capability);
            assert_eq!(actual.kind, expected.kind);
            assert_eq!(actual.payload, expected.payload);
            assert_eq!(link.house_podium_panel().revision, before.revision);
            assert_eq!(link.house_podium_panel().summary, before.summary);
            assert!(commands.try_recv().is_err());
        }
        Ok(())
    }

    #[test]
    fn house_podium_rejected_when_session_replaced_or_target_changed() {
        let (link, mut commands, _) = crate::play::play_channel();
        seed(&link);
        let expected = link.house_podium_panel();
        link.house_podium_edit(0, "ffeeddcc-bbaa-9988-7766-554433221100".into());
        let changed = link.house_podium_panel();
        assert!(!link.house_podium_draft_bound(
            &expected,
            expected.fields.clone(),
            HousePodiumControl::Request(Intent::Refresh)
        ));
        assert_eq!(link.house_podium_panel(), changed);
        lock(&link.mailbox).properties.house_podium.open = false;
        let closed = link.house_podium_panel();
        assert!(!link.house_podium_draft_bound(
            &closed,
            closed.fields.clone(),
            HousePodiumControl::Request(Intent::Refresh)
        ));
        assert_eq!(link.house_podium_panel(), closed);
        assert!(commands.try_recv().is_err());
        drop(commands);
        lock(&link.mailbox).properties.house_podium.open = true;
        let ended = link.house_podium_panel();
        assert!(!link.house_podium_draft_bound(
            &ended,
            ended.fields.clone(),
            HousePodiumControl::Request(Intent::Refresh)
        ));
        assert_eq!(link.house_podium_panel(), ended);
    }
}
