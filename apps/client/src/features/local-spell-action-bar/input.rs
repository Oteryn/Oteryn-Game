use crate::play::{Input, PlayView};
use crate::spell::SPELL_SLOTS;
use std::num::NonZeroU32;

impl PlayView {
    /// A spell hotkey: casts the locally bound server spell index. Entries past the bar are ignored and
    /// at most one press per bar slot waits for the next tick.
    pub fn cast(&mut self, spell: NonZeroU32) {
        if spell.get() <= u32::from(SPELL_SLOTS) {
            self.cursor_spell = None;
            self.queue(Input::Cast(spell));
        }
    }
}

pub(in crate::play) fn send_cast(link: &crate::play::PlayLink, spell: NonZeroU32) {
    use crate::play::mailbox::lock;
    let bound = lock(&link.mailbox).spell_bar.bar.spell(spell);
    if let Some(bound) = bound {
        link.request_combat(crate::combat_input::CombatCommand::Cast(bound));
    }
}

impl crate::play::PlayLink {
    pub fn spell_bar_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::spell_action_bar::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox).spell_bar.open = true;
        }
        true
    }
    pub fn spell_bar_bind(&self, fields: &[String]) -> bool {
        configure(
            &mut crate::play::mailbox::lock(&self.mailbox).spell_bar,
            fields,
        )
    }
}
pub(in crate::play) fn configure(
    panel: &mut oteryn_client_ui::spell_action_bar::ActionBarPanel,
    fields: &[String],
) -> bool {
    if fields.len() != 4 {
        return false;
    }
    let mut next = panel.bar.clone();
    for (slot, text) in fields.iter().enumerate() {
        let spell = if text.trim().is_empty() {
            None
        } else {
            let Ok(id) = text.trim().parse::<NonZeroU32>() else {
                panel.feedback = tr!("Enter positive server cast indices").into();
                return false;
            };
            Some(id)
        };
        if !next.bind(slot, spell) {
            panel.feedback = tr!("Cast index is out of range").into();
            return false;
        }
    }
    panel.bar = next;
    panel.feedback = tr!("Bindings applied").into();
    true
}

#[cfg(any(windows, test))]
pub(in crate::play) fn submit(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    action: usize,
    fields: &[String],
) {
    if action == 0 {
        configure(&mut mailbox.spell_bar, fields);
        return;
    }
    if mailbox.world_input_blocked() {
        return;
    }
    let Some(slot) = u32::try_from(action).ok().and_then(NonZeroU32::new) else {
        return;
    };
    if let Some(spell) = mailbox.spell_bar.bar.spell(slot) {
        let _ = commands.send(crate::combat_input::PlayCommand::Combat(
            crate::combat_input::CombatCommand::Cast(spell),
        ));
    }
}
