use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{ActorVitals, SessionEvent};

pub(in crate::play) fn vitals_events(mailbox: &mut Mailbox, events: &[SessionEvent]) {
    for event in events {
        match event {
            SessionEvent::ActorCooldowns(delta) => {
                mailbox.spell_bar.bar.cooldowns = Some(delta.value.clone());
                mailbox.spell_bar.cooldown_received_at = Some(std::time::Instant::now());
            }
            SessionEvent::GatedSnapshotReplaced(delta) if delta.value.domain_id == 20 => {
                mailbox.spell_bar.bar.cooldowns =
                    oteryn_session::decode_actor_cooldowns(&delta.value.payload).ok();
                mailbox.spell_bar.cooldown_received_at = Some(std::time::Instant::now());
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
        mailbox.spell_bar.bar.cooldowns =
            cooldowns.and_then(|bytes| oteryn_session::decode_actor_cooldowns(&bytes).ok());
        mailbox.spell_bar.cooldown_received_at = Some(std::time::Instant::now());
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
