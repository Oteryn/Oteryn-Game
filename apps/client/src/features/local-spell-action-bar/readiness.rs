//! Shared readiness for multiaction activation and next-action labels.
use super::{BarAction, Mailbox, NonZeroU32, item_source};

pub(super) fn action_ready(mailbox: &Mailbox, action: &BarAction) -> bool {
    let entry = match action {
        BarAction::Spell { name, words, .. } => mailbox
            .spell_catalog
            .catalog
            .entries()
            .iter()
            .find(|entry| entry.name == *name && entry.words == *words),
        BarAction::Use {
            definition,
            sub_type,
            ..
        } => {
            if item_source(mailbox, *definition, *sub_type).is_none()
                || mailbox.world_context.pending
            {
                return false;
            }
            mailbox
                .spell_catalog
                .catalog
                .entries()
                .iter()
                .find(|entry| entry.rune_item == Some(definition.get()))
        }
        BarAction::Equip { definition, .. } => {
            return mailbox.hotkey_equip.items.contains(&definition.get())
                && !mailbox.hotkey_equip.pending;
        }
        _ => return false,
    };
    let Some(entry) = entry else {
        return match action {
            BarAction::Use { definition, .. } => mailbox
                .bar_actions
                .item_cooldowns
                .get(&definition.get())
                .is_some_and(|(received, remaining)| {
                    u64::try_from(received.elapsed().as_millis()).unwrap_or(u64::MAX) >= *remaining
                }),
            _ => false,
        };
    };
    if !entry.available {
        return false;
    }
    if matches!(action, BarAction::Spell { .. })
        && entry.rune_item.is_none()
        && let Some(vitals) = mailbox.vitals
    {
        let mana = if entry.mana_percent {
            u64::from(vitals.max_mana) * u64::from(entry.mana) / 100
        } else {
            u64::from(entry.mana)
        };
        if u64::from(vitals.mana) < mana || vitals.soul < entry.soul {
            return false;
        }
    }
    let elapsed = mailbox
        .spell_bar
        .cooldown_received_at
        .map_or(0, |received| {
            u64::try_from(received.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
    NonZeroU32::new(entry.cast_index).is_some_and(|index| {
        mailbox
            .spell_bar
            .bar
            .remaining_ms(index, &entry.cooldown_groups)
            <= elapsed
    })
}
