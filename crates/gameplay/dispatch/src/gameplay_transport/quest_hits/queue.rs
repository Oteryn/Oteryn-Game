//! Actor-census-scoped hit jobs. One slot per authored objective; no allocation after damage.
use crate::durability::quest_state::{
    native_hit_capture::PreparedQuestHit, native_hits::NativeQuestHit,
};
use crate::foundation::{
    ChannelRuntimeV1, CharacterLease, CommandRef, ExactActorRef, GameSessionId,
};
use oteryn_gameplay_entities::runtime_actor_carrier::CommittedNativeHit;
use oteryn_gameplay_quests::quest::{
    QUESTSTATE0_RL_08_TRACKS, QuestStateCatalogue,
    authored_bindings::{MAX_BOUND_STAGES, hits},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug)]
struct Row {
    transition: String,
    active: bool,
    finished: bool,
    plan: Option<PreparedQuestHit>,
    catalogue: Arc<QuestStateCatalogue>,
}
#[derive(Debug)]
pub(crate) struct Snapshot {
    actor: ExactActorRef,
    session: GameSessionId,
    lease: CharacterLease,
    revision: u64,
    tracks: BTreeMap<String, i64>,
    catalogue: Arc<QuestStateCatalogue>,
    rows: Vec<Row>,
}
impl Snapshot {
    pub(crate) fn new(
        catalogue: Arc<QuestStateCatalogue>,
        actor: ExactActorRef,
        session: GameSessionId,
        lease: CharacterLease,
        revision: u64,
        tracks: &BTreeMap<String, i64>,
    ) -> Option<Self> {
        if revision == 0 || tracks.len() > QUESTSTATE0_RL_08_TRACKS {
            return None;
        }
        let rows: Vec<_> = hits::objectives(&catalogue)
            .filter(|(_, objective)| objective.active(&catalogue, tracks))
            .map(|(_, objective)| Row {
                transition: objective.key.clone(),
                active: true,
                finished: false,
                plan: None,
                catalogue: Arc::clone(&catalogue),
            })
            .collect();
        if rows.is_empty() || rows.len() > MAX_BOUND_STAGES {
            return None;
        }
        Some(Self {
            actor,
            session,
            lease,
            revision,
            tracks: tracks.clone(),
            catalogue,
            rows,
        })
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Pending {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) lease: CharacterLease,
    pub(crate) transition: String,
    pub(crate) hit: NativeQuestHit,
    pub(crate) catalogue: Arc<QuestStateCatalogue>,
}
impl Pending {
    pub(crate) fn current_for(&self, current: &QuestStateCatalogue) -> bool {
        hits::objective_for(&self.catalogue, &self.transition).is_some_and(|(quest, _)| {
            hits::objective_for(current, &self.transition).is_some_and(|(owner, _)| owner == quest)
                && self
                    .catalogue
                    .definition_hash(quest)
                    .is_some_and(|hash| current.definition_hash(quest) == Some(hash))
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct HitQueue {
    snapshots: Vec<Snapshot>,
}
impl HitQueue {
    /// The current native player census bounds snapshots; the existing authored stage
    /// bound includes both active slots and unacknowledged jobs retained by a refresh.
    pub(crate) fn install(
        &mut self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
        next: Option<Snapshot>,
    ) -> bool {
        self.snapshots.retain(|snapshot| {
            runtime.bound_attacker_lease(snapshot.actor, snapshot.session) == Ok(snapshot.lease)
        });
        let existing = self
            .snapshots
            .iter()
            .position(|snapshot| snapshot.session == session);
        let Some(mut next) = next else {
            if let Some(index) = existing {
                let old = &mut self.snapshots[index];
                old.rows.retain(|row| {
                    row.plan
                        .as_ref()
                        .and_then(PreparedQuestHit::pending)
                        .is_some()
                });
                for row in &mut old.rows {
                    row.active = false;
                }
                if old.rows.is_empty() {
                    self.snapshots.remove(index);
                }
            }
            return true;
        };
        if next.session != session
            || runtime.bound_attacker_lease(next.actor, session) != Ok(next.lease)
        {
            return false;
        }
        if let Some(index) = existing {
            let extra = self.snapshots[index]
                .rows
                .iter()
                .filter(|old| {
                    old.plan
                        .as_ref()
                        .and_then(PreparedQuestHit::pending)
                        .is_some()
                        && !next.rows.iter().any(|row| row.transition == old.transition)
                })
                .count();
            if next.rows.len().saturating_add(extra) > MAX_BOUND_STAGES
                || next.rows.try_reserve(extra).is_err()
            {
                return false;
            }
            for mut old in std::mem::take(&mut self.snapshots[index].rows) {
                if old
                    .plan
                    .as_ref()
                    .and_then(PreparedQuestHit::pending)
                    .is_none()
                {
                    continue;
                }
                if let Some(row) = next
                    .rows
                    .iter_mut()
                    .find(|row| row.transition == old.transition)
                {
                    row.plan = old.plan.take();
                    row.catalogue = old.catalogue;
                } else {
                    old.active = false;
                    next.rows.push(old);
                }
            }
            self.snapshots[index] = next;
        } else {
            if self.snapshots.try_reserve(1).is_err() {
                return false;
            }
            self.snapshots.push(next);
        }
        true
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "exact native command and actual committed damage source"
    )]
    pub(crate) fn prepare<'a>(
        &'a mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        target: ExactActorRef,
        command: CommandRef,
        ordinal: u16,
        creature: &str,
        item: &str,
        spell: Option<&str>,
    ) -> Result<PreparedHits<'a>, ()> {
        let session = command.game_session_id();
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| ())?;
        let index = self.snapshots.iter().position(|snapshot| {
            snapshot.actor == actor && snapshot.session == session && snapshot.lease == lease
        });
        let mut rows = Vec::new();
        if let Some(index) = index {
            let snapshot = &mut self.snapshots[index];
            rows.try_reserve(snapshot.rows.len()).map_err(|_| ())?;
            for (offset, row) in snapshot.rows.iter_mut().enumerate() {
                if !row.active || row.finished || row.plan.is_some() {
                    continue;
                }
                let Some((_, objective)) =
                    hits::objective_for(&snapshot.catalogue, &row.transition)
                else {
                    continue;
                };
                if !objective.matches(creature, item, spell) {
                    continue;
                }
                if let Some(plan) = PreparedQuestHit::prepare(
                    &snapshot.catalogue,
                    &row.transition,
                    &snapshot.tracks,
                    snapshot.revision,
                    actor,
                    target,
                    lease,
                    command,
                    ordinal,
                ) {
                    row.plan = Some(plan);
                    row.catalogue = Arc::clone(&snapshot.catalogue);
                    rows.push(offset);
                }
            }
        }
        Ok(PreparedHits {
            queue: self,
            index,
            rows,
        })
    }
    pub(crate) fn pending(&self) -> Vec<Pending> {
        self.snapshots
            .iter()
            .flat_map(|snapshot| {
                snapshot.rows.iter().filter_map(move |row| {
                    let hit = row.plan.as_ref()?.pending()?;
                    Some(Pending {
                        actor: snapshot.actor,
                        session: snapshot.session,
                        lease: snapshot.lease,
                        transition: row.transition.clone(),
                        hit: hit.clone(),
                        catalogue: Arc::clone(&row.catalogue),
                    })
                })
            })
            .take(MAX_BOUND_STAGES)
            .collect()
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.snapshots.iter().any(|snapshot| {
            snapshot.rows.iter().any(|row| {
                row.plan
                    .as_ref()
                    .and_then(PreparedQuestHit::pending)
                    .is_some()
            })
        })
    }
    pub(crate) fn acknowledge(&mut self, pending: &Pending) -> bool {
        let Some(snapshot) = self.snapshots.iter_mut().find(|snapshot| {
            snapshot.actor == pending.actor
                && snapshot.session == pending.session
                && snapshot.lease == pending.lease
        }) else {
            return false;
        };
        let Some(row) = snapshot.rows.iter_mut().find(|row| {
            row.transition == pending.transition
                && row
                    .plan
                    .as_ref()
                    .and_then(PreparedQuestHit::pending)
                    .is_some_and(|hit| hit.occurrence() == pending.hit.occurrence())
        }) else {
            return false;
        };
        row.plan = None;
        row.finished = true;
        snapshot.rows.retain(|row| row.active || row.plan.is_some());
        self.snapshots.retain(|snapshot| !snapshot.rows.is_empty());
        true
    }
}
/// Keep this borrow under the capture-cache guard for the entire native commit turn.
/// Cancellation or an early return discards only uncommitted preparations.
pub(crate) struct PreparedHits<'a> {
    queue: &'a mut HitQueue,
    index: Option<usize>,
    rows: Vec<usize>,
}
impl PreparedHits<'_> {
    pub(crate) fn capture(
        &mut self,
        witness: CommittedNativeHit,
        creature: &str,
        item: &str,
        spell: Option<&str>,
    ) -> usize {
        let Some(index) = self.index else {
            return 0;
        };
        let mut captured = 0;
        for offset in &self.rows {
            if self.queue.snapshots[index].rows[*offset]
                .plan
                .as_mut()
                .is_some_and(|plan| plan.capture(witness, creature, item, spell))
            {
                captured += 1;
            }
        }
        captured
    }
}
impl Drop for PreparedHits<'_> {
    fn drop(&mut self) {
        let Some(index) = self.index else {
            return;
        };
        for offset in &self.rows {
            let row = &mut self.queue.snapshots[index].rows[*offset];
            if row
                .plan
                .as_ref()
                .and_then(PreparedQuestHit::pending)
                .is_none()
            {
                row.plan = None;
            }
        }
    }
}
