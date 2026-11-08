//! Serialize background gear reads with the existing inspection controller.
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::imbue::equipment_tracker::GearScope;
use oteryn_client_ui::inspection::Request;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn scope(mailbox: &Mailbox) -> Vec<GearScope> {
    oteryn_client_ui::equipment::rows(Some(&mailbox.depot.items.inventory), true)
        .into_iter()
        .filter_map(|slot| {
            slot.item.map(|item| GearScope {
                handle: item.handle,
                definition_ref: item.item_definition_ref.get(),
                label: slot.label.into(),
            })
        })
        .collect()
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    force: bool,
) -> bool {
    let scope = scope(mailbox);
    mailbox.imbue.equipment.reconcile(scope);
    mailbox.imbue.equipment.available = mailbox.discovery.inspection.selected;
    if !mailbox.imbue.tracker_open || mailbox.imbue_shutdown {
        return false;
    }
    if mailbox.world_input_blocked() {
        mailbox.imbue.equipment.reconcile(vec![]);
        return false;
    }
    if mailbox.discovery.inspection.open {
        mailbox.imbue.equipment.feedback =
            Some("Close item inspection to resume automatic equipment refresh");
        return false;
    }
    if mailbox.discovery.inspection.pending {
        return false;
    }
    if force {
        mailbox.imbue.equipment.refresh_now();
    }
    let Some(item) = mailbox.imbue.equipment.next(Instant::now()) else {
        return false;
    };
    let Some(command) = oteryn_client_ui::inspection::request(
        &mut mailbox.discovery.inspection,
        Request::Item(item),
    ) else {
        mailbox.imbue.equipment.pending = None;
        return false;
    };
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        mailbox.discovery.inspection.pending = false;
        mailbox.imbue.equipment.pending = None;
        mailbox.imbue.equipment.feedback = Some("The session has ended");
        return false;
    }
    true
}
impl PlayLink {
    pub fn imbue_tracker_refresh(&self) -> bool {
        enqueue(&mut lock(&self.mailbox), &self.commands, true)
    }
}
#[cfg(windows)]
pub(in crate::play) fn poll(
    weak: std::sync::Weak<std::sync::Mutex<Mailbox>>,
    commands: UnboundedSender<PlayCommand>,
) {
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("equipped-imbuement-refresh".into())
        .spawn(move || {
            loop {
                let Some(shared) = weak.upgrade() else { break };
                {
                    let mut m = lock(&shared);
                    if !m.imbue.tracker_open || m.imbue_shutdown || commands.is_closed() {
                        m.imbue.tracker_open = false;
                        break;
                    }
                    enqueue(&mut m, &commands, false);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        })
        .is_err()
    {
        if let Some(shared) = failed.upgrade() {
            lock(&shared).imbue.equipment.feedback =
                Some("Automatic refresh unavailable; use Refresh equipment");
        }
    }
}
