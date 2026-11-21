//! Exact source addEvent queue; Pod callbacks survive parent death.
use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::{
    PrimalMenaceEffect, WaveKind,
};
/// Deferred callbacks belong to the source room, not the parent's live actor.
/// In particular Pod and presentation closures survive the parent disappearing.
#[derive(Debug, Default, Clone)]
pub(super) struct DeferredCallbacks(pub(super) Vec<DeferredOccurrence>, u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeferredOccurrence {
    pub(super) id: u64,
    pub(super) effect: PrimalMenaceEffect,
    pub(super) replacement_removed: bool,
    original_due_ms: u64,
}
impl DeferredCallbacks {
    #[cfg(test)]
    pub(super) fn enqueue(&mut self, effect: PrimalMenaceEffect) -> Result<(), ()> {
        self.enqueue_at(effect, 0)
    }
    pub(super) fn enqueue_at(
        &mut self,
        effect: PrimalMenaceEffect,
        executed_at_ms: u64,
    ) -> Result<(), ()> {
        if self.0.len() >= 128 {
            return Err(());
        }
        let id = self.1.checked_add(1).ok_or(())?;
        self.0.push(DeferredOccurrence {
            id,
            effect,
            replacement_removed: false,
            original_due_ms: match effect {
                PrimalMenaceEffect::SpawnSparkle { at_ms, .. }
                | PrimalMenaceEffect::ScheduledSpawn { at_ms, .. } => at_ms,
                PrimalMenaceEffect::ReplaceWithFungosaurus { .. } => executed_at_ms,
            },
        });
        self.1 = id;
        Ok(())
    }
    pub(super) fn peek_due_through(
        &self,
        now_ms: u64,
        eligible_through_ms: u64,
    ) -> Result<Option<DeferredOccurrence>, ()> {
        Ok(self
            .0
            .iter()
            .filter(|queued| queued.original_due_ms <= now_ms.min(eligible_through_ms))
            .min_by_key(|queued| (queued.original_due_ms, queued.id))
            .copied())
    }
    pub(super) fn original_next_due(&self) -> Option<u64> {
        self.0.iter().map(|queued| queued.original_due_ms).min()
    }
    /// Read-only observation of source work that the real native Think enqueued.
    #[cfg(test)]
    pub(super) fn original_deadlines_for_test(&self) -> Vec<(u64, u64)> {
        self.0
            .iter()
            .map(|work| (work.id, work.original_due_ms))
            .collect()
    }
    #[cfg(test)]
    pub(super) fn retry(&mut self, occurrence: DeferredOccurrence) -> Result<(), ()> {
        if self.0.len() >= 128 || occurrence.id == 0 || occurrence.id > self.1 {
            return Err(());
        }
        let index = self.0.partition_point(|queued| queued.id < occurrence.id);
        if self
            .0
            .get(index)
            .is_some_and(|queued| queued.id == occurrence.id)
        {
            return Err(());
        }
        self.0.insert(index, occurrence);
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn take_due(&mut self, now_ms: u64, parent_live: bool) -> Vec<DeferredOccurrence> {
        let mut due = Vec::new();
        self.0.retain(|occurrence| {
            let at = match &occurrence.effect {
                PrimalMenaceEffect::SpawnSparkle { at_ms, .. }
                | PrimalMenaceEffect::ScheduledSpawn { at_ms, .. } => *at_ms,
                PrimalMenaceEffect::ReplaceWithFungosaurus { .. } => 0,
            };
            if at > now_ms {
                return true;
            }
            if parent_live
                || !matches!(
                    occurrence.effect,
                    PrimalMenaceEffect::ScheduledSpawn {
                        kind: WaveKind::Beast,
                        ..
                    }
                )
            {
                due.push(*occurrence);
            }
            false
        });
        due
    }
    /// Inspect the original occurrence without moving it across a Map await.
    #[cfg(test)]
    pub(super) fn peek_due(
        &mut self,
        now_ms: u64,
        parent_live: bool,
    ) -> Option<DeferredOccurrence> {
        self.peek_due_registered_through(now_ms, now_ms, parent_live)
    }
    pub(super) fn peek_due_registered_through(
        &mut self,
        now_ms: u64,
        cutoff_ms: u64,
        parent_registered: bool,
    ) -> Option<DeferredOccurrence> {
        self.0.retain(|occurrence| {
            parent_registered
                || occurrence.original_due_ms > now_ms.min(cutoff_ms)
                || !matches!(
                    occurrence.effect,
                    PrimalMenaceEffect::ScheduledSpawn {
                        kind: WaveKind::Beast,
                        ..
                    }
                )
        });
        self.peek_due_through(now_ms, cutoff_ms).ok().flatten()
    }
    /// Remove only after its physical source effect committed synchronously.
    pub(super) fn acknowledge(&mut self, id: u64) -> Result<(), ()> {
        let index = self
            .0
            .iter()
            .position(|occurrence| occurrence.id == id)
            .ok_or(())?;
        self.0.remove(index);
        Ok(())
    }
    pub(super) fn next_due(&self) -> Option<u64> {
        self.0
            .iter()
            .map(|occurrence| match &occurrence.effect {
                PrimalMenaceEffect::SpawnSparkle { .. }
                | PrimalMenaceEffect::ScheduledSpawn { .. } => occurrence.original_due_ms,
                PrimalMenaceEffect::ReplaceWithFungosaurus { .. } => 0,
            })
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult<T = ()> = Result<T, String>;
    use crate::foundation::MovementLocalPosition;
    use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::{
        PrimalMenaceEffect, WaveKind,
    };

    #[test]
    fn primal_native_due_cutoff_uses_original_execution_not_retry_wake() -> TestResult {
        let at = MovementLocalPosition {
            x: 33558,
            y: 32759,
            floor: 15,
        };
        let mut callbacks = DeferredCallbacks::default();
        callbacks
            .enqueue_at(
                PrimalMenaceEffect::ReplaceWithFungosaurus {
                    beast: 17,
                    position: at,
                },
                600,
            )
            .map_err(|_| "actual replacement execution".to_owned())?;
        callbacks
            .enqueue_at(
                PrimalMenaceEffect::SpawnSparkle {
                    position: at,
                    at_ms: 700,
                },
                400,
            )
            .map_err(|_| "later original source deadline".to_owned())?;
        assert_eq!(
            callbacks.next_due(),
            Some(0),
            "retry wake remains immediate"
        );
        assert!(
            callbacks
                .peek_due_through(1_000, 599)
                .map_err(|_| "no old work".to_owned())?
                .is_none()
        );
        let first = callbacks
            .peek_due_through(1_000, 600)
            .map_err(|_| "due-only owner must consume eligible original work".to_owned())?
            .ok_or("original replacement, despite wake0")?;
        assert_eq!(first.original_due_ms, 600);
        callbacks
            .acknowledge(first.id)
            .map_err(|_| "known source replacement".to_owned())?;
        assert_eq!(callbacks.next_due(), Some(700));
        assert!(
            callbacks
                .peek_due_through(1_000, 600)
                .map_err(|_| "later work retained".to_owned())?
                .is_none()
        );
        let later = callbacks
            .peek_due_through(1_000, 700)
            .map_err(|_| "real owner time stays1000".to_owned())?
            .ok_or("original700ms source work")?;
        assert_eq!(later.original_due_ms, 700);
        Ok(())
    }

    #[test]
    fn primal_native_deferred_retry_retains_original_creation_occurrence() -> TestResult {
        let effect = PrimalMenaceEffect::ScheduledSpawn {
            kind: WaveKind::Beast,
            position: MovementLocalPosition {
                x: 33558,
                y: 32759,
                floor: 15,
            },
            at_ms: 24_000,
        };
        let mut callbacks = DeferredCallbacks::default();
        callbacks
            .enqueue(effect)
            .map_err(|_| "source addEvent".to_owned())?;
        let first = callbacks.take_due(24_000, true).remove(0);
        let id = first.id;
        callbacks
            .retry(first)
            .map_err(|_| "unknown Map retains original callback".to_owned())?;
        callbacks
            .enqueue(effect)
            .map_err(|_| "a distinct later source creation".to_owned())?;
        let retried = callbacks.take_due(25_000, true);
        assert_eq!(retried.len(), 2);
        assert_eq!(retried[0].id, id);
        assert_eq!(retried[0].effect, effect);
        assert_ne!(
            retried[1].id, id,
            "failed placement cannot alias a later literal creation"
        );
        Ok(())
    }

    #[test]
    fn primal_native_deferred_retained_replacement_stays_due_after_parent_death() -> TestResult {
        let mut callbacks = DeferredCallbacks::default();
        callbacks
            .enqueue(PrimalMenaceEffect::ReplaceWithFungosaurus {
                beast: 17,
                position: MovementLocalPosition {
                    x: 33558,
                    y: 32759,
                    floor: 15,
                },
            })
            .map_err(|_| "actual aged beast replacement".to_owned())?;
        let callback = callbacks.take_due(25_000, false).remove(0);
        callbacks
            .retry(callback)
            .map_err(|_| "unavailable map retains source replacement".to_owned())?;
        assert_eq!(
            callbacks.next_due(),
            Some(0),
            "a pending replacement must wake without a live parent profile"
        );
        Ok(())
    }

    #[test]
    fn primal_native_deferred_callbacks_keep_exact_due_and_pod_after_boss_death() -> TestResult {
        let at = MovementLocalPosition {
            x: 33558,
            y: 32759,
            floor: 15,
        };
        let mut callbacks = DeferredCallbacks::default();
        callbacks
            .enqueue(PrimalMenaceEffect::ScheduledSpawn {
                kind: WaveKind::Pod,
                position: at,
                at_ms: 24_000,
            })
            .map_err(|_| "bounded source event".to_owned())?;
        callbacks
            .enqueue(PrimalMenaceEffect::ScheduledSpawn {
                kind: WaveKind::Beast,
                position: at,
                at_ms: 24_000,
            })
            .map_err(|_| "bounded source event".to_owned())?;
        assert!(callbacks.take_due(23_999, false).is_empty());
        let due = callbacks.take_due(24_000, false);
        assert_eq!(
            due.len(),
            1,
            "source Pod callback ignores dead parent; beast checks parent"
        );
        assert_eq!(
            due[0].effect,
            PrimalMenaceEffect::ScheduledSpawn {
                kind: WaveKind::Pod,
                position: at,
                at_ms: 24_000
            }
        );
        assert!(
            callbacks.take_due(24_001, true).is_empty(),
            "callbacks execute once"
        );
        Ok(())
    }
}
