use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::boss_encounter::{BossCategory, BossEncounterPanel, Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn boss_encounter_browse(&self, search: &str, category: BossCategory, previous: bool) {
        browse(
            &mut lock(&self.mailbox).boss_encounter,
            search,
            category,
            previous,
        );
    }

    pub fn boss_slot_select_boss(&self, previous: bool) {
        lock(&self.mailbox)
            .boss_encounter
            .select_slot_boss(previous);
    }
    pub fn boss_slot_select_slot(&self) {
        lock(&self.mailbox).boss_encounter.select_slot();
    }

    pub fn boss_encounter_edit(&self, field: usize, value: String) {
        edit(&mut lock(&self.mailbox).boss_encounter, field, value);
    }
    pub fn boss_encounter_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn boss_encounter_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::boss_encounter::cooldowns_key(events) {
            self.boss_encounter_request(Intent::Refresh);
            #[cfg(windows)]
            super::view::cooldown_window(self);
            #[cfg(not(windows))]
            {
                lock(&self.mailbox).boss_encounter.cooldowns_open = true;
            }
            return true;
        }
        if !oteryn_client_ui::boss_encounter::open_key(events) {
            return false;
        }
        self.boss_encounter_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).boss_encounter.open = true;
        }
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.boss_encounter, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.boss_encounter.pending = false;
        mailbox.boss_encounter.feedback = Some("The session has ended");
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::boss_encounter::BossEncounterPanel,
    field: usize,
    value: String,
) {
    if !panel.pending
        && value.len() <= 512
        && !value.contains('\0')
        && let Some(slot) = panel.fields.get_mut(field)
    {
        *slot = value;
    }
}

pub(in crate::play) fn browse(
    panel: &mut BossEncounterPanel,
    search: &str,
    category: BossCategory,
    previous: bool,
) {
    if !panel.selected || panel.pending || search.len() > 512 || search.contains('\0') {
        return;
    }
    panel.boss_search = search.to_owned();
    panel.browser_search = search.to_owned();
    panel.browser_category = category;
    panel.select_boss(previous);
}
