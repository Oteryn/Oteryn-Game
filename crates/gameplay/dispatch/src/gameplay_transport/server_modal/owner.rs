use crate::foundation::GameSessionId;
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
    selection: Option<super::store::Selection>,
    notification: Option<Dialog>,
    submitted: Option<Answer>,
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
    pub(super) fn offer_store(
        &self,
        root: Root,
        mut dialog: Dialog,
        selection: super::store::Selection,
    ) -> Option<Vec<u8>> {
        if root.1 == 0 || !dialog.valid() {
            return None;
        }
        let mut state = self.state.lock().ok()?;
        if let Some(entry) = state.entries.get(&root) {
            if let Some(pending) = &entry.pending {
                return (entry.selection.as_ref() == Some(&selection))
                    .then(|| Reply::Dialog(pending.clone()).encode().ok())
                    .flatten();
            }
            if entry.notification.is_some() {
                return None;
            }
        } else if state.entries.len() >= 256 {
            return None;
        }
        state.next = state.next.checked_add(1)?;
        dialog.id = state.next;
        let bytes = Reply::Dialog(dialog.clone()).encode().ok()?;
        let entry = state.entries.entry(root).or_default();
        entry.pending = Some(dialog);
        entry.selection = Some(selection);
        entry.answered = None;
        entry.submitted = None;
        Some(bytes)
    }

    pub(super) fn store_selection(
        &self,
        root: Root,
        answer: Answer,
    ) -> Option<super::store::Selection> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(&root)?;
        if !entry.pending.as_ref()?.accepts(answer)
            || entry.submitted.is_some_and(|sent| sent != answer)
        {
            return None;
        }
        let selection = entry.selection.clone()?;
        if answer.button_id != 2 {
            entry.submitted = Some(answer);
        }
        Some(selection)
    }

    pub(super) fn finish_store(&self, root: Root, answer: Answer) -> Option<Vec<u8>> {
        let mut state = self.state.lock().ok()?;
        let entry = state.entries.get_mut(&root)?;
        if entry.selection.is_none()
            || !entry.pending.as_ref()?.accepts(answer)
            || entry.submitted.is_some_and(|sent| sent != answer)
        {
            return None;
        }
        entry.pending = None;
        entry.selection = None;
        entry.submitted = None;
        entry.answered = Some(answer);
        Reply::Answered(answer).encode().ok()
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
            COMMAND_TYPE_SERVER_MODAL_QUERY if payload.is_empty() => {
                if let Some(dialog) = state
                    .entries
                    .get(&root)
                    .and_then(|entry| entry.pending.as_ref())
                {
                    return Reply::Dialog(dialog.clone()).encode().ok();
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
                entry.pending = Some(dialog.clone());
                Reply::Dialog(dialog)
            }
            COMMAND_TYPE_SERVER_MODAL_ANSWER => {
                let answer = Answer::decode(payload).ok()?;
                let entry = state.entries.get_mut(&root)?;
                if entry.answered == Some(answer) {
                    return Reply::Answered(answer).encode().ok();
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
                entry.answered = Some(answer);
                Reply::Answered(answer)
            }
            _ => return None,
        };
        reply.encode().ok()
    }
}
