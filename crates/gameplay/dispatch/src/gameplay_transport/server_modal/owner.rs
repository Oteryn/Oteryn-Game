use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::server_modal::input::{
    DialogInput, InputAnswer, InputValue, encode_dialog_with_input,
};
use oteryn_protocol_oteryn::server_modal::{
    Answer, COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY, Dialog, Reply,
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex};
pub(super) type Root = (GameSessionId, u64);
#[derive(Default)]
struct Entry {
    pending: Option<Dialog>,
    seen: Option<[u8; 32]>,
    answered: Option<Answer>,
    answered_terminal: bool,
    terminal_replies: bool,
    selection: Option<super::rules::Selection>,
    notification: Option<Dialog>,
    submitted: Option<Answer>,
    /// Capability 490: dialog replies carry their input block on this connection.
    input_replies: bool,
    /// The input the pending dialog asks for.
    input: Option<DialogInput>,
    /// The value and "don't show again" choice of the last checked 490 answer.
    answer_input: Option<AnswerInput>,
}

/// What a capability 490 answer adds to the button and choice, handed to the dialog's owner:
/// the checked value and the "don't show again" choice.
pub(super) type AnswerInput = (Option<InputValue>, bool);

impl Entry {
    /// The pending dialog as this connection receives it.
    fn dialog_reply(&self, dialog: &Dialog) -> Option<Vec<u8>> {
        if self.input_replies {
            encode_dialog_with_input(dialog, self.input.as_ref()).ok()
        } else {
            Reply::Dialog(dialog.clone()).encode().ok()
        }
    }
}
#[derive(Default)]
struct State {
    next: u32,
    entries: HashMap<Root, Entry>,
}
#[derive(Default)]
pub(in crate::gameplay_transport) struct ModalHub {
    state: Mutex<State>,
}
impl ModalHub {
    pub(super) fn offer_rule(
        &self,
        root: Root,
        dialog: Dialog,
        selection: super::rules::Selection,
    ) -> Option<Vec<u8>> {
        self.offer_rule_with_input(root, dialog, None, selection)
    }

    /// [`Self::offer_rule`] for a dialog that asks for an amount or text (capability 490). The
    /// owner receives the checked value with the selection; a connection without 490 is never
    /// offered an input dialog.
    pub(super) fn offer_rule_with_input(
        &self,
        root: Root,
        mut dialog: Dialog,
        input: Option<DialogInput>,
        selection: super::rules::Selection,
    ) -> Option<Vec<u8>> {
        if root.1 == 0 || !dialog.valid() || input.as_ref().is_some_and(|input| !input.valid()) {
            return None;
        }
        let mut state = self.state.lock().ok()?;
        if matches!(&selection, super::rules::Selection::Bed(_))
            && !state
                .entries
                .get(&root)
                .is_some_and(|entry| entry.terminal_replies)
        {
            return None;
        }
        if let Some(entry) = state.entries.get(&root) {
            if let Some(pending) = &entry.pending {
                return (entry.selection.as_ref() == Some(&selection))
                    .then(|| entry.dialog_reply(pending))
                    .flatten();
            }
            if entry.notification.is_some() || (input.is_some() && !entry.input_replies) {
                return None;
            }
        } else if state.entries.len() >= 256 || input.is_some() {
            return None;
        }
        state.next = state.next.checked_add(1)?;
        dialog.id = state.next;
        let entry = state.entries.entry(root).or_default();
        let previous = std::mem::replace(&mut entry.input, input);
        let Some(bytes) = entry.dialog_reply(&dialog) else {
            entry.input = previous;
            return None;
        };
        entry.pending = Some(dialog);
        entry.selection = Some(selection);
        entry.answered = None;
        entry.answered_terminal = false;
        entry.submitted = None;
        entry.answer_input = None;
        Some(bytes)
    }

    /// Capability 490 was selected on this connection: dialog replies carry the input block.
    pub(in crate::gameplay_transport) fn select_input(&self, root: Root) -> Option<()> {
        if root.1 == 0 {
            return None;
        }
        let mut state = self.state.lock().ok()?;
        if !state.entries.contains_key(&root) && state.entries.len() >= 256 {
            return None;
        }
        state.entries.entry(root).or_default().input_replies = true;
        Some(())
    }

