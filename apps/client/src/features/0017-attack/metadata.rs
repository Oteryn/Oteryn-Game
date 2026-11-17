//! Query only for enabled filters and apply observations to the current actor generations.
use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::server_mechanic::ServerMechanicResponse;
use std::time::Instant;
pub(in crate::play) fn refresh(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    if mailbox.battle.shutdown
        || mailbox.world_input_blocked()
        || !mailbox
            .battle_lists
            .preferences
            .iter()
            .any(|list| list.needs_metadata())
    {
        return;
    }
    let visible = mailbox
        .battle
        .rows
        .iter()
        .filter(|row| row.kind == oteryn_session::EntityKind::Player)
        .map(|row| row.entity)
        .collect::<Vec<_>>();
    if let Some(command) = oteryn_client_ui::battle::metadata_request(
        &mut mailbox.battle.metadata,
        &visible,
        Instant::now(),
    ) && commands
        .send(crate::combat_input::PlayCommand::ServerMechanic(command))
        .is_err()
    {
        mailbox.battle.metadata.send_failed();
    }
}
impl PlayLink {
    pub fn filtered_battle_rows(&self, index: usize) -> Vec<oteryn_session::WorldSpatialEntity> {
        let mut mailbox = lock(&self.mailbox);
        if index >= mailbox.battle_lists.preferences.len() {
            return Vec::new();
        }
        refresh(&mut mailbox, &self.commands);
        mailbox.battle_lists.preferences[index]
            .rows_with_metadata(&mailbox.battle.rows, &mailbox.battle.metadata.values)
    }
}
impl Outbox {
    pub(in crate::play) fn battle_metadata_initialize(&self, selected: bool) {
        lock(&self.0).battle.metadata =
            oteryn_client_ui::battle::PlayerMetadataState::new(selected);
    }
    pub(in crate::play) fn battle_metadata_result(&self, response: &ServerMechanicResponse) {
        let mut mailbox = lock(&self.0);
        mailbox.battle.metadata.finish(response);
        let visible = mailbox
            .battle
            .rows
            .iter()
            .map(|row| row.entity)
            .collect::<Vec<_>>();
        mailbox
            .battle
            .metadata
            .values
            .retain(|entity, _| visible.contains(entity));
    }
}
