//! Same canonical copied-death head; Item I/O runs only after volatile guards drop.
use super::*;
use crate::durability::creature_source_items::W01DeathGroundCallback as Callback;
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::gameplay_transport::source_death::PreparedSourceDeathAction;
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    Expiry,
}
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        512
    }
}
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
#[derive(Debug)]
struct Pending {
    action: PreparedSourceDeathAction,
    callback: Arc<Callback>,
    known: bool,
    expiry: Option<Arc<Callback>>,
    published: bool,
}
#[derive(Debug, Default)]
pub(super) struct NativeDeathItems {
    pending: Option<Pending>,
    timers: Option<OwnerTimerLane<Family, [u8; 32]>>,
    work: Vec<Arc<Callback>>,
}
impl NativeDeathItems {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.timers.as_ref()?.earliest_due().map(|at| at.get())
    }
    pub(super) fn prepare(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        action: &PreparedSourceDeathAction,
    ) -> Result<bool, ()> {
        if let Some(pending) = &self.pending {
            if pending.action == *action {
                return Ok(true);
            }
            if !pending.published {
                return Err(());
            }
        }
        if !matches!(
            (action.begin().definition_key(), action.event().name()),
            ("oteryn:creature.minion_of_gaz_haragoth", "MinionGazDeath")
                | ("oteryn:creature.corrupted_soul", "CorruptedSoul")
        ) {
            return Ok(false);
        }
        use crate::map::boot::BundleCollisionIndexExt;
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return Err(());
        };
        let at = runtime
            .read_actor_position(action.begin().actor())
            .map_err(|_| ())?
            .position();
        let tile = super::visual_position(at).ok_or(())?;
        let map = index.map();
        let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
            .map_err(|_| ())?
            .ok_or(())?;
        // CURRENT durable down-items are read under the same SQL Item lock.
        // A static WorldObject with these IDs needs its own existing transform
        // owner; never silently turn it into a portable Ground Item.
        if stack.entries.iter().any(|entry| {
            matches!(entry.source, crate::map::view::EntrySource::Base { .. })
                && matches!(entry.facts.appearance_id, 20121 | 23726 | 23727 | 23728)
        }) {
            return Err(());
        }
        let callback = Callback::from_source_death(
            runtime,
            room,
            native,
            action.begin(),
            action.event(),
            action.now_us(),
        )
        .map_err(|_| ())?
        .ok_or(())?;
        self.work.try_reserve(1).map_err(|_| ())?;
        self.pending = Some(Pending {
            action: action.clone(),
            callback: Arc::new(callback),
            known: false,
            expiry: None,
            published: false,
        });
        Ok(true)
    }
    pub(super) fn callback(
        &self,
        action: &PreparedSourceDeathAction,
    ) -> Result<Option<Arc<Callback>>, ()> {
        let pending = self
            .pending
            .as_ref()
            .filter(|pending| pending.action == *action)
            .ok_or(())?;
        Ok((!pending.known).then(|| Arc::clone(&pending.callback)))
    }
    pub(super) fn record(
        &mut self,
        action: &PreparedSourceDeathAction,
        expiry: Option<Callback>,
    ) -> Result<(), ()> {
        let pending = self
            .pending
            .as_mut()
            .filter(|pending| pending.action == *action)
            .ok_or(())?;
        if !pending.known {
            pending.known = true;
            pending.expiry = expiry.map(Arc::new);
        }
        Ok(())
    }
    pub(super) fn publish(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        action: &PreparedSourceDeathAction,
    ) -> Result<bool, ()> {
        let pending = self
            .pending
            .as_mut()
            .filter(|pending| pending.action == *action)
            .ok_or(())?;
        if !pending.known {
            return Ok(false);
        }
        if pending.published {
            return Ok(true);
        }
        if let Some(expiry) = &pending.expiry {
            if self.timers.is_none() {
                self.timers = Some(
                    OwnerTimerLane::for_generation(
                        runtime.binding().runtime_scope(),
                        runtime.binding().scope_generation(),
                        [(
                            Family::Expiry,
                            FamilyPolicy {
                                max_pending: 512,
                                catch_up: CatchUpPolicy::DeadlineState,
                            },
                        )],
                    )
                    .map_err(|_| ())?,
                );
            }
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            self.timers
                .as_mut()
                .ok_or(())?
                .schedule(
                    runtime.owner_fence().map_err(|_| ())?,
                    stamp,
                    Family::Expiry,
                    expiry.identity(),
                    None,
                    SemanticTimeMicros::from_micros(expiry.due_us()),
                )
                .map_err(|_| ())?;
            self.work.push(Arc::clone(expiry));
        }
        pending.published = true;
        Ok(true)
    }
    pub(super) fn due(
        &self,
        runtime: &ChannelRuntimeV1,
        cutoff: u64,
        now: u64,
    ) -> Option<Arc<Callback>> {
        self.work
            .iter()
            .filter(|callback| callback.current_for(runtime))
            .filter_map(|callback| {
                self.timers
                    .as_ref()?
                    .occurrence_due(Family::Expiry, callback.identity())
                    .filter(|due| due.get() <= cutoff && due.get() <= now)
                    .map(|due| (due, callback))
            })
            .min_by_key(|(due, _)| *due)
            .map(|(_, callback)| Arc::clone(callback))
    }
    pub(super) fn finish_expiry(
        &mut self,
        runtime: &ChannelRuntimeV1,
        callback: &Callback,
        now: u64,
    ) -> Result<(), ()> {
        if !callback.current_for(runtime) {
            return Err(());
        }
        let timers = self.timers.as_mut().ok_or(())?;
        let fired = timers.drain_due_matching(
            &Now(now),
            runtime.owner_fence().map_err(|_| ())?,
            |_| true,
            |_, key| key == callback.identity(),
        );
        if fired.len() != 1 {
            return Err(());
        }
        self.work
            .retain(|entry| entry.identity() != callback.identity());
        Ok(())
    }
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// Same native liveness pass, before taking Runtime/Map guards. Original
    /// deadlines arbitrate against the canonical death head and older callbacks.
    pub(in crate::gameplay_transport) async fn drain_w01_death_item_expiry(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        now: u64,
    ) -> bool {
        for _ in 0..8 {
            let (callbacks, callback) = {
                let runtime = self.runtime.lock().await;
                let Ok(states) = self.states.spell_states.try_lock() else {
                    return false;
                };
                let cutoff = match runtime.next_source_creature_death() {
                    Ok(Some(notice)) => match notice.enqueued_at_us() {
                        Some(at) => at.min(now),
                        None => return false,
                    },
                    Ok(None) => now,
                    Err(_) => return false,
                };
                let callbacks = states.native_callbacks.clone();
                let Some(room) = self.effective_rooms().gate_monster_ai() else {
                    return true;
                };
                let Some(native) = self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                else {
                    return true;
                };
                let callback = match callbacks.due_death_item(&runtime, room, native, cutoff, now) {
                    Ok(Some(callback)) => callback,
                    Ok(None) => return true,
                    Err(_) => return false,
                };
                if runtime.has_pending_source_creature_deaths()
                    || states.native_profile_pass.has_pending()
                {
                    match crate::gameplay_transport::source_death::next_source_work(
                        &runtime, &states, native,
                    ) {
                        Ok(Some((_, due))) if due < callback.due_us() && due <= now => return true,
                        Ok(_) => {}
                        Err(()) => return false,
                    }
                }
                (callbacks, callback)
            };
            if let Err(error) = self
                .root
                .commit_w01_death_ground(permit, self.holder, &callback, now)
                .await
            {
                #[cfg(test)]
                eprintln!("SRX3_W01_DEATH_EXPIRY_REFUSED error={error:?}");
                #[cfg(not(test))]
                let _ = error;
                return false;
            }
            self.states.loop_ground.invalidate();
            let runtime = self.runtime.lock().await;
            if callbacks
                .complete_death_item(&runtime, &callback, now)
                .is_err()
            {
                return false;
            }
        }
        false
    }
}
