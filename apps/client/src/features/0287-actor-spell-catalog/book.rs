pub use super::preferences::SpellbookFilter;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::spell_catalog::spellbook_details::{PAGE_SIZE, Page, Query};
#[derive(Debug, Clone, Default)]
pub struct SpellbookState {
    pub archive: super::archive::ArchiveBindings,
    pub archive_edit: u64,
    pub selected: bool,
    pub open: bool,
    pub native_open: bool,
    pub pending: Option<Query>,
    pub filter: SpellbookFilter,
    pub page: Option<Page>,
    pub confirmed_filter: Option<SpellbookFilter>,
    pub feedback: String,
    pub edit: u64,
}
impl SpellbookState {
    fn finish(&mut self, accepted: bool, bytes: &[u8]) {
        let Some(query) = self.pending.take() else {
            return;
        };
        let Some(page) = accepted
            .then(|| Page::decode(bytes))
            .flatten()
            .filter(|p| p.page == query.page && p.entries.iter().all(|e| query.matches(e)))
        else {
            self.feedback =
                "The spellbook query failed; the last confirmed page is retained.".into();
            return;
        };
        self.page = Some(page);
        self.confirmed_filter = Some(SpellbookFilter {
            search: query.search,
            vocation: query.vocation,
            premium: query.premium,
            min_level: query.min_level,
            max_level: query.max_level,
            group: query.group,
        });
        self.feedback.clear();
    }
    pub fn text(&self, row: usize) -> String {
        let Some(page) = &self.page else {
            return if self.pending.is_some() {
                "Loading spellbook…".into()
            } else {
                self.feedback.clone()
            };
        };
        let mut text = format!(
            "{} matching spells. Page {} of {}.\r\n{}",
            page.total,
            page.page + 1,
            usize::from(page.total).div_ceil(PAGE_SIZE).max(1),
            self.feedback
        );
        if self.pending.is_some() {
            text.push_str("\r\nLoading; showing the previous confirmed page.");
        }
        if self.confirmed_filter.as_ref() != Some(&self.filter) {
            text.push_str("\r\nThe displayed page belongs to the previous confirmed filters.");
        }
        if let Some(entry) = page.entries.get(row) {
            use std::fmt::Write;
            let spell = &entry.spell;
            let _ = write!(
                text,
                "\r\n\r\n{} / {}\r\nLevel {} · Mana {}{} · Soul {} · Rune magic level {}\r\nPremium required: {} · Learned: {} · Eligible now: {}\r\nRecovery categories: {}\r\nVocations: {}\r\nEligibility does not promise a legal target or an expired cooldown.",
                spell.name,
                spell.words,
                spell.level,
                spell.mana,
                if spell.mana_percent { "%" } else { "" },
                spell.soul,
                entry.magic_level,
                entry.premium,
                spell.learned,
                spell.available,
                spell
                    .cooldown_groups
                    .iter()
                    .map(|&group| usize::try_from(group)
                        .ok()
                        .and_then(|i| GROUPS.get(i))
                        .copied()
                        .unwrap_or("Other recovery category"))
                    .collect::<Vec<_>>()
                    .join(", "),
                (0..11)
                    .filter(|i| entry.vocations & (1 << i) != 0)
                    .map(|i| VOCATIONS[i + 1])
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        text
    }
}
pub const GROUPS: &[&str] = &[
    "Any recovery category",
    "Offense",
    "Healing",
    "Utility",
    "Special",
    "Nature bursts",
    "Crippling",
    "Focus",
    "Heavy beams",
    "Stances",
    "Finishing strikes",
    "Virtues",
];
pub const VOCATIONS: &[&str] = &[
    "Any vocation",
    "Untrained",
    "Druid",
    "Elder Druid",
    "Sorcerer",
    "Master Sorcerer",
    "Knight",
    "Elite Knight",
    "Paladin",
    "Royal Paladin",
    "Monk",
    "Exalted Monk",
];
impl PlayLink {
    pub fn spellbook_state(&self) -> SpellbookState {
        lock(&self.mailbox).spellbook_details.clone()
    }
    pub fn spellbook_request(&self, filter: SpellbookFilter, page: u16) -> bool {
        request(&self.commands, &self.mailbox, filter, page)
    }
    pub fn spellbook_open(&self) -> bool {
        let filter = {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.spellbook_details.selected {
                return false;
            }
            mailbox.spellbook_details.open = true;
            mailbox.spellbook_details.filter.clone()
        };
        self.spellbook_request(filter, 0);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::book_native::open_window(self);
        }
        true
    }
    pub fn sync_spellbook_filter(&self, filter: &mut SpellbookFilter, edit: u64) -> Option<u64> {
        let mut mailbox = lock(&self.mailbox);
        let state = &mut mailbox.spellbook_details;
        if state.edit > edit {
            *filter = state.filter.clone();
            Some(state.edit)
        } else {
            if state.filter != *filter {
                state.filter = filter.clone();
            }
            state.edit = edit;
            None
        }
    }
}
pub(super) fn request(
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    mailbox: &crate::play::mailbox::SharedMailbox,
    filter: SpellbookFilter,
    page: u16,
) -> bool {
    let query = filter.query(page);
    let Some(command) = oteryn_client_ui::spellbook_details::protocol::command(&query) else {
        return false;
    };
    let mut mailbox = lock(mailbox);
    let state = &mut mailbox.spellbook_details;
    if !state.selected || state.pending.is_some() {
        return false;
    }
    if state.filter != filter {
        state.filter = filter;
        state.edit = crate::play::feature_0017_attack::view::next_edit();
    }
    state.pending = Some(query);
    if commands
        .send(crate::combat_input::PlayCommand::Feature(command))
        .is_err()
    {
        state.pending = None;
        state.feedback = "The session has ended".into();
        return false;
    }
    true
}

impl Outbox {
    pub(in crate::play) fn spellbook_details_initialize(&self, selected: bool) {
        lock(&self.0).spellbook_details = SpellbookState {
            selected,
            ..Default::default()
        };
    }
    pub(in crate::play) fn spellbook_details_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .spellbook_details
            .finish(accepted, &result.payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{CommandStatus, GatedCommandResult};
    #[test]
    fn spellbook_routes_queries_retains_confirmed_pages_and_persists_filters()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(oteryn_session::CLIENT_SUPPORTED_CAPABILITIES.contains(&449));
        let (link, mut commands, outbox) = crate::play::play_channel();
        assert!(!link.spellbook_open());
        outbox.spellbook_details_initialize(true);
        assert!(link.spellbook_open());
        let _ = commands.try_recv()?;
        let page = Page {
            page: 0,
            total: 1,
            entries: vec![
                oteryn_client_runtime::spell_catalog::spellbook_details::DetailedSpell {
                    spell: oteryn_session::spell_catalog::SpellCatalogEntry {
                        cast_index: 19,
                        name: "Mending".into(),
                        words: "mend".into(),
                        level: 20,
                        mana: 15,
                        mana_percent: false,
                        soul: 0,
                        learned: true,
                        available: true,
                        cooldown_groups: vec![2],
                        rune_item: None,
                    },
                    vocations: 1 << 9,
                    premium: true,
                    magic_level: 0,
                },
            ],
        };
        outbox.spellbook_details_result(&GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: page.encode().ok_or("encode")?,
        });
        assert_eq!(link.spellbook_state().page, Some(page.clone()));
        let details = link.spellbook_state().text(0);
        assert!(details.contains("Mending / mend"));
        assert!(details.contains("Premium required: true"));
        assert!(details.contains("Recovery categories: Healing"));
        assert!(details.contains("Vocations: Monk"));
        let filter = SpellbookFilter {
            vocation: 10,
            premium: 2,
            min_level: 20,
            max_level: 200,
            group: 2,
            search: "mend".into(),
        };
        assert!(link.spellbook_request(filter.clone(), 0));
        assert!(!link.spellbook_request(filter.clone(), 0));
        let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("wrong command".into());
        };
        assert_eq!(command.capability, 449);
        assert_eq!(command.kind, 733);
        assert_eq!(Query::decode(&command.payload), Some(filter.query(0)));
        outbox.spellbook_details_result(&GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Rejected,
            payload: vec![],
        });
        assert_eq!(link.spellbook_state().page, Some(page));
        assert!(
            link.spellbook_state()
                .text(0)
                .contains("previous confirmed filters")
        );
        let path =
            std::env::temp_dir().join(format!("spellbook-preferences-{}.json", std::process::id()));
        let mut settings = crate::play::SettingsState::new(Some(path.clone()));
        settings.sync_spellbook_filter(&link);
        assert_eq!(settings.preferences.spellbook_filter, filter);
        assert_eq!(
            crate::play::SettingsState::new(Some(path.clone()))
                .preferences
                .spellbook_filter,
            filter
        );
        std::fs::remove_file(path)?;
        outbox.logged_out();
        assert!(!link.spellbook_state().selected);
        assert!(link.spellbook_state().page.is_none());
        Ok(())
    }
}
