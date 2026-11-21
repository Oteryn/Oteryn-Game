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
    pub fn boss_encounter_difficulty_request(
        &self,
        intent: oteryn_client_ui::boss_difficulty::Intent,
        difficulty: u16,
    ) {
        enqueue_difficulty(&mut lock(&self.mailbox), &self.commands, intent, difficulty);
    }
    pub fn boss_encounter_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn boss_encounter_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::boss_encounter::cooldowns_key(events) {
            self.boss_encounter_request(Intent::Refresh);
            lock(&self.mailbox).boss_encounter.cooldowns_open = true;
            return true;
        }
        if !oteryn_client_ui::boss_encounter::open_key(events) {
            return false;
        }
        self.boss_encounter_request(Intent::Refresh);
        lock(&self.mailbox).boss_encounter.open = true;
        true
    }
}
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.boss_encounter;
    if panel
        .fields
        .iter()
        .any(|value| !oteryn_client_ui::modal_dialogs::TextEditor::accepts(value, 512))
        || !oteryn_client_ui::modal_dialogs::TextEditor::accepts(&panel.browser_search, 512)
    {
        panel.feedback = Some("A form field exceeds its UTF-8 byte limit or contains invalid text");
        return;
    }
    if intent == Intent::CatalogueNext
        && panel.catalogue_query.as_ref().is_none_or(|query| {
            query.search != panel.browser_search.trim()
                || query.category != panel.browser_category.server_filter()
        })
    {
        panel.feedback = Some("Browse again after changing boss catalogue filters");
        return;
    }
    if let Some(command) = request(panel, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.boss_encounter.pending = false;
        mailbox.boss_encounter.feedback = Some(tr!("The session has ended"));
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::boss_encounter::BossEncounterPanel,
    field: usize,
    value: String,
) {
    // Keep invalid input for whole-form validation instead of reusing a stale value.
    if !panel.pending
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

/// The integrated Archfoe controls use the same server context as the standalone window.
pub(in crate::play) fn enqueue_difficulty(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: oteryn_client_ui::boss_difficulty::Intent,
    difficulty: u16,
) {
    let Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
        boss_key,
        boss_creature_key,
        ..
    }) = &mailbox.boss_difficulty.reply
    else {
        return;
    };
    if mailbox.boss_encounter.fields[0] != *boss_key
        && mailbox.boss_encounter.fields[0] != *boss_creature_key
    {
        return;
    }
    mailbox.boss_difficulty.difficulty = difficulty;
    if let Some(command) =
        oteryn_client_ui::boss_difficulty::request(&mut mailbox.boss_difficulty, intent)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.boss_difficulty.pending = false;
        mailbox.boss_difficulty.feedback = Some(tr!("The session has ended"));
    }
}
