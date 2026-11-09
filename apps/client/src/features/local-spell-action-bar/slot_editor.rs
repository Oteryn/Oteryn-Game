//! Native adapter for the 36-slot API reused from cg-options commit 252372ef3.
use crate::play::mailbox::Mailbox;
use std::num::NonZeroU32;

#[cfg(any(windows, test))]
pub(in crate::play) fn edit_slot(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    action: usize,
    fields: &[String],
) {
    let Some(slot) = fields
        .first()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .filter(|slot| (1..=oteryn_client_runtime::spell::ACTION_BAR_SLOTS).contains(slot))
    else {
        mailbox.spell_bar.feedback = "Enter a slot from 1 to 36 (12 slots per row)".into();
        return;
    };
    if action == 2 {
        super::submit(mailbox, commands, slot, &[]);
        return;
    }
    let cast = if action == 1 {
        None
    } else {
        let Some(cast) = fields
            .get(1)
            .and_then(|text| text.trim().parse::<NonZeroU32>().ok())
        else {
            mailbox.spell_bar.feedback = "Enter a positive server cast index".into();
            return;
        };
        Some(cast)
    };
    mailbox.spell_bar.feedback = if mailbox.spell_bar.bar.bind(slot - 1, cast) {
        "Slot assignment applied".into()
    } else {
        "Cast index out of range".into()
    };
}
