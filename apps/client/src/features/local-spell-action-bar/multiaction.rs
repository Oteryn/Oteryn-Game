//! Multiaction slot activation and the resumed fallback cast after a refused spell.
use super::*;

pub(super) fn activate_multi(
    mailbox: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    slot: usize,
    start: usize,
    actions: Vec<BarAction>,
) {
    let count = actions.len();
    let Some((index, action)) = actions
        .into_iter()
        .enumerate()
        .skip(start)
        .find(|(_, action)| action_ready(mailbox, action, false))
    else {
        mailbox.spell_bar.feedback =
            "All multiaction slots are cooling down or unavailable".into();
        return;
    };
    let next = index + 1;
    let fallback = (matches!(action, BarAction::Spell { .. }) && next < count).then(|| {
        Box::new(BarFallback {
            slot,
            next,
            scope: mailbox.bar_actions.scope,
            edit: mailbox.bar_actions.edit,
            cursor: mailbox.bar_actions.cursor.clone(),
        })
    });
    activate_action(mailbox, commands, action, fallback);
}

impl crate::play::Outbox {
    pub(in crate::play) fn resume_bar_cast(&self, fallback: BarFallback) -> Option<PlayCommand> {
        let mut mailbox = lock(&self.0);
        if !bar_cast_current(&mailbox, &fallback) {
            return None;
        }
        let BarAction::Multi { actions } = mailbox.bar_actions.saved.0.get(&fallback.slot)?.clone()
        else {
            return None;
        };
        let (commands, mut queued) = tokio::sync::mpsc::unbounded_channel();
        let cursor = std::mem::replace(&mut mailbox.bar_actions.cursor, fallback.cursor);
        activate_multi(&mut mailbox, &commands, fallback.slot, fallback.next, actions);
        mailbox.bar_actions.cursor = cursor;
        queued.try_recv().ok()
    }
}

pub(in crate::play) fn bar_cast_current(mailbox: &Mailbox, fallback: &BarFallback) -> bool {
    !mailbox.world_input_blocked()
        && !mailbox.activity_reset_commands
        && mailbox.bar_actions.scope == fallback.scope
        && mailbox.bar_actions.edit == fallback.edit
}
