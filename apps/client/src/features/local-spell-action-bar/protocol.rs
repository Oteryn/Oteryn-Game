use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{ActorVitals, SessionEvent};

pub(in crate::play) fn vitals_events(mailbox: &mut Mailbox, events: &[SessionEvent]) {
    for event in events {
        match event {
            SessionEvent::ActorCooldowns(delta) => {
                mailbox
                    .spell_bar
                    .replace_cooldowns(Some(delta.value.clone()));
            }
            SessionEvent::GatedSnapshotReplaced(delta) if delta.value.domain_id == 20 => {
                mailbox.spell_bar.replace_cooldowns(
                    oteryn_session::decode_actor_cooldowns(&delta.value.payload).ok(),
                );
            }
            _ => {}
        }
        if let SessionEvent::ActorVitals(delta) = event {
            mailbox.apply_death_vitals(delta.value);
        }
    }
}

impl Outbox {
    pub(in crate::play) fn spell_bar_initialize(&self, cooldowns: Option<Vec<u8>>) {
        let mut mailbox = lock(&self.0);
        mailbox.spell_bar = Default::default();
        mailbox.spell_bar.replace_cooldowns(
            cooldowns.and_then(|bytes| oteryn_session::decode_actor_cooldowns(&bytes).ok()),
        );
    }
    pub(in crate::play) fn vitals(&self, vitals: Option<ActorVitals>) {
        if let Some(vitals) = vitals {
            lock(&self.0).apply_death_vitals(vitals);
        }
    }
}

impl PlayLink {
    #[must_use]
    pub fn actor_vitals(&self) -> Option<ActorVitals> {
        lock(&self.mailbox).vitals
    }
}

impl Outbox {
    pub(in crate::play) fn hotkey_own_actor(
        &self,
        actor: Option<oteryn_session::WorldSpatialEntity>,
    ) {
        lock(&self.0).bar_actions.own_actor = actor;
    }
}

impl Outbox {
    pub(in crate::play) fn bar_item_cooldowns_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        let state = &mut mailbox.bar_actions;
        state.item_cooldowns_selected = selected;
        state.item_cooldowns.clear();
        state.item_cooldown_read = None;
        state.deferred_item_read = None;
        state.item_cooldown_rejected.clear();
    }
    pub(in crate::play) fn bar_item_cooldowns_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        let state = &mut mailbox.bar_actions;
        let Some(request) = &mut state.item_cooldown_read else {
            return;
        };
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        let items = accepted
            .then(|| oteryn_client_runtime::action_bar_cooldowns::decode_reply(&result.payload))
            .flatten();
        if let Some(items) = items
            && items
                .iter()
                .map(|item| item.definition)
                .eq(request.definitions.iter().copied())
        {
            let now = std::time::Instant::now();
            for item in items {
                state
                    .item_cooldowns
                    .insert(item.definition, (now, item.remaining_ms));
                state.item_cooldown_rejected.remove(&item.definition);
            }
            request.ready = true;
        } else {
            state
                .item_cooldown_rejected
                .extend(request.definitions.iter().copied());
            state.item_cooldown_read = None;
            state.deferred_item_read = None;
            mailbox.spell_bar.feedback = "Item cooldown data unavailable; no action sent".into();
        }
    }
}
