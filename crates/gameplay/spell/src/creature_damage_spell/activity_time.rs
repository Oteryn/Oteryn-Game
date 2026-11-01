//! Read the original combat occurrence time for consumers such as Prey activity.
use super::{DamageSpellOwner, ExactActorRef, ScheduleList, SemanticTimeMicros};

impl DamageSpellOwner {
    /// Call only after the corresponding execution returned its successful receipt.
    /// A replay retains the first occurrence's time, so it cannot refresh activity.
    pub fn retained_activity_time(
        &self,
        issuer: ExactActorRef,
        list: ScheduleList,
        entry: usize,
    ) -> Option<SemanticTimeMicros> {
        self.casts
            .iter()
            .find(|memo| {
                memo.issuer == issuer
                    && memo.list == list
                    && memo.entry == entry
                    && memo.result.is_ok()
            })
            .map(|memo| memo.at)
    }
}
