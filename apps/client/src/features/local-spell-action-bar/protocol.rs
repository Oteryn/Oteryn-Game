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
                    .apply_cooldown_delta(&delta.value);
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
        let mut mailbox = lock(&self.0);
        let frame = mailbox.wire_frame();
        mailbox.bar_actions.own_actor = actor.map(|mut actor| {
            actor.position = frame.to_native(actor.position);
            actor
        });
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
    pub(in crate::play) fn bar_action_scope(&self) -> u64 {
        lock(&self.0).bar_actions.scope
    }
    pub(in crate::play) fn bar_item_cooldowns_result(
        &self,
        scope: u64,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        let state = &mut mailbox.bar_actions;
        if state.scope != scope {
            return;
        }
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

#[cfg(test)]
mod tests {
    use crate::play::{mailbox::lock, play_channel};
    use oteryn_session::{ActorCooldown, ActorCooldownKind, ActorCooldowns, AppliedDelta, SessionEvent};
    use std::num::NonZeroU32;

    fn push(outbox: &super::Outbox, revision: u64, rtt_ms: Option<u32>, entries: Vec<ActorCooldown>) {
        outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
            server_sequence: revision,
            base_revision: revision - 1,
            new_revision: revision,
            value: ActorCooldowns {
                server_now_ms: 1_000,
                rtt_ms,
                entries,
            },
        })]);
    }

    #[test]
    fn spell_cooldowns_keep_unchanged_attack_and_apply_keyed_tombstones() {
        let (link, _, outbox) = play_channel();
        let attack = ActorCooldown {
            kind: ActorCooldownKind::Group,
            id: 1,
            expires_at_ms: 5_000,
        };
        let exura = ActorCooldown { kind: ActorCooldownKind::Spell, id: 1, expires_at_ms: 2_000 };
        push(&outbox, 1, None, vec![attack]);
        push(&outbox, 2, None, vec![exura]);
        push(&outbox, 3, None, vec![ActorCooldown { expires_at_ms: 0, ..exura }]);
        let mailbox = lock(&link.mailbox);
        assert_eq!(
            mailbox.spell_bar.bar.cooldowns.as_ref().map(|state| state.entries.as_slice()),
            Some([attack].as_slice())
        );
    }

    #[test]
    fn spell_cooldowns_compensate_for_half_the_round_trip_on_correction() -> Result<(), &'static str> {
        let (link, _, outbox) = play_channel();
        let spell = NonZeroU32::new(1).ok_or("spell index")?;
        let entry = ActorCooldown {
            kind: ActorCooldownKind::Spell,
            id: spell.get(),
            expires_at_ms: 2_000,
        };
        push(&outbox, 1, None, vec![entry]);
        assert_eq!(lock(&link.mailbox).spell_bar.bar.remaining_ms(spell, &[]), 1_000);
        push(&outbox, 2, Some(200), vec![entry]);
        assert_eq!(lock(&link.mailbox).spell_bar.bar.remaining_ms(spell, &[]), 900);
        Ok(())
    }
}
