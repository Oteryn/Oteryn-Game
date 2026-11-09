//! A training batch retains selections, never credentials or game authority.
use super::CharacterRow;
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex, PoisonError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingSelection {
    pub character_id: String,
    pub channel_id: Option<String>,
}
#[derive(Debug, Default)]
struct State {
    remaining: VecDeque<TrainingSelection>,
    ready: bool,
}
#[derive(Debug, Clone, Default)]
pub struct TrainingQueue(Arc<Mutex<State>>);
impl TrainingQueue {
    pub(crate) fn start(
        &self,
        rows: &[CharacterRow],
        selections: Vec<TrainingSelection>,
    ) -> Option<TrainingSelection> {
        let mut seen = HashSet::new();
        if selections.is_empty()
            || selections.len() > rows.len()
            || !selections.iter().all(|selection| {
                seen.insert(selection.character_id.as_str())
                    && rows.iter().any(|row| {
                        row.available
                            && row.character.character_id == selection.character_id
                            && selection.channel_id.as_ref().is_none_or(|id| {
                                row.channels.iter().any(|channel| &channel.0 == id)
                            })
                    })
            })
        {
            return None;
        }
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.remaining = selections.into();
        state.ready = false;
        state.remaining.pop_front()
    }
    pub(crate) fn next(&self) -> Option<TrainingSelection> {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !state.ready {
            return None;
        }
        state.ready = false;
        state.remaining.pop_front()
    }
    pub fn remaining(&self) -> usize {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remaining
            .len()
    }
    pub(crate) fn finish_current(&self, confirmed: bool) -> bool {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if !confirmed {
            state.remaining.clear();
        }
        state.ready = confirmed && !state.remaining.is_empty();
        state.ready
    }
    pub(crate) fn clear(&self) {
        self.finish_current(false);
    }
}
