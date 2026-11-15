use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayLink, mailbox::lock},
};
use oteryn_client_ui::server_info::{self, Query};
impl PlayLink {
    pub fn server_info_request(&self, query: Query) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = server_info::request(&mut mailbox.server_info, query.clone())
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.server_info.pending.retain(|q| q != &query);
        }
    }
    pub(in crate::play) fn server_info_catalog(&self) {
        let ids: Vec<_> = lock(&self.mailbox)
            .spell_catalog
            .catalog
            .entries()
            .iter()
            .map(|s| s.cast_index)
            .collect();
        for chunk in ids.chunks(16) {
            self.server_info_request(Query::Spells(chunk.to_vec()));
        }
    }
}
impl Outbox {
    pub(in crate::play) fn server_info_initial_commands(
        &self,
    ) -> Vec<oteryn_client_runtime::raw_feature::FeatureCommand> {
        let ids: Vec<_> = lock(&self.0)
            .spell_catalog
            .catalog
            .entries()
            .iter()
            .map(|s| s.cast_index)
            .collect();
        ids.chunks(16)
            .map(|ids| Query::Spells(ids.to_vec()))
            .filter_map(|q| self.server_info_command(q))
            .collect()
    }
    pub(in crate::play) fn server_info_command(
        &self,
        query: Query,
    ) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
        server_info::request(&mut lock(&self.0).server_info, query)
    }
    pub(in crate::play) fn server_info_initialize(&self, selected: bool) {
        let mut m = lock(&self.0);
        m.server_info = server_info::Panel {
            selected,
            ..Default::default()
        };
        m.quest.titles.clear();
        m.spell_catalog.info.clear();
        m.spellbook_details.info.clear();
    }
    pub(in crate::play) fn server_info_result(
        &self,
        query: &Query,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mut m = lock(&self.0);
        let Some(reply) = server_info::apply_result(&mut m.server_info, query, result) else {
            return;
        };
        let m = &mut *m;
        server_info::protocol::observe(reply, &mut m.quest, &mut m.spell_catalog);
        m.spellbook_details.info.clone_from(&m.spell_catalog.info);
    }
    pub(in crate::play) fn server_info_spell_commands(
        &self,
    ) -> Vec<oteryn_client_runtime::raw_feature::FeatureCommand> {
        let mut m = lock(&self.0);
        let ids: Vec<_> = m
            .spellbook_details
            .page
            .as_ref()
            .map(|p| {
                p.entries
                    .iter()
                    .map(|e| e.spell.cast_index)
                    .filter(|id| !m.spellbook_details.info.contains_key(id))
                    .collect()
            })
            .unwrap_or_default();
        ids.chunks(16)
            .filter_map(|ids| server_info::request(&mut m.server_info, Query::Spells(ids.to_vec())))
            .collect()
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_server_info_initialize(&self) {
        self.server_info_initialize(true);
    }
    pub(crate) fn bevy_server_info_result(
        &self,
        query: &Query,
        result: &oteryn_session::GatedCommandResult,
    ) {
        self.server_info_result(query, result);
    }
}
