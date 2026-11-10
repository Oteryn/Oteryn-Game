//! One original native Think batch and its ordered, cancellation-safe consumers.
use super::*;
use crate::ai_think::profile_schedule::ProfileAbilityProposal;
use crate::ai_think::summon_plan::ReservedNativeThinkBatch;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::soul_war_callbacks::RetainedSoulWarAttack;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(super) struct RetainedProposal {
    pub(super) proposal: ProfileAbilityProposal,
    pub(super) soul_war: Option<Arc<RetainedSoulWarAttack>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Consumer {
    SoulWar,
    Native,
    Physical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Cursor {
    Prelude(Consumer),
    Proposal(usize, Consumer),
    Drain,
    Items,
}
impl Default for Cursor {
    fn default() -> Self {
        Self::Prelude(Consumer::SoulWar)
    }
}
impl Cursor {
    fn successor(self, count: usize) -> Result<Self, ()> {
        use Consumer::{Native, Physical, SoulWar};
        match self {
            Self::Prelude(SoulWar) => Ok(Self::Prelude(Native)),
            Self::Prelude(Native) => Ok(Self::Prelude(Physical)),
            Self::Prelude(Physical) if count == 0 => Ok(Self::Drain),
            Self::Prelude(Physical) => Ok(Self::Proposal(0, SoulWar)),
            Self::Proposal(index, SoulWar) if index < count => Ok(Self::Proposal(index, Native)),
            Self::Proposal(index, Native) if index < count => Ok(Self::Proposal(index, Physical)),
            Self::Proposal(index, Physical) if index.checked_add(1) == Some(count) => {
                Ok(Self::Drain)
            }
            Self::Proposal(index, Physical) if index < count => {
                Ok(Self::Proposal(index + 1, SoulWar))
            }
            Self::Drain => Ok(Self::Items),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default)]
struct Progress {
    source_known: bool,
    source_due: Option<u64>,
    ai_known: bool,
    movement: Option<Option<u64>>,
    heals_known: bool,
    fields_known: bool,
    proposals: Vec<RetainedProposal>,
    actors: Vec<ExactActorRef>,
    rejected: bool,
    cursor: Cursor,
}

/// The opaque reservation and source DATA are retained before any consumer await.
/// A later actor/type observation can neither mint nor replace a captured proposal.
pub(in crate::gameplay_transport) struct PendingProfilePass {
    batch: Arc<ReservedNativeThinkBatch>,
    actor_bound: usize,
    proposal_bound: usize,
    progress: Mutex<Progress>,
}
impl std::fmt::Debug for PendingProfilePass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingProfilePass")
            .field("observed_at_us", &self.batch.observed_at_us())
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}
impl PendingProfilePass {
    fn prepare(batch: Arc<ReservedNativeThinkBatch>) -> Result<Arc<Self>, ()> {
        let actor_bound = batch.occurrences().len();
        let proposal_bound = actor_bound
            .checked_mul(
                crate::ai_think::profile_schedule::MAX_ATTACK_ENTRIES
                    + crate::ai_think::profile_schedule::MAX_DEFENCE_ENTRIES,
            )
            .ok_or(())?;
        let mut progress = Progress::default();
        progress
            .proposals
            .try_reserve_exact(proposal_bound)
            .map_err(|_| ())?;
        progress
            .actors
            .try_reserve_exact(actor_bound)
            .map_err(|_| ())?;
        Ok(Arc::new(Self {
            batch,
            actor_bound,
            proposal_bound,
            progress: Mutex::new(progress),
        }))
    }
    fn progress(&self) -> std::sync::MutexGuard<'_, Progress> {
        self.progress
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn batch(&self) -> &ReservedNativeThinkBatch {
        &self.batch
    }
    pub(super) fn shared_batch(&self) -> Arc<ReservedNativeThinkBatch> {
        Arc::clone(&self.batch)
    }
    pub(super) fn source_known(&self) -> bool {
        self.progress().source_known
    }
    pub(super) fn finish_source(&self, primal_due: Option<u64>, rotten_due: Option<u64>) {
        let mut progress = self.progress();
        progress.source_due = primal_due.into_iter().chain(rotten_due).min();
        progress.source_known = true;
    }
    pub(super) fn source_due(&self) -> Option<u64> {
        self.progress().source_due
    }
    pub(super) fn ai_known(&self) -> bool {
        self.progress().ai_known
    }
    /// Called synchronously beside the AI owner's sequence/deadline commit.
    pub(super) fn record_report(
        &self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        actor: ExactActorRef,
        proposals: impl IntoIterator<Item = ProfileAbilityProposal>,
    ) {
        let mut progress = self.progress();
        if progress.rejected || progress.actors.contains(&actor) {
            return;
        }
        if progress.ai_known
            || progress.actors.len() == self.actor_bound
            || !self
                .batch
                .occurrences()
                .iter()
                .any(|occurrence| occurrence.actor == actor)
        {
            progress.rejected = true;
            return;
        }
        for proposal in proposals {
            if proposal.issuer != actor || progress.proposals.len() == self.proposal_bound {
                progress.rejected = true;
                return;
            }
            let soul_war = match super::super::soul_war_callbacks::capture_attack_proposal(
                runtime,
                room,
                native,
                &proposal,
                self.batch.observed_at_us(),
            ) {
                Ok(source) => source.map(Arc::new),
                Err(()) => {
                    progress.rejected = true;
                    return;
                }
            };
            progress
                .proposals
                .push(RetainedProposal { proposal, soul_war });
        }
        progress.actors.push(actor);
    }
    pub(super) fn finish_ai(&self) -> Result<(), ()> {
        let mut progress = self.progress();
        if progress.rejected || !progress.source_known {
            return Err(());
        }
        progress.ai_known = true;
        Ok(())
    }
    pub(super) fn results(&self) -> Result<(Vec<ProfileAbilityProposal>, Vec<ExactActorRef>), ()> {
        let progress = self.progress();
        if !progress.ai_known || progress.rejected {
            return Err(());
        }
        let mut proposals = Vec::new();
        proposals
            .try_reserve_exact(progress.proposals.len())
            .map_err(|_| ())?;
        proposals.extend(
            progress
                .proposals
                .iter()
                .map(|retained| retained.proposal.clone()),
        );
        let mut actors = Vec::new();
        actors
            .try_reserve_exact(progress.actors.len())
            .map_err(|_| ())?;
        actors.extend_from_slice(&progress.actors);
        Ok((proposals, actors))
    }
    pub(super) fn movement(&self) -> Option<Option<u64>> {
        self.progress().movement
    }
    pub(super) fn finish_movement(&self, due: Option<u64>) {
        self.progress().movement = Some(due);
    }
    pub(super) fn heals_known(&self) -> bool {
        self.progress().heals_known
    }
    pub(super) fn finish_heals(&self) {
        self.progress().heals_known = true;
    }
    pub(super) fn fields_known(&self) -> bool {
        self.progress().fields_known
    }
    pub(super) fn finish_fields(&self) {
        self.progress().fields_known = true;
    }
    fn consumers_ready(&self) -> Result<bool, ()> {
        let progress = self.progress();
        if progress.rejected {
            return Err(());
        }
        Ok(progress.ai_known && progress.fields_known)
    }
    pub(super) fn current(&self) -> Result<(Cursor, Option<RetainedProposal>), ()> {
        let progress = self.progress();
        if !progress.ai_known || progress.rejected || !progress.fields_known {
            return Err(());
        }
        let proposal = match progress.cursor {
            Cursor::Proposal(index, _) => Some(progress.proposals.get(index).ok_or(())?.clone()),
            _ => None,
        };
        Ok((progress.cursor, proposal))
    }
    pub(super) fn advance(&self, expected: Cursor) -> Result<(), ()> {
        let mut progress = self.progress();
        if progress.rejected || progress.cursor != expected {
            return Err(());
        }
        progress.cursor = expected.successor(progress.proposals.len())?;
        Ok(())
    }
    pub(super) fn items_ready(&self) -> bool {
        let progress = self.progress();
        !progress.rejected && progress.cursor == Cursor::Items
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(in crate::gameplay_transport) struct ProfilePassSnapshot {
    pub(in crate::gameplay_transport) observed_at_us: u64,
    pub(in crate::gameplay_transport) completed_physical: usize,
    pub(in crate::gameplay_transport) proposals: Vec<(ProfileAbilityProposal, bool)>,
}

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct ProfilePassOwner {
    pending: Option<Arc<PendingProfilePass>>,
    #[cfg(test)]
    pause_after_physical: Option<(usize, Arc<tokio::sync::Notify>)>,
}
impl ProfilePassOwner {
    pub(in crate::gameplay_transport) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
    /// Original due for the global arbiter, never a retry clock or sentinel.
    pub(in crate::gameplay_transport) fn next_due(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Option<u64>, ()> {
        self.pending
            .as_ref()
            .map(|pending| {
                pending
                    .batch
                    .current_for(runtime)
                    .then_some(pending.batch.observed_at_us())
                    .ok_or(())
            })
            .transpose()
    }
    pub(in crate::gameplay_transport) fn pending(&self) -> Option<Arc<PendingProfilePass>> {
        self.pending.as_ref().map(Arc::clone)
    }
    pub(super) fn prepare(
        &mut self,
        batch: Arc<ReservedNativeThinkBatch>,
    ) -> Result<Arc<PendingProfilePass>, ()> {
        if self.pending.is_some() {
            return Err(());
        }
        let pending = PendingProfilePass::prepare(batch)?;
        self.pending = Some(Arc::clone(&pending));
        Ok(pending)
    }
    pub(super) fn complete(&mut self, expected: &Arc<PendingProfilePass>) -> Result<(), ()> {
        if !expected.items_ready()
            || !self
                .pending
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, expected))
        {
            return Err(());
        }
        self.pending = None;
        Ok(())
    }
    /// Read the actual retained order/cursor; no proposal or source proof is created.
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn snapshot_for_test(&self) -> Option<ProfilePassSnapshot> {
        let pending = self.pending.as_ref()?;
        let progress = pending.progress();
        let completed_physical = match progress.cursor {
            Cursor::Prelude(_) => 0,
            Cursor::Proposal(index, _) => index,
            Cursor::Drain | Cursor::Items => progress.proposals.len(),
        };
        Some(ProfilePassSnapshot {
            observed_at_us: pending.batch.observed_at_us(),
            completed_physical,
            proposals: progress
                .proposals
                .iter()
                .map(|retained| (retained.proposal.clone(), retained.soul_war.is_some()))
                .collect(),
        })
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) fn pause_after_physical_for_test(
        &mut self,
        count: usize,
    ) -> Result<Arc<tokio::sync::Notify>, ()> {
        if count == 0 {
            return Err(());
        }
        let ready = Arc::new(tokio::sync::Notify::new());
        self.pause_after_physical = Some((count, Arc::clone(&ready)));
        Ok(ready)
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn clear_physical_pause_for_test(&mut self) {
        self.pause_after_physical = None;
    }
    #[cfg(test)]
    pub(super) fn take_physical_pause(&mut self) -> bool {
        let Some((count, ready)) = self.pause_after_physical.as_mut() else {
            return false;
        };
        *count -= 1;
        if *count != 0 {
            return false;
        }
        ready.notify_one();
        self.pause_after_physical = None;
        true
    }
}

/// One retained consumer selected by the global original-due arbiter. A source
/// or AI prefix still awaiting preparation, and the detached Item tail, remain
/// owned by drive_monster_profiles; this port never reserves a fresh Think.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn resume_one_due(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    parties: &physical_spell::ReferenceParties,
    through: u64,
    execution_now: u64,
    native_health_names: &super::super::source_names::NativeHealthNamesData,
) -> Result<bool, ()> {
    let Some(due) = states.native_profile_pass.next_due(runtime)? else {
        return Ok(false);
    };
    if due > through || due > execution_now {
        return Ok(false);
    }
    let pending = states.native_profile_pass.pending().ok_or(())?;
    if !pending.consumers_ready()? {
        return Ok(false);
    }
    let (cursor, proposal) = pending.current()?;
    let current = proposal
        .as_ref()
        .map_or(&[][..], |retained| std::slice::from_ref(&retained.proposal));
    match cursor {
        Cursor::Prelude(Consumer::SoulWar) | Cursor::Proposal(_, Consumer::SoulWar) => {
            if let Some(captured) = proposal
                .as_ref()
                .and_then(|retained| retained.soul_war.as_ref())
            {
                let _ = super::super::soul_war_callbacks::drive_retained_attack_proposal(
                    runtime,
                    states,
                    room,
                    native,
                    map,
                    Arc::clone(captured),
                    execution_now,
                )
                .await?;
            } else {
                // A later retry clock must not run a later scheduled attack
                // ahead of the original proposal prefix or another source owner.
                while states
                    .soul_war_callbacks
                    .next_attack_due()
                    .is_some_and(|next| next <= due)
                {
                    let _ = super::super::soul_war_callbacks::drive_attack_due_through(
                        runtime,
                        states,
                        room,
                        native,
                        map,
                        due,
                        execution_now,
                    )
                    .await?;
                }
            }
        }
        Cursor::Prelude(Consumer::Native) | Cursor::Proposal(_, Consumer::Native) => {
            if let Some(active) = owner.active_generation {
                let callbacks = states.native_callbacks.clone();
                callbacks
                    .execute(
                        active,
                        runtime,
                        states,
                        attack,
                        room,
                        native,
                        map,
                        current,
                        parties,
                        due,
                        native_health_names,
                    )
                    .await?;
            }
        }
        Cursor::Prelude(Consumer::Physical) | Cursor::Proposal(_, Consumer::Physical) => {
            physical_spell::execute_proposals(
                runtime,
                states,
                attack,
                room,
                native,
                map,
                current,
                parties,
                execution_now,
                owner.imported_charms,
                native_health_names,
            )
            .await?;
        }
        Cursor::Drain => {
            physical_spell::drain_due_through(
                runtime,
                states,
                attack,
                room,
                native,
                map,
                parties,
                due,
                execution_now,
                owner.imported_charms,
            )
            .await?;
        }
        Cursor::Items => return Ok(false),
    }
    // Do not acknowledge an unavailable physical read as a completed proposal.
    // This is synchronous: no new cancellation point follows a successful HP write.
    if !map.view().matches(runtime) {
        return Err(());
    }
    // There is no await between a completed physical consumer and this cursor.
    pending.advance(cursor)?;
    #[cfg(test)]
    if matches!(cursor, Cursor::Proposal(_, Consumer::Physical))
        && states.native_profile_pass.take_physical_pause()
    {
        std::future::pending::<()>().await;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::channel_content_pin::ChannelContentPin;
    use crate::foundation::{ChannelId, NodeId, WorldId};
    use std::future::Future;

    #[test]
    fn cancellation_retains_the_original_batch_and_never_grants_a_retired_owner()
    -> Result<(), String> {
        let id = |seed| {
            let mut value = [seed; 16];
            value[6] = 0x70;
            value[8] = 0x80;
            value
        };
        let world = WorldId::decode(&id(1)).map_err(|e| format!("world: {e:?}"))?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (0, 0, 0),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(|e| format!("channel: {e:?}"))?,
            NodeId::decode(&id(3)).map_err(|e| format!("node: {e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(|e| format!("runtime: {e:?}"))?;
        let mut table = crate::ai_think::CreatureAiTable::default();
        let batch = Arc::new(
            table
                .reserve_current_thinks(&mut runtime, 5)
                .ok_or("real owner batch")?,
        );
        let mut owner = ProfilePassOwner::default();
        let pending = owner
            .prepare(Arc::clone(&batch))
            .map_err(|_| "retain real batch")?;
        assert!(Arc::ptr_eq(&batch, &pending.shared_batch()));
        assert_eq!(owner.next_due(&runtime), Ok(Some(5)));
        assert!(
            owner.complete(&pending).is_err(),
            "unfinished consumers cannot be forgotten"
        );
        let lease = Arc::clone(&pending);
        let mut cancelled = Box::pin(async move {
            let _lease = lease;
            std::future::pending::<()>().await;
        });
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(cancelled.as_mut().poll(&mut cx).is_pending());
        drop(cancelled);
        assert!(owner.has_pending());
        assert!(Arc::ptr_eq(
            &pending,
            &owner.pending().ok_or("retained owner after cancellation")?
        ));
        assert_eq!(
            owner.next_due(&runtime),
            Ok(Some(5)),
            "no fresh retry timestamp"
        );
        runtime.retire_owner_cycle();
        assert!(owner.next_due(&runtime).is_err());
        assert!(
            owner.has_pending(),
            "stale authority cannot silently clear the queued work"
        );
        Ok(())
    }
}
