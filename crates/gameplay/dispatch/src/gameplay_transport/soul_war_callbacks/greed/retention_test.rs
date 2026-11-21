//! Source-path observation ports; they cannot create a callback or a timer.
use super::*;

pub(super) fn after_effect(
    states: &mut ChannelSpellStates,
    effect: GreedEffect,
) -> Result<Option<Arc<tokio::sync::Notify>>, ()> {
    let retention = &mut states.soul_war_callbacks.greed_work;
    let pending = retention.pending.as_mut().ok_or(())?;
    pending.committed = pending.committed.checked_add(1).ok_or(())?;
    if retention.pause_after_spawn.as_ref().is_some_and(|(key, _)| {
        matches!(effect, GreedEffect::SpawnMonster { creature_key, .. } if creature_key == *key)
    })
    {
        return Ok(retention
            .pause_after_spawn
            .take()
            .map(|(_, notify)| notify));
    }
    if matches!(effect, GreedEffect::PlayerHealthLoss { .. }) {
        return Ok(retention.pause_after_health.take());
    }
    Ok(None)
}

pub(super) fn before_think(
    states: &mut ChannelSpellStates,
    occurrence: ThinkOccurrence,
) -> Option<Arc<tokio::sync::Notify>> {
    let pending = &mut states.soul_war_callbacks.greed_work.pause_before_think;
    if pending
        .as_ref()
        .is_some_and(|(actor, _)| *actor == occurrence.actor)
    {
        pending.take().map(|(_, notify)| notify)
    } else {
        None
    }
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct Snapshot {
    pub(in crate::gameplay_transport) original_due: u64,
    pub(in crate::gameplay_transport) sampled_at: u64,
    pub(in crate::gameplay_transport) occurrence: Option<ThinkOccurrence>,
    pub(in crate::gameplay_transport) remaining: Vec<GreedEffect>,
    pub(in crate::gameplay_transport) committed: usize,
}

impl SoulWarCallbacks {
    pub(in crate::gameplay_transport) fn pause_greed_before_think_for_test(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<Arc<tokio::sync::Notify>, ()> {
        if self.greed_work.pause_before_think.is_some() {
            return Err(());
        }
        let notify = Arc::new(tokio::sync::Notify::new());
        self.greed_work.pause_before_think = Some((actor, Arc::clone(&notify)));
        Ok(notify)
    }

    pub(in crate::gameplay_transport) fn pause_greed_after_health_for_test(
        &mut self,
    ) -> Result<Arc<tokio::sync::Notify>, ()> {
        if self.greed_work.pause_after_health.is_some() {
            return Err(());
        }
        let notify = Arc::new(tokio::sync::Notify::new());
        self.greed_work.pause_after_health = Some(Arc::clone(&notify));
        Ok(notify)
    }

    pub(in crate::gameplay_transport) fn pause_greed_after_spawn_for_test(
        &mut self,
        creature: &'static str,
    ) -> Result<Arc<tokio::sync::Notify>, ()> {
        if self.greed_work.pause_after_spawn.is_some() {
            return Err(());
        }
        let notify = Arc::new(tokio::sync::Notify::new());
        self.greed_work.pause_after_spawn = Some((creature, Arc::clone(&notify)));
        Ok(notify)
    }

    pub(in crate::gameplay_transport) fn greed_pending_for_test(&self) -> Option<Snapshot> {
        let pending = self.greed_work.pending.as_ref()?;
        Some(Snapshot {
            original_due: pending.original_due,
            sampled_at: pending.sampled_at,
            occurrence: match pending.origin {
                Origin::Think { occurrence, .. } => Some(occurrence),
                Origin::Timer => None,
            },
            remaining: pending.effects.iter().copied().collect(),
            committed: pending.committed,
        })
    }

    pub(in crate::gameplay_transport) fn greed_observed_for_test(
        &self,
        actor: ExactActorRef,
    ) -> Option<(u64, bool, Option<ThinkOccurrence>)> {
        self.observed
            .iter()
            .find(|entry| entry.actor == actor)
            .map(|entry| (entry.id, entry.death_observed, entry.last_think))
    }

    pub(in crate::gameplay_transport) fn greed_timer_due_for_test(&self) -> Option<u64> {
        self.greed
            .next_due_ms()
            .and_then(|due| due.checked_mul(1_000))
    }

    pub(in crate::gameplay_transport) fn greed_spawn_due_for_test(&self, key: &str) -> Option<u64> {
        // Inspect copies of actual queued DATA. This never mutates, installs or
        // executes a timer in the physical owner, including recovered entry delays.
        let mut clock = self.greed.clone();
        while let Some((due, effect)) = clock.take_next_due_through(u64::MAX) {
            if matches!(effect, GreedEffect::SpawnMonster { creature_key, .. } if creature_key == key)
            {
                return due.checked_mul(1_000);
            }
        }
        None
    }
}
