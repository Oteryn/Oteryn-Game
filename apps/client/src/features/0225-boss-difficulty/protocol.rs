use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn boss_difficulty_initialize(&self, selected: bool) {
        lock(&self.0).boss_difficulty.selected = selected;
    }
    pub(in crate::play) fn boss_difficulty_result(&self, result: &GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        if oteryn_client_ui::boss_difficulty::apply_result(&mut mailbox.boss_difficulty, result)
            && let Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
                boss_key,
                boss_creature_key,
                ..
            }) = &mailbox.boss_difficulty.reply
        {
            let key = boss_key.clone();
            let creature = boss_creature_key.clone();
            let reply = mailbox.boss_difficulty.reply.clone();
            if let Some(reply) = reply {
                mailbox
                    .boss_encounter
                    .difficulty_windows
                    .insert(creature, reply.clone());
                mailbox.boss_encounter.difficulty_windows.insert(key, reply);
            }
        }
    }
}
