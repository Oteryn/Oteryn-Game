//! Bevy's snapshot and controls retain the session owner; no transport lives in the window.
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::appearance::{Intent, Panel};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AppearanceSnapshot {
    pub panel: Panel,
    session: SharedMailbox,
}
impl PartialEq for AppearanceSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.session, &other.session) && self.panel == other.panel
    }
}
impl Eq for AppearanceSnapshot {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppearanceControl {
    Outfit(String),
    Addon(u8),
    Mount(Option<String>),
    Colour { part: usize, index: u16 },
    Apply(String),
    Close,
}
/// Read-only preview assets retained across presentation rebuilds.
pub struct AppearancePreview(super::gallery_preview::PreviewAssets);
impl AppearancePreview {
    pub fn load(root: &std::path::Path) -> Result<Self, String> {
        super::gallery_preview::PreviewAssets::load(root).map(Self)
    }
    pub fn pixels(&mut self, snapshot: &AppearanceSnapshot) -> Result<Vec<u8>, String> {
        self.0.render(&snapshot.panel).map(|image| image.rgba)
    }
}
impl PlayLink {
    pub fn appearance_snapshot(&self) -> AppearanceSnapshot {
        AppearanceSnapshot {
            panel: self.appearance_panel(),
            session: Arc::clone(&self.mailbox),
        }
    }
    /// Compare, validate, and dispatch under the same owner lock. Draft edits never touch confirmed.
    pub fn appearance_dispatch_bound(
        &self,
        expected: &AppearanceSnapshot,
        control: AppearanceControl,
    ) -> bool {
        use AppearanceControl as C;
        if !Arc::ptr_eq(&self.mailbox, &expected.session) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        // Closing presentation preserves every in-flight operation and receipt.
        if control == C::Close {
            if !std::sync::Arc::ptr_eq(&self.mailbox, &expected.session)
                || self.commands.is_closed()
                || mailbox.quest_shutdown
                || mailbox.custom_hotkeys_ended
            {
                return false;
            }
            mailbox.properties.appearance.open = false;
            return true;
        }

        if mailbox.properties.appearance != expected.panel
            || !expected.panel.open
            || !expected.panel.selected
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
        {
            return false;
        }
        if control == C::Close {
            mailbox.properties.appearance.open = false;
            return true;
        }
        if expected.panel.pending {
            return false;
        }
        let mut draft = expected.panel.clone();
        let Some(catalogue) = &draft.catalogue else {
            return false;
        };
        match control {
            C::Outfit(key) => {
                let Some(outfit) = catalogue.outfits.iter().find(|outfit| outfit.key == key) else {
                    return false;
                };
                draft.fields[0] = key;
                draft.fields[1] = (draft.fields[1].parse::<u8>().unwrap_or(0)
                    & outfit.permitted_addons)
                    .to_string();
            }
            C::Addon(bit) => {
                if !draft.toggle_addon(bit) {
                    return false;
                }
            }
            C::Mount(key) => {
                if key
                    .as_ref()
                    .is_some_and(|key| !catalogue.mounts.contains(key))
                {
                    return false;
                }
                draft.fields[2] = key.unwrap_or_default();
            }
            C::Colour { part, index } => {
                if part >= 4 || usize::from(index) >= catalogue.palette.len() {
                    return false;
                }
                let Some(mut colours) = draft.fields[3]
                    .split(',')
                    .map(|s| s.trim().parse::<u16>().ok())
                    .collect::<Option<Vec<_>>>()
                    .filter(|v| v.len() == 4)
                else {
                    return false;
                };
                colours[part] = index;
                draft.fields[3] = colours
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
            }
            C::Apply(revision) => {
                if revision.len() > 128 || revision.contains('\0') {
                    return false;
                }
                draft.fields[4] = revision;
                let Ok(command) = super::super::protocol::command(&draft, Intent::Apply) else {
                    return false;
                };
                if !oteryn_client_ui::appearance::request(&mut draft, command.kind) {
                    return false;
                }
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
                {
                    return false;
                }
            }
            C::Close => return false,
        }
        mailbox.properties.appearance = draft;
        true
    }
}
