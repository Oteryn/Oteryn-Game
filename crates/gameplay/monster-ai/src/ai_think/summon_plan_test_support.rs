//! Read-only test views of the existing think and step owners; no alternate constructors.
use super::*;
impl CreatureAiTable {
    pub fn step_due_for_test(&self, actor: ExactActorRef) -> Option<u64> {
        self.entry(actor)?.motion.due
    }
    pub fn last_report_for_test(&self, actor: ExactActorRef) -> Option<&ThinkReport> {
        self.entry(actor)?.state.last.as_ref()
    }
}
