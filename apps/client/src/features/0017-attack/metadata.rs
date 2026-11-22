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
impl Mailbox {
    pub(in crate::play) fn party_summons(
        &self,
    ) -> std::collections::BTreeMap<oteryn_session::EntityRef, oteryn_session::EntityRef> {
        self.party
            .state
            .iter()
            .flat_map(|p| &p.members)
            .flat_map(|member| {
                let owner = member
                    .list
                    .first()
                    .filter(|entry| !entry.summon)
                    .map(|entry| entry.entity);
                member
                    .list
                    .iter()
                    .filter(|entry| entry.summon)
                    .filter_map(move |entry| owner.map(|master| (entry.entity, master)))
            })
            .collect()
    }
}
impl PlayLink {
    pub fn filtered_battle_rows(&self, index: usize) -> Vec<oteryn_session::WorldSpatialEntity> {
        let mut mailbox = lock(&self.mailbox);
        if index >= mailbox.battle_lists.preferences.len() {
            return Vec::new();
        }
        refresh(&mut mailbox, &self.commands);
        mailbox.filtered_battle_rows(index)
    }
}
impl Mailbox {
    pub(in crate::play) fn filtered_battle_rows(
        &self,
        index: usize,
    ) -> Vec<oteryn_session::WorldSpatialEntity> {
        let mailbox = self;
        let list = &mailbox.battle_lists.preferences[index];
        let summons = list.hide_summons.then(|| mailbox.party_summons());
        let icons = &mailbox.properties.party_skull.icons;
        list.rows_with_metadata(
            &mailbox.battle.visible_actors,
            mailbox.wire.own_native(),
            &mailbox.battle.metadata.values,
        )
        .into_iter()
        .filter(|row| {
            if summons
                .as_ref()
                .is_some_and(|summons| summons.contains_key(&row.entity))
            {
                return false;
            }
            if row.kind != oteryn_session::EntityKind::Player
                || (!list.hide_party && !list.hide_non_skull)
            {
                return true;
            }
            let icon = icons.iter().find(|icon| icon.entity == row.entity);
            !(list.hide_party && icon.is_some_and(|icon| (3..=10).contains(&icon.shield))
                || list.hide_non_skull && icon.is_none_or(|icon| icon.skull == 0))
        })
        .collect()
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
