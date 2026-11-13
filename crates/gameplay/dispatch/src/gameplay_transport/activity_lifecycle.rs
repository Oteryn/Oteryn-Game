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
    /// Every lifecycle step, the durable traversals included, as the tests drive it directly.
    #[cfg(test)]
    pub(super) async fn activity_lifecycle_pass(&self) -> Result<(), ()> {
        self.activity_lifecycle_pass_with(true).await
    }

    /// One lifecycle pass. `durable_scan` gates the two ownerless database traversals (departed
    /// recovery and due boss rooms): each takes every admission relation in EXCLUSIVE mode, so the
    /// autonomous cadence runs them on [`super::activity_cycles::ActivityDurableScan`] rather than
    /// on every 50 ms heartbeat, which kept the single durability holder busy (NBHOG-1).
    pub(super) async fn activity_lifecycle_pass_with(&self, durable_scan: bool) -> Result<(), ()> {
        let start = self.background_start("activity-lifecycle", 4);
        let mut deferred = false;
        for index in (start..4).chain(0..start) {
            deferred |= self
                .background_turn("activity-lifecycle", index + 1, async {
                    match index {
                        0 if durable_scan => self.recover_departed_activity_pass().await,
                        0 => Ok(()),
                        1 => self.drain_native_activity_attempts().await,
                        2 => self.drain_native_return_attempts().await,
                        _ => self.activity_room_lifecycle_pass(durable_scan).await,
                    }
                })
                .await
                .is_err();
        }
        if deferred { Err(()) } else { Ok(()) }
    }

    async fn activity_room_lifecycle_pass(&self, durable_scan: bool) -> Result<(), ()> {
        let mut deferred = false;
        let mut owners = self.activity_runtimes.activity_owners()?;
        owners.sort_by_key(|owner| ScopeKey::new(owner.scope, owner.generation));
        let owner_start = self.background_start("activity-kill-owners", owners.len());
        owners.rotate_left(owner_start);
        let mut drained = BTreeSet::new();

        // Complete already captured lethals before reading a room's deadline.
        // Native return seals this same queue under the actor lock, closing the
        // capture race between this pre-drain and the durable timeout CAS.
        for (owner_index, owner) in owners.iter().enumerate() {
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
            let mut principals: Vec<_> = principals.into_iter().collect();
            let start = view.background_start("activity-kill-principals", principals.len());
            principals.rotate_left(start);
            for (index, principal) in principals.into_iter().enumerate() {
                if self
                    .background_turn(
                        "activity-kill-owners",
                        owner_start + owner_index + 1,
                        view.background_unit(
                            "activity-kill-principals",
                            start + index + 1,
                            view.drain_kill_rewards(principal),
                        ),
                    )
                    .await
                    == DrainOutcome::Deferred
                {
                    deferred = true;
                    ready = false;
                }
            }
            if ready {
                drained.insert(ScopeKey::new(owner.scope, owner.generation));
            }
        }

        let room_start = self.background_start("activity-room-phase", 2);
        if room_start == 1 {
            deferred |= self
                .background_turn(
                    "activity-room-phase",
                    0,
                    self.retire_registered_activity_units(),
                )
                .await
                .is_err();
        }
        let handoffs = ActivityHandoffOwner::new(self.root.clone());
        let mut after = *self
            .activity_runtimes
            .boss_rooms_after
            .lock()
            .map_err(|_| ())?;
        // A due room is acted on only for a drained registered owner: with none, the traversal
        // cannot return a room this pass would use.
        if durable_scan && !drained.is_empty() {
            loop {
                let (page, first_deferred) = self
                    .background_turn(
                        "activity-room-phase",
                        1,
                        self.root.with_background_operation(async {
                            let page = handoffs
                                .due_boss_rooms_after(
                                    self.character,
                                    self.holder,
                                    self.world_id,
                                    after,
                                )
                                .await
                                .map_err(|_| ())?;
                            *self
                                .activity_runtimes
                                .boss_rooms_after
                                .lock()
                                .map_err(|_| ())? = page
                                .rooms
                                .first()
                                .map(|room| room.handoff_id)
                                .or(page.next_after);
                            let first_deferred = if let Some(room) = page.rooms.first() {
                                self.return_due_room(room, &drained).await?
                            } else {
                                false
                            };
                            Ok::<_, ()>((page, first_deferred))
                        }),
                    )
                    .await?;
                after = page.next_after;
                deferred |= first_deferred;
                for room in page.rooms.iter().skip(1) {
                    deferred |= self
                        .root
                        .with_background_operation(self.return_due_room(room, &drained))
                        .await?;
                }
                if after.is_none() {
                    break;
                }
            }
        }

        if room_start == 0 {
            deferred |= self
                .background_turn(
                    "activity-room-phase",
                    0,
                    self.retire_registered_activity_units(),
                )
                .await
                .is_err();
        }
        if deferred { Err(()) } else { Ok(()) }
    }

    async fn retire_registered_activity_units(&self) -> Result<(), ()> {
        let mut deferred = false;
        let handoffs = ActivityHandoffOwner::new(self.root.clone());
        // All participants, including offline/dead/frozen actors, must be gone
        // in both owners. Ground is never cleared from a caller's room flag.
        let mut owners = self.activity_runtimes.activity_owners()?;
        owners.sort_by_key(|owner| ScopeKey::new(owner.scope, owner.generation));
        let start = self.background_start("activity-retire", owners.len());
        let count = owners.len();
        for index in (start..count).chain(0..start) {
            let owner = &owners[index];
            self.background_unit("activity-retire", index + 1, async {
                let _transition = self.activity_runtimes.transitions.lock().await;
                if !owner
                    .runtime
                    .lock()
                    .await
                    .activity_retirement_empty()
                    .map_err(|_| ())?
                {
                    return Ok(());
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
                    return Ok(());
                }
                let generation = ScopeOwnershipGeneration::new(owner.generation).map_err(|_| ())?;
                let Some(proof) = handoffs
                    .retirement_ready(self.character, self.holder, owner.scope, generation)
                    .await
                    .map_err(|_| ())?
                else {
                    return Ok(());
                };
                let permit = match owner.lane.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        deferred = true;
                        return Ok(());
                    }
                };
                {
                    let runtime = owner.runtime.lock().await;
                    let attack = owner.states.attack.lock().await;
                    if !runtime.activity_retirement_empty().map_err(|_| ())?
                        || !attack.kills().activity_retirement_drained()
                    {
                        deferred = true;
                        return Ok(());
                    }
                }
                // The transition lock excludes entrants, native retirement-empty
                // excludes all participants, and the durable witness seals return.
                // Join this Instance only; other Instances and the Channel keep serving.
                owner.states.kill_source_owner.stop_and_join().await;
                if !self.runtime_view(owner).finish_activity_daily_kills().await {
                    deferred = true;
                    return Ok(());
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
                Ok::<(), ()>(())
            })
            .await?;
        }
        if deferred { Err(()) } else { Ok(()) }
    }

    async fn return_due_room(
        &self,
        room: &crate::durability::activity_handoff::boss_room::ActivityDueRoom,
        drained: &BTreeSet<ScopeKey>,
    ) -> Result<bool, ()> {
        let mut deferred = false;
        let key = ScopeKey::new(room.scope, room.generation.get());
        if !drained.contains(&key) {
            return Ok(false);
        }
        let Some(owner) = self
            .activity_runtimes
            .get(room.scope, room.generation.get())
        else {
            return Ok(false);
        };
        let sessions = match owner.runtime.lock().await.activity_participant_sessions() {
            Ok(sessions) => sessions,
            Err(_) => {
                return Ok(true);
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
            match self
                .root
                .with_background_operation(view.return_due_activity(admitted, room.reason))
                .await
            {
                Ok(NativeActivityOutcome::Committed(commit)) => {
                    self.publish_activity_commit(&commit)?
                }
                // A captured defeat can have superseded a timeout page.
                // The transaction's typed refusal remains authoritative.
                Ok(NativeActivityOutcome::Refused(_)) => {}
                Err(()) => deferred = true,
            }
        }
        Ok(deferred)
    }
}
