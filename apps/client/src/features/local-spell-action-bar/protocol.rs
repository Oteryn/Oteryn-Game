use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{ActorVitals, SessionEvent};

pub(in crate::play) fn vitals_events(mailbox: &mut Mailbox, events: &[SessionEvent]) {
    for event in events {
        match event {
            SessionEvent::SpellParametersSelected => {
                mailbox.bar_actions.parameters_selected = true;
            }
            SessionEvent::ActivityScope(delta) => {
                mailbox.bar_actions.parameters_selected = delta.value.selection.contains(
                    &oteryn_session::actor_spell_v2::CAPABILITY_ACTOR_SPELL_PARAMETERS_V2,
                );
                mailbox.bar_actions.parameter_feedback = None;
            }
            SessionEvent::ParameterSpellCast { result, position } => {
                let position = mailbox.wire_frame().to_native(*position);
                let (text, effect) =
                    oteryn_client_ui::spell_action_bar::parameter_cast_feedback(result, position);
                mailbox.bar_actions.parameter_feedback = Some((text, std::time::Instant::now()));
                super::super::feature_0023_presentation::protocol::absorb_visuals(
                    &mut mailbox.pushed.visuals,
                    effect,
                );
            }
            SessionEvent::ActorCooldowns(delta) => {
                mailbox.spell_bar.apply_cooldowns(&delta.value);
            }
            SessionEvent::ActorVitals(delta) => mailbox.apply_death_vitals(delta.value),
            SessionEvent::GiftOfLife { cooldowns, vitals } => {
                mailbox.spell_bar.apply_cooldowns(&cooldowns.value);
                mailbox.apply_death_vitals(vitals.value);
            }
            _ => {}
        }
    }
}

impl Outbox {
    pub(in crate::play) fn spell_bar_initialize(
        &self,
        cooldowns: Option<oteryn_session::ActorCooldownUpdate>,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.spell_bar = Default::default();
        mailbox.bar_actions.parameters_selected = false;
        mailbox.bar_actions.parameter_feedback = None;
        if let Some(cooldowns) = cooldowns {
            mailbox.spell_bar.apply_cooldowns(&cooldowns);
        }
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

pub(super) fn send_parameter_cast(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    spell: std::num::NonZeroU32,
    parameter: Option<String>,
) {
    let message = if let Some(command) =
        oteryn_client_runtime::spell::parameter_cast(spell, parameter)
    {
        if commands.send(crate::combat_input::PlayCommand::Feature(command)).is_ok() {
            return;
        }
        "Session unavailable"
    } else {
        "Spell parameter must fit in 128 UTF-8 bytes and contain no control characters."
    };
    mailbox.bar_actions.parameter_feedback = Some((message.into(), std::time::Instant::now()));
}

/// House lists and Magic Rope return private results even with no parameter.
pub(super) fn parameterless_cast_selected(mailbox: &Mailbox, words: &str) -> bool {
    mailbox.bar_actions.parameters_selected
        && ["aleta grav", "aleta sio", "aleta som", "exani tera"]
            .iter()
            .any(|incantation| incantation.eq_ignore_ascii_case(words))
}

pub(super) fn send_text_cast(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    text: String,
) {
    if mailbox.bar_actions.parameters_selected
        && let Some((spell, Some(parameter))) =
            super::view::spoken_spell(&text, mailbox.spell_catalog.catalog.entries())
        && let Some(spell) = std::num::NonZeroU32::new(spell.cast_index)
    {
        send_parameter_cast(mailbox, commands, spell, Some(parameter.to_owned()));
    } else {
        crate::play::feature_0007_chat::input::enqueue_hotkey_text(mailbox, commands, text);
    }
}
