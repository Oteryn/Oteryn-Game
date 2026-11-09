use crate::play::{Input, PlayView};
use oteryn_client_runtime::spell::ACTION_BAR_SLOTS;
use std::num::NonZeroU32;

impl PlayView {
    /// A spell hotkey: casts the locally bound server spell index. Entries past the bar are ignored and
    /// at most one press per bar slot waits for the next tick.
    pub fn cast(&mut self, spell: NonZeroU32) {
        if usize::try_from(spell.get()).is_ok_and(|slot| slot <= ACTION_BAR_SLOTS) {
            self.cursor_spell = None;
            self.queue(Input::Cast(spell));
        }
    }
}

pub(in crate::play) fn send_cast(link: &crate::play::PlayLink, spell: NonZeroU32) {
    use crate::play::mailbox::lock;
    let bound = {
        let mut mailbox = lock(&link.mailbox);
        crate::play::feature_0287_actor_spell_catalog::view::resolve_archive_bindings(&mut mailbox);
        mailbox.spell_bar.bar.spell(spell)
    };
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
        let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
        let applied = configure(&mut mailbox.spell_bar, fields);
        if applied {
            crate::play::feature_0287_actor_spell_catalog::view::manual_archive_bindings(
                &mut mailbox,
            );
        }
        applied
    }
}
pub(in crate::play) fn configure(
    panel: &mut oteryn_client_ui::spell_action_bar::ActionBarPanel,
    fields: &[String],
) -> bool {
    if fields.len() != 4 && fields.len() != ACTION_BAR_SLOTS {
        return false;
    }
    let mut next = panel.bar.clone();
    for (slot, text) in fields.iter().enumerate() {
        let spell = if text.trim().is_empty() {
            None
        } else {
            let Ok(id) = text.trim().parse::<NonZeroU32>() else {
                panel.feedback = "Enter positive server cast indices".into();
                return false;
            };
            Some(id)
        };
        if !next.bind(slot, spell) {
            panel.feedback = "Cast index is out of range".into();
            return false;
        }
    }
    panel.bar = next;
    panel.feedback = "Bindings applied".into();
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
        if configure(&mut mailbox.spell_bar, fields) {
            crate::play::feature_0287_actor_spell_catalog::view::manual_archive_bindings(mailbox);
        }
        return;
    }
    if mailbox.world_input_blocked() {
        return;
    }
    let Some(slot) = u32::try_from(action).ok().and_then(NonZeroU32::new) else {
        return;
    };
    crate::play::feature_0287_actor_spell_catalog::view::resolve_archive_bindings(mailbox);
    if let Some(spell) = mailbox.spell_bar.bar.spell(slot) {
        let _ = commands.send(crate::combat_input::PlayCommand::Combat(
            crate::combat_input::CombatCommand::Cast(spell),
        ));
    }
}

#[cfg(windows)]
pub(in crate::play) fn edit_slot(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    action: usize,
    fields: &[String],
) {
    let Some(slot) = fields
        .first()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .filter(|slot| (1..=ACTION_BAR_SLOTS).contains(slot))
    else {
        mailbox.spell_bar.feedback = "Select a slot in the row".into();
        return;
    };
    if action == 2 {
        submit(mailbox, commands, slot, &[]);
        return;
    }
    let index = if action == 1 {
        None
    } else {
        let Some(index) = fields
            .get(1)
            .and_then(|text| text.trim().parse::<u32>().ok())
        else {
            mailbox.spell_bar.feedback = "Select a cast index from the live catalogue".into();
            return;
        };
        Some(index)
    };
    mailbox.spell_bar.feedback =
        if crate::play::feature_0287_actor_spell_catalog::view::assign_cast_index(
            mailbox,
            slot - 1,
            index,
        ) {
            "Saved slot assignment".into()
        } else {
            "That spell is absent from the live catalogue".into()
        };
}
