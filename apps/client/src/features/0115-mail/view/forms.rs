//! The same bounded feature forms drive native controls and shared widgets.
#[cfg(any(test, feature = "bevy-shell"))]
use crate::play::{PlayLink, mailbox::lock};
use crate::{combat_input::PlayCommand, play::mailbox::Mailbox};
use tokio::sync::mpsc::UnboundedSender;
#[path = "forms/services.rs"]
#[cfg(any(test, feature = "bevy-shell"))]
mod services;
type SnapshotFn = fn(&Mailbox) -> (bool, bool, Vec<String>, String);
#[derive(Clone, Copy)]
pub(crate) struct Form {
    pub title: &'static str,
    pub labels: &'static [&'static str],
    pub buttons: &'static [&'static str],
    pub(in crate::play) snapshot: SnapshotFn,
    pub(in crate::play) set_open: fn(&mut Mailbox, bool),
    pub(in crate::play) submit: fn(&mut Mailbox, &UnboundedSender<PlayCommand>, usize, Vec<String>),
}
#[cfg(any(test, feature = "bevy-shell"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub open: bool,
    pub pending: bool,
    pub fields: Vec<String>,
    pub text: String,
    pub drawing: Option<oteryn_client_ui::cyclopedia_map::atlas_view::Drawing>,
}
#[cfg(any(test, feature = "bevy-shell"))]
impl Form {
    pub fn read(&self, link: &PlayLink) -> Snapshot {
        let mailbox = lock(&link.mailbox);
        let (open, pending, fields, text) = (self.snapshot)(&mailbox);
        Snapshot {
            open,
            pending,
            fields,
            text,
            drawing: services::drawing(self.title, &mailbox),
        }
    }
    pub fn dispatch(
        &self,
        link: &PlayLink,
        expected: &Snapshot,
        action: usize,
        values: Vec<String>,
    ) -> bool {
        if action >= self.buttons.len() || !expected.open || expected.pending {
            return false;
        }
        let mut mailbox = lock(&link.mailbox);
        let (open, pending, fields, text) = (self.snapshot)(&mailbox);
        if !open || pending || fields != expected.fields || text != expected.text {
            return false;
        }
        (self.submit)(&mut mailbox, &link.commands, action, values);
        true
    }
    pub fn close(&self, link: &PlayLink) {
        (self.set_open)(&mut lock(&link.mailbox), false);
    }
}
#[cfg(any(test, feature = "bevy-shell"))]
pub(crate) fn all() -> Vec<Form> {
    use crate::play::*;
    vec![
        feature_0054_training::view::form(),
        feature_0051_staff::view::form(),
        feature_0055_guild_bank::view::form(),
        feature_0057_cyclopedia_map::view::form(),
        feature_0059_house_move_out::view::form(),
        services::world_event(),
        services::atlas(),
        services::ownership(),
        services::house_map(),
        services::overview(),
    ]
}
#[cfg(test)]
mod tests {
    #[test]
    fn service_forms_reject_closed_and_stale_controls() {
        let (link, _, _) = crate::play::play_channel();
        for form in super::all() {
            let snapshot = form.read(&link);
            assert!(!form.title.is_empty());
            assert!(form.labels.len() == snapshot.fields.len());
            assert!(!form.dispatch(&link, &snapshot, 0, snapshot.fields.clone()));
            form.close(&link);
        }
    }
}
