use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::boss_encounter::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
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
            { lock(&self.mailbox).boss_encounter.cooldowns_open = true; }
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
