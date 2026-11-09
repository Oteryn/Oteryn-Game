//! A live capture keeps the runtime unavailable until its quest commit is known.
use crate::durability::quest_state::QuestTransitionOutcome;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use std::cell::Cell;

pub(in crate::gameplay_transport) struct CaptureCommitGuard<'a> {
    runtime: &'a mut ChannelRuntimeV1,
    capture: bool,
    uncertain: Cell<bool>,
}

impl<'a> CaptureCommitGuard<'a> {
    pub(in crate::gameplay_transport) fn new(
        runtime: &'a mut ChannelRuntimeV1,
        capture: bool,
    ) -> Self {
        Self {
            runtime,
            capture,
            uncertain: Cell::new(false),
        }
    }

    pub(in crate::gameplay_transport) fn runtime(&self) -> &ChannelRuntimeV1 {
        self.runtime
    }

    /// Interior state only: the proof's immutable runtime borrow stays in force.
    pub(in crate::gameplay_transport) fn arm(&self) {
        self.uncertain.set(self.capture);
    }

    pub(in crate::gameplay_transport) fn finish(
        &mut self,
        outcome: &QuestTransitionOutcome,
        target: Option<ExactActorRef>,
        key: Option<&str>,
    ) -> Result<(), ()> {
        if self.capture
            && matches!(
                outcome,
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
            )
        {
            let (Some(target), Some(key)) = (target, key) else {
                return Err(());
            };
            self.runtime
                .remove_mount_taming_subject(target, key)
                .map_err(|_| ())?;
        }
        self.uncertain.set(false);
        Ok(())
    }
}

impl Drop for CaptureCommitGuard<'_> {
    fn drop(&mut self) {
        if self.uncertain.get() {
            self.runtime.retire_owner_cycle();
        }
    }
}
