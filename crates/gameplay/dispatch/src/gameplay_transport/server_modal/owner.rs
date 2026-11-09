use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::server_modal::{
    Answer, COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY, Dialog, Reply,
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex};
type Root = (GameSessionId, u64);
#[derive(Default)]
struct Entry {
    pending: Option<Dialog>,
    seen: Option<[u8; 32]>,
    answered: Option<Answer>,
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
                let Some(mut dialog) = offer.take() else {
                    return Reply::None.encode().ok();
                };
                dialog.id = 1;
                let bytes = Reply::Dialog(dialog.clone()).encode().ok()?;
                let digest: [u8; 32] = Sha256::digest(&bytes).into();
                if state
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
                entry.seen = Some(digest);
                entry.pending = Some(dialog.clone());
                Reply::Dialog(dialog)
            }
            COMMAND_TYPE_SERVER_MODAL_ANSWER => {
                let answer = Answer::decode(payload).ok()?;
                let entry = state.entries.get_mut(&root)?;
                if entry.answered == Some(answer) {
                    return Reply::Answered(answer).encode().ok();
                }
                if !entry.pending.as_ref()?.accepts(answer) {
                    return None;
                }
                entry.pending = None;
                entry.answered = Some(answer);
                Reply::Answered(answer)
            }
            _ => return None,
        };
        reply.encode().ok()
    }
}
