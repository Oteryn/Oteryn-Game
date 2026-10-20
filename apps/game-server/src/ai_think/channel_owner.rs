//! CREATURE-AI-1 §2.1: the Channel owner's side of the creature think. The think
//! (`CreatureAiState`) decides; these admit, wake, budget and commit its one step through
//! Movement on the Channel runtime, and return each report's Ability proposals to the caller,
//! which issues them through Ability. A refused step or proposal changes only the next think.
//!
//! The table is held beside `ChannelRuntimeV1` by its owner rather than inside `foundation`:
//! standalone test crates path-include `foundation/mod.rs` without `ai_think`/`content`.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use super::behaviour_profile::{CreatureBehaviourProfile, ProfileRefusal};
use super::targeting::{self, TargetCandidate};
use super::{
    CREATUREAI0_THINK_WINDOW_MICROS, CREATUREAI0_THINKS_PER_WINDOW, CREATUREAI1_WAKE_CANDIDATES,
    CreatureAiState, CreatureAiTable, CreatureThinkFacts, ThinkReport,
};
use crate::ability::RevisionSet;
use crate::content::{
    ProjectV2AbilityAuthoring, ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef,
};
use crate::foundation::{CarrierError, ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::movement::interest::tibia_floor;
use oteryn_simulation_determinism::GameplayDecisionRoot;

impl CreatureAiTable {
    /// Admits live creature `actor` of `runtime` to thinking with its projected behaviour
    /// profile; its first think is due at `now_us`. A missing or invalid profile is refused and
    /// admits nothing.
    pub(crate) fn admit_creature(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        behavior: Option<&ProjectV2BehaviorAuthoring>,
        abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        let profile = CreatureBehaviourProfile::project(behavior, abilities).map_err(
            |refusal| match refusal {
                ProfileRefusal::Missing => CarrierError::ProfileMissing,
                ProfileRefusal::Invalid => CarrierError::ProfileInvalid,
            },
        )?;
        if !runtime.contains_live_creature(actor) {
            return Err(CarrierError::NotCreature);
        }
        let anchor = runtime.read_actor_position(actor)?.position();
        self.admit(CreatureAiState::new(actor, profile, anchor), now_us);
        Ok(())
    }

    /// Forgets every creature that is no longer live (SPAWN-CEILINGS-1). A thinking creature
    /// leaves at its next think, but one that died or despawned while idle has no due think, so
    /// without this its entry would outlive its slot. The table then holds at most one entry per
    /// live creature, so at most `CREATUREAI0-RL-04`.
    pub(crate) fn forget_dead(&mut self, runtime: &ChannelRuntimeV1) {
        self.entries
            .retain(|_, entry| runtime.contains_live_creature(entry.state.actor()));
    }

    /// The earliest pending think, `None` while every creature is idle (SPAWN-CEILINGS-1, Codex
    /// 4243189401). A think left due past the window's `CREATUREAI0-RL-05` budget keeps its due
    /// time, so after a pass this is at most that pass's time while a backlog remains. One pass
    /// over the table, like `forget_dead`.
    pub(crate) fn earliest_due(&self) -> Option<u64> {
        self.entries.values().filter_map(|entry| entry.due_us).min()
    }

    /// §2.1 item 1: `player` moved or was admitted; wakes the idle creatures it sees.
    pub(crate) fn wake_for_player(
        &mut self,
        runtime: &ChannelRuntimeV1,
        player: ExactActorRef,
        now_us: u64,
    ) -> Result<Vec<ExactActorRef>, CarrierError> {
        let at = runtime.read_actor_position(player)?.position();
        Ok(self
            .wake_indexed(&[at], |creature| live_position(runtime, creature), now_us)
            .0)
    }

    /// §2.1 item 1 for every player of one pass, in `players` order: the result equals one
    /// `wake` per player in turn (a creature woken for an earlier player is no longer idle), and
    /// a player whose position cannot be read wakes nothing (SPAWN-CEILINGS-1, Codex 4242808144).
    pub(crate) fn wake_for_players(
        &mut self,
        runtime: &ChannelRuntimeV1,
        players: &[ExactActorRef],
        now_us: u64,
    ) -> Vec<ExactActorRef> {
        let viewers = players
            .iter()
            .filter_map(|player| runtime.read_actor_position(*player).ok())
            .map(|snapshot| snapshot.position())
            .collect::<Vec<_>>();
        self.wake_indexed(
            &viewers,
            |creature| live_position(runtime, creature),
            now_us,
        )
        .0
    }

    /// The wake of `viewers` in turn, bounded independently of the product of players and
    /// creatures: one walk of the table with one `position_of` per idle creature builds an index
    /// by (floor, x, y); each viewer then examines only the index rows of its view box, at most
    /// the 16 VIS-1 floors of `2 * WAKE_VIEW_REACH + 1` rows. Returns the woken and the number
    /// of index entries the viewers examined.
    pub(crate) fn wake_indexed(
        &mut self,
        viewers: &[MovementLocalPosition],
        position_of: impl Fn(ExactActorRef) -> Option<MovementLocalPosition>,
        now_us: u64,
    ) -> (Vec<ExactActorRef>, usize) {
        if viewers.is_empty() {
            return (Vec::new(), 0);
        }
        let idle = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.due_us.is_none())
            .filter_map(|(identity, entry)| {
                let position = position_of(entry.state.actor())?;
                Some((position.floor, position.x, position.y, *identity))
            })
            .collect::<BTreeSet<_>>();
        let floors = idle.iter().map(|key| key.0).collect::<BTreeSet<_>>();
        let mut woken = Vec::new();
        let mut taken = BTreeSet::new();
        let mut examined = 0_usize;
        for viewer in viewers {
            let mut seen = Vec::new();
            for &floor in &floors {
                let Some((columns, rows)) = wake_view_box(*viewer, floor) else {
                    continue;
                };
                for x in columns {
                    let low = (floor, x, *rows.start(), [u8::MIN; 16]);
                    let high = (floor, x, *rows.end(), [u8::MAX; 16]);
                    for &(_, x, y, identity) in idle.range(low..=high) {
                        examined += 1;
                        let position = MovementLocalPosition { x, y, floor };
                        if !taken.contains(&identity) && targeting::sees(*viewer, position) {
                            seen.push((targeting::plane_distance(*viewer, position), identity));
                        }
                    }
                }
            }
            seen.sort_unstable();
            seen.truncate(CREATUREAI1_WAKE_CANDIDATES);
            for (_, identity) in seen {
                if let Some(entry) = self.entries.get_mut(&identity) {
                    entry.due_us = Some(now_us);
                    taken.insert(identity);
                    woken.push(entry.state.actor());
                }
            }
        }
        (woken, examined)
    }

    /// The thinks to start at `now_us` under `CREATUREAI0-RL-05`, in (due, placement) order,
    /// each counted against the window, and the earliest due think left in the table
    /// (SPAWN-CEILINGS-1, Codex 4243397472). The same selection as `take_due`, but one read-only
    /// walk of the table keeps only the window's remaining budget in a bounded heap: it neither
    /// collects nor sorts every due think, so a backlog of n due thinks costs a window
    /// `O(n log 1,024)` reads and `O(1,024)` memory instead of an `O(n log n)` sort of n.
    pub(crate) fn take_due_bounded(&mut self, now_us: u64) -> (Vec<ExactActorRef>, Option<u64>) {
        let window = now_us / CREATUREAI0_THINK_WINDOW_MICROS;
        if self.window != Some(window) {
            self.window = Some(window);
            self.window_started = 0;
        }
        let budget = CREATUREAI0_THINKS_PER_WINDOW.saturating_sub(self.window_started);
        let mut taken = BinaryHeap::with_capacity(budget.min(self.entries.len()));
        let mut rest: Option<u64> = None;
        let mut leave = |due: u64| rest = Some(rest.map_or(due, |rest| rest.min(due)));
        for (identity, entry) in &self.entries {
            let Some(due) = entry.due_us else {
                continue;
            };
            if due > now_us {
                leave(due);
                continue;
            }
            let key = (due, *identity);
            if taken.len() < budget {
                taken.push(key);
            } else if taken.peek().is_some_and(|latest| key < *latest) {
                if let Some((left, _)) = taken.pop() {
                    leave(left);
                }
                taken.push(key);
            } else {
                leave(due);
            }
        }
        let taken = taken.into_sorted_vec();
        self.window_started += taken.len();
        let actors = taken
            .iter()
            .filter_map(|(_, identity)| self.entries.get(identity))
            .map(|entry| entry.state.actor())
            .collect();
        (actors, rest)
    }

    /// Runs the thinks due at `now_us` within `CREATUREAI0-RL-05`. `players` are the Channel's
    /// player candidates with the owner's eligibility facts; `walkable` is the map's admission of
    /// a cell. A dead or despawned creature leaves the table. Each step is committed here.
    pub(crate) fn run_due_thinks(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        root: &GameplayDecisionRoot,
        revisions: &RevisionSet,
        players: &[TargetCandidate],
        walkable: impl Fn(MovementLocalPosition) -> bool,
    ) -> Vec<(ExactActorRef, ThinkReport)> {
        self.run_due_thinks_until_next(runtime, now_us, root, revisions, players, walkable)
            .0
    }

    /// [`Self::run_due_thinks`], also returning the earliest pending think after the pass
    /// (`earliest_due` without its walk of the table, SPAWN-CEILINGS-1, Codex 4243397472): the
    /// earliest think the selection left, or a taken creature's next think.
    pub(crate) fn run_due_thinks_until_next(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        root: &GameplayDecisionRoot,
        revisions: &RevisionSet,
        players: &[TargetCandidate],
        walkable: impl Fn(MovementLocalPosition) -> bool,
    ) -> (Vec<(ExactActorRef, ThinkReport)>, Option<u64>) {
        let (taken, mut next) = self.take_due_bounded(now_us);
        let reports =
            self.run_taken_thinks(runtime, &taken, now_us, root, revisions, players, walkable);
        for actor in taken {
            if let Some(due) = self.due(actor) {
                next = Some(next.map_or(due, |next| next.min(due)));
            }
        }
        (reports, next)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_taken_thinks(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        taken: &[ExactActorRef],
        now_us: u64,
        root: &GameplayDecisionRoot,
        revisions: &RevisionSet,
        players: &[TargetCandidate],
        walkable: impl Fn(MovementLocalPosition) -> bool,
    ) -> Vec<(ExactActorRef, ThinkReport)> {
        let mut reports = Vec::new();
        for &actor in taken {
            let Some(health) = runtime.live_creature_health(actor) else {
                self.remove(actor);
                continue;
            };
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                self.remove(actor);
                continue;
            };
            let has_active_condition = runtime
                .actor_conditions(actor, None)
                .is_ok_and(|store| !store.instances().is_empty());
            let facts = CreatureThinkFacts {
                position: snapshot.position(),
                health,
                has_active_condition,
                players,
                now_us,
                root,
                revisions,
            };
            let report = self.think(actor, &facts, |destination| {
                walkable(destination)
                    && runtime.position_occupied_by_other(actor, destination) == Ok(false)
            });
            let Some(Ok(report)) = report else {
                continue;
            };
            if let Some(step) = report.step {
                // A refused step changes only the next think.
                let _ = runtime
                    .borrow_movement_position()
                    .commit_cardinal(snapshot, step.destination);
            }
            reports.push((actor, report));
        }
        reports
    }
}

