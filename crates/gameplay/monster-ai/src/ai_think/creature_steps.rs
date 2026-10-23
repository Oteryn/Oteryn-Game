//! Creature movement state and the Channel-owned step timer, independent of the think cadence.
use super::CreatureAiTable;
use super::movement_goal::{MovementGoal, NearCounter, destination};
use crate::ai::PathProposal;
use crate::content::{LogicalCell, NativeEntryMovementCells};
use crate::foundation::owner_timer::{OwnerTimerLane, TimerFamily};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::map::floor::CellBlocker;
use crate::movement::path_budget::ChannelPathBudget;
use crate::movement::speed::{CurrentCreatureSpeedReader, GroundSpeedSource};
use oteryn_simulation_determinism::GameplayDecisionRoot;

mod creature_goal_adoption;
mod creature_paths;
mod creature_step_choice;
mod creature_step_timer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StepFamily;
impl TimerFamily for StepFamily {
    fn registered_maximum(self) -> usize {
        1
    }
}
pub(super) type StepTimers = OwnerTimerLane<StepFamily, ExactActorRef>;

#[derive(Debug)]
pub(super) struct CreatureMotion {
    pub(super) goal: MovementGoal,
    pub(super) near: NearCounter,
    pub(super) seen_think: u64,
    pub(super) step_sequence: u64,
    pub(super) content: [u8; 32],
    pub(super) navigation: [u8; 32],
    pub(super) pending: Option<u64>,
    pub(super) route: Option<PathProposal>,
    pub(super) cursor: usize,
    pub(super) expected: Option<MovementLocalPosition>,
    pub(super) due: Option<u64>,
    pub(super) prepared: Option<(crate::movement::CardinalStep, MovementLocalPosition, bool)>,
}
impl Default for CreatureMotion {
    fn default() -> Self {
        Self {
            goal: MovementGoal::Stand,
            near: NearCounter::default(),
            seen_think: 0,
            step_sequence: 0,
            content: [0; 32],
            navigation: [0; 32],
            pending: None,
            route: None,
            cursor: 0,
            expected: None,
            due: None,
            prepared: None,
        }
    }
}
impl CreatureMotion {
    fn stop(&mut self) {
        self.goal = MovementGoal::Stand;
        self.pending = None;
        self.route = None;
        self.expected = None;
        self.due = None;
        self.prepared = None;
    }
}

impl CreatureAiTable {
    /// Composes goals, bounded paths and step deadlines under the actual Channel owner.
    /// Think reports never commit movement. `navigation` is the active map revision.
    #[allow(clippy::too_many_arguments)]
    pub fn drive_movement(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        blocking: &dyn CellBlocker,
        navigation: [u8; 32],
        source: &dyn CurrentCreatureSpeedReader,
        ground: &dyn GroundSpeedSource,
        root: &GameplayDecisionRoot,
        now: u64,
        budget: &mut ChannelPathBudget,
        player_waiting: bool,
    ) -> Option<u64> {
        runtime.owner_fence().ok()?;
        if cells.scope().world_id != runtime.binding().world_id()
            || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
            || navigation != runtime.content_pin().map_revision_digest()
        {
            for entry in self.entries.values_mut() {
                entry.motion.stop();
            }
            self.step_timers = None;
            return None;
        }
        self.forget_dead(runtime);
        self.refresh_goals(runtime, cells, blocking, navigation, now, source);
        self.drain_creature_paths(runtime, cells, blocking, now, budget, player_waiting);
        self.fire_steps(runtime, cells, blocking, source, ground, root, now);
        let mut next = self.entries.values().filter_map(|e| e.motion.due).min();
        if self.entries.values().any(|e| e.motion.pending.is_some()) {
            let window = (now / crate::movement::path_budget::PATH_WINDOW_US)
                .checked_add(1)
                .and_then(|window| {
                    window.checked_mul(crate::movement::path_budget::PATH_WINDOW_US)
                });
            if let Some(window) = window {
                next = Some(next.map_or(window, |due| due.min(window)));
            }
        }
        next
    }
}

fn cell(at: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: at.x,
        y: at.y,
        z: i32::from(at.floor),
    }
}
