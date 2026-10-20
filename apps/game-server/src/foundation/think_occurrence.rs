use super::ExactActorRef;

/// §4.2: "a think occurrence is (creature `ExactActorRef`, per-actor think sequence number)."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThinkOccurrence {
    pub actor: ExactActorRef,
    pub sequence: u64,
}
