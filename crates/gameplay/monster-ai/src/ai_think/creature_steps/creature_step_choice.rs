//! Evaluate one local goal or consume one immutable route waypoint at a step deadline.
use super::*;
use crate::ai_think::{ThinkOccurrence, profile_schedule::decision_occurrence};
use oteryn_simulation_determinism::deterministic_decision_u64;

impl CreatureMotion {
    pub(super) fn choose_step(
        &mut self,
        actor: ExactActorRef,
        at: MovementLocalPosition,
        static_chance: u32,
        sight_clear: bool,
        root: &GameplayDecisionRoot,
        admits: impl Fn(MovementLocalPosition) -> bool,
    ) -> Option<crate::movement::CardinalStep> {
        let purpose = match self.goal {
            MovementGoal::Wander { .. } => Some("AI_WANDER"),
            MovementGoal::Chase { at: target, .. }
                if crate::movement::path_search::chebyshev(at, target) == 1 =>
            {
                Some("AI_DANCE")
            }
            _ => None,
        };
        let draw = match purpose {
            Some(purpose) => deterministic_decision_u64(
                root,
                decision_occurrence(ThinkOccurrence {
                    actor,
                    sequence: self.seen_think,
                }),
                purpose,
                self.step_sequence,
            )
            .ok()?,
            None => 0,
        };
        if matches!(self.goal, MovementGoal::KeepDistance { at: target, distance, .. }
            if sight_clear && crate::movement::path_search::chebyshev(at, target) == i64::from(distance))
        {
            self.route = None;
            return None;
        }
        if (!matches!(self.goal, MovementGoal::KeepDistance { .. }) || sight_clear)
            && let Some(step) = self.goal.local_step(at, static_chance, draw, &admits)
        {
            self.route = None;
            return Some(step);
        }
        if matches!(self.goal, MovementGoal::Chase { at: target, .. } if crate::movement::path_search::chebyshev(at, target) <= 1)
            || !matches!(
                self.goal,
                MovementGoal::Chase { .. }
                    | MovementGoal::KeepDistance { .. }
                    | MovementGoal::WalkBack { .. }
            )
        {
            return None;
        }
        if self.expected != Some(at) {
            self.route = None;
        }
        let node = self.route.as_ref()?.steps().get(self.cursor)?.node();
        crate::ai_think::movement_goal::CARDINALS
            .get(usize::try_from(node).ok()?)
            .copied()
    }
}