    /// Check a capability 490 answer against the pending dialog's input before the V1 answer
    /// reaches its owner: a cancel button carries no value, an input dialog needs an accepted
    /// value and any other dialog none. A retry of a checked answer must repeat it exactly.
    pub(in crate::gameplay_transport) fn check_answer_input(
        &self,
        root: Root,
        answer: &InputAnswer,
    ) -> Option<()> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(&root)?;
        if !entry.input_replies {
            return None;
        }
        let recorded: AnswerInput = (answer.value.clone(), answer.dont_show_again);
        let Some(pending) = entry.pending.as_ref() else {
            return (entry.answered == Some(answer.answer)
                && entry.answer_input.as_ref() == Some(&recorded))
            .then_some(());
        };
        if pending.id != answer.answer.dialog_id {
            return None;
        }
        let cancelled = pending.cancel_button == Some(answer.answer.button_id);
        let valid = match (&entry.input, &answer.value) {
            (_, None) => cancelled || entry.input.is_none(),
            (Some(input), Some(value)) => !cancelled && input.accepts(value),
            (None, Some(_)) => false,
        };
        if !valid
            || entry
                .answer_input
                .as_ref()
                .is_some_and(|sent| entry.submitted.is_some() && *sent != recorded)
        {
            return None;
        }
        entry.answer_input = Some(recorded);
        Some(())
    }

    /// The owner's selection with the checked capability 490 value and "don't show again"
    /// choice of this answer (no value and `false` for a connection without 490).
    pub(super) fn rule_selection(
        &self,
        root: Root,
        answer: Answer,
    ) -> Option<(super::rules::Selection, Option<InputValue>, bool)> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(&root)?;
        if !entry.pending.as_ref()?.accepts(answer)
            || entry.submitted.is_some_and(|sent| sent != answer)
        {
            return None;
        }
        let (value, dont_show_again) = if entry.input_replies {
            entry.answer_input.clone()?
        } else {
            (None, false)
        };
        let selection = entry.selection.clone()?;
        if answer.button_id != 2 {
            entry.submitted = Some(answer);
        }
        Some((selection, value, dont_show_again))
    }

    pub(super) fn finish_rule(&self, root: Root, answer: Answer) -> Option<Vec<u8>> {
        self.finish(root, answer, false)
    }

    pub(super) fn finish_terminal_rule(&self, root: Root, answer: Answer) -> Option<Vec<u8>> {
        self.finish(root, answer, true)
    }

    fn finish(&self, root: Root, answer: Answer, terminal: bool) -> Option<Vec<u8>> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(&root)?;
        if entry.selection.is_none()
            || (terminal && !entry.terminal_replies)
            || (terminal && entry.pending.as_ref()?.cancel_button == Some(answer.button_id))
            || !entry.pending.as_ref()?.accepts(answer)
            || entry.submitted.is_some_and(|sent| sent != answer)
        {
            return None;
        }
        entry.pending = None;
        entry.selection = None;
        entry.submitted = None;
        entry.input = None;
        entry.answered = Some(answer);
        entry.answered_terminal = terminal;
        accepted(answer, terminal).encode().ok()
    }

    pub(super) fn accepts_terminal_reply(&self, root: Root) -> bool {
        self.state.lock().ok().is_some_and(|state| {
            state
                .entries
                .get(&root)
                .is_some_and(|entry| entry.terminal_replies)
        })
    }

    /// A proved refusal performed no mutation, so the player may correct or cancel it.
    pub(super) fn refused_rule(&self, root: Root, answer: Answer) -> Option<()> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(&root)?;
        if entry.submitted != Some(answer) || entry.selection.is_none() {
            return None;
        }
        entry.submitted = None;
        entry.answer_input = None;
        Some(())
    }

    pub(super) fn notify(&self, root: Root, dialog: Dialog) -> Option<()> {
        let mut state = self.state.lock().ok()?;
        state.entries.get_mut(&root)?.notification = Some(dialog);
        Some(())
    }
    pub(in crate::gameplay_transport) fn leave(&self, root: Root) {
        if let Ok(mut state) = self.state.lock() {
            state.entries.remove(&root);
        }
    }
    pub(in crate::gameplay_transport) fn execute(
        &self,
        root: Root,
        kind: u32,
        payload: &[u8],
        mut offer: Option<Dialog>,
    ) -> Option<Vec<u8>> {
        if root.1 == 0 {
            return None;
        }
        let mut state = self.state.lock().ok()?;
        let reply = match kind {
            COMMAND_TYPE_SERVER_MODAL_QUERY => {
                if oteryn_protocol_oteryn::server_modal::decode_query(payload).ok()? {
                    if !state.entries.contains_key(&root) && state.entries.len() >= 256 {
                        return None;
                    }
                    state.entries.entry(root).or_default().terminal_replies = true;
                }
                if let Some(entry) = state.entries.get(&root)
                    && let Some(dialog) = entry.pending.as_ref()
                {
                    return entry.dialog_reply(dialog);
                }
                let notification = state
                    .entries
                    .get_mut(&root)
                    .and_then(|entry| entry.notification.take());
                let notice = notification.is_none();
                let Some(mut dialog) = notification.or_else(|| offer.take()) else {
                    return Reply::None.encode().ok();
                };
                dialog.id = 1;
                let bytes = Reply::Dialog(dialog.clone()).encode().ok()?;
                let digest: [u8; 32] = Sha256::digest(&bytes).into();
                if notice
                    && state
                        .entries
                        .get(&root)
                        .is_some_and(|entry| entry.seen == Some(digest))
                {
                    return Reply::None.encode().ok();
                }
                if !state.entries.contains_key(&root) && state.entries.len() >= 256 {
                    return None;
                }
                state.next = state.next.checked_add(1)?;
                dialog.id = state.next;
                let entry = state.entries.entry(root).or_default();
                if notice {
                    entry.seen = Some(digest);
                }
                entry.input = None;
                entry.answer_input = None;
                entry.pending = Some(dialog.clone());
                return entry.dialog_reply(&dialog);
            }
            COMMAND_TYPE_SERVER_MODAL_ANSWER => {
                let answer = Answer::decode(payload).ok()?;
                let entry = state.entries.get_mut(&root)?;
                if entry.answered == Some(answer) {
                    return accepted(answer, entry.answered_terminal).encode().ok();
                }
                // Gameplay answers must finish through their rule callback, including
                // retries after an uncertain debit; they cannot fall back to an info ack.
                if entry.selection.is_some() {
                    return None;
                }
                if !entry.pending.as_ref()?.accepts(answer) {
                    return None;
                }
                entry.pending = None;
                entry.selection = None;
                entry.input = None;
                entry.answered = Some(answer);
                entry.answered_terminal = false;
                Reply::Answered(answer)
            }
            _ => return None,
        };
        reply.encode().ok()
    }
}

fn accepted(answer: Answer, terminal: bool) -> Reply {
    if terminal {
        Reply::AnsweredAndLoggedOut(answer)
    } else {
        Reply::Answered(answer)
    }
}