/// The reference view's plane half-extent (`VisibilitySettings::REFERENCE`, 18 x 14): a cell a
/// player sees on floor `f` lies within it of the player's cell shifted by `player.floor - f`.
const WAKE_VIEW_REACH: i64 = 9;

/// The cells of `floor` a player at `viewer` may see, as (columns, rows); `None` when it sees no
/// cell of `floor`. A superset of `targeting::sees`, which still decides each cell: a native floor
/// sees as the Tibia floor of its negation, and a player off the VIS-1 floors sees nothing.
fn wake_view_box(
    viewer: MovementLocalPosition,
    floor: i16,
) -> Option<(std::ops::RangeInclusive<i32>, std::ops::RangeInclusive<i32>)> {
    let shift = i64::from(tibia_floor(viewer.floor)?) - i64::from(tibia_floor(floor)?);
    let span = |centre: i32| {
        let centre = i64::from(centre) + shift;
        to_i32(centre - WAKE_VIEW_REACH)..=to_i32(centre + WAKE_VIEW_REACH)
    };
    Some((span(viewer.x), span(viewer.y)))
}

fn to_i32(value: i64) -> i32 {
    i32::try_from(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or_default()
}

/// The owner's read of a creature's position: `None` unless it is a live creature.
fn live_position(
    runtime: &ChannelRuntimeV1,
    creature: ExactActorRef,
) -> Option<MovementLocalPosition> {
    if !runtime.contains_live_creature(creature) {
        return None;
    }
    runtime
        .read_actor_position(creature)
        .ok()
        .map(|snapshot| snapshot.position())
}

#[cfg(test)]
#[path = "channel_owner_creature_ai_tests.rs"]
mod channel_owner_creature_ai_tests;
