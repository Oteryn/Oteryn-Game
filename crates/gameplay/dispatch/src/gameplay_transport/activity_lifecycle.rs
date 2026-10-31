//! The existing scoped owners drive boss deadlines and completed-room cleanup.
//! All returns use retained native/durable tokens; this pass invents neither
//! client command ids nor assignments. Each Ground pass is one bounded step.
use super::kill_reward::DrainOutcome;
use super::native_activity::NativeActivityOutcome;
use super::{ComposedFreshAdmission, activity_runtime::ScopeKey};
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::item_decay_retire::ActivityGroundRetirement;
use crate::foundation::ScopeOwnershipGeneration;
use std::collections::BTreeSet;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn activity_lifecycle_pass(&self) -> Result<(), ()> {
        let mut deferred = self.drain_native_activity_attempts().await.is_err();
        deferred |= self.drain_native_return_attempts().await.is_err();
        let owners = self.activity_runtimes.activity_owners()?;
        let mut drained = BTreeSet::new();

        // Complete already captured lethals before reading a room's deadline.
        // Native return seals this same queue under the actor lock, closing the
        // capture race between this pre-drain and the durable timeout CAS.
        for owner in &owners {
            let view = self.runtime_view(owner);
            let sessions = match owner.runtime.lock().await.activity_participant_sessions() {
                Ok(sessions) => sessions,
                Err(_) => {
                    deferred = true;
                    continue;
                }
            };
            let mut principals = BTreeSet::new();
            {
                let attack = owner.states.attack.lock().await;
                for session in sessions {
                    principals.insert(session);
                    principals.extend(attack.kills().proficiency_principals(session));
                }
            }
            let mut ready = true;
            for principal in principals {
                if view.drain_kill_rewards(principal).await == DrainOutcome::Deferred {
                    deferred = true;
                    ready = false;
                }
            }
            if ready {
                drained.insert(ScopeKey::new(owner.scope, owner.generation));
            }
        }

        let handoffs = ActivityHandoffOwner::new(self.root.clone());
        let mut after = None;
        loop {
            let page = handoffs
                .due_boss_rooms_after(self.character, self.holder, self.world_id, after)
                .await
                .map_err(|_| ())?;
            after = page.next_after;
            for room in page.rooms {
                let key = ScopeKey::new(room.scope, room.generation.get());
                if !drained.contains(&key) {
                    continue;
                }
                let Some(owner) = self
                    .activity_runtimes
                    .get(room.scope, room.generation.get())
                else {
                    continue;
                };
                let sessions = match owner.runtime.lock().await.activity_participant_sessions() {
                    Ok(sessions) => sessions,
                    Err(_) => {
                        deferred = true;
                        continue;
                    }
                };
                let view = self.runtime_view(&owner);
                for session in sessions {
                    let Some(admitted) = self.activity_runtimes.connection(session) else {
                        deferred = true;
                        continue;
                    };
                    let admitted = self.activity_runtimes.current_connection(admitted);
                    if admitted.item_fence.is_none_or(|fence| {
                        fence.runtime_scope != room.scope
                            || fence.scope_ownership_generation != room.generation
                    }) {
                        deferred = true;
                        continue;
                    }
                    match view.return_due_activity(admitted, room.reason).await {
                        Ok(NativeActivityOutcome::Committed(commit)) => {
                            self.publish_activity_commit(&commit)?
                        }
                        // A captured defeat can have superseded a timeout page.
                        // The transaction's typed refusal remains authoritative.
                        Ok(NativeActivityOutcome::Refused(_)) => {}
                        Err(()) => deferred = true,
                    }
                }
            }
            if after.is_none() {
                break;
            }
        }

        // All participants, including offline/dead/frozen actors, must be gone
        // in both owners. Ground is never cleared from a caller's room flag.
        for owner in self.activity_runtimes.activity_owners()? {
            let _transition = self.activity_runtimes.transitions.lock().await;
            if !owner
                .runtime
                .lock()
                .await
                .activity_retirement_empty()
                .map_err(|_| ())?
            {
                continue;
            }
            if self
                .prune_completed_activity_entries(owner.scope, owner.generation)
                .await
                .is_err()
                || self
                    .prune_completed_activity_returns(owner.scope, owner.generation)
                    .await
                    .is_err()
            {
                deferred = true;
                continue;
            }
            let generation = ScopeOwnershipGeneration::new(owner.generation).map_err(|_| ())?;
            let Some(proof) = handoffs
                .retirement_ready(self.character, self.holder, owner.scope, generation)
                .await
                .map_err(|_| ())?
            else {
                continue;
            };
            let permit = match owner.lane.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    deferred = true;
                    continue;
                }
            };
            {
                let runtime = owner.runtime.lock().await;
                let attack = owner.states.attack.lock().await;
                if !runtime.activity_retirement_empty().map_err(|_| ())?
                    || !attack.kills().activity_retirement_drained()
                {
                    deferred = true;
                    continue;
                }
            }
            match self
                .root
                .retire_activity_ground_step(&permit, self.character, self.holder, &proof)
                .await
            {
                Ok(ActivityGroundRetirement::Progress(_)) => {}
                Ok(ActivityGroundRetirement::Complete) => {
                    // The hook prunes before locking and synchronously retires
                    // plus unregisters under one native lock: no cancellation
                    // can strand an invalidated owner still in the registry.
                    self.retire_and_unregister_activity(owner.scope, owner.generation)
                        .await?;
                }
                Err(_) => deferred = true,
            }
        }
        if deferred { Err(()) } else { Ok(()) }
    }
}
