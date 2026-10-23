//! Small test-only wrapper around the existing think fixture; no duplicate actor harness.
pub(in crate::ai_think) fn actor() -> crate::foundation::ExactActorRef {
    super::rat()
}
