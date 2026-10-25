//! Islet-hole refusal and committed movement tutorial transitions.
use super::*;
use oteryn_gameplay_quests::tutorial::{FORCEFIELD_ARRIVAL, HOLE_TEXT, ISLET_HOLE, TutorialEvent};
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn tutorial_paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        self.tutorial_paced_step_from(actor, session, direction, None)
            .await
    }

    pub(super) async fn tutorial_paced_step_from(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
        expected: Option<crate::foundation::MovementPositionSnapshot>,
    ) -> (StepOutcome, Option<Duration>) {
        if self.states.newhaven_travel.held(session) {
            return (StepOutcome::rejected(), None);
        }
        if !self.tutorial_enabled().await {
            return self
                .ordinary_paced_step(actor, session, direction, expected)
                .await;
        }
        let here = {
            let mut runtime = self.runtime.lock().await;
            if runtime.player_control_facts(actor, session).is_err()
                || runtime.owner_fence().is_err()
                || expected.is_some_and(|expected| {
                    runtime.borrow_movement_position().read(actor) != Ok(expected)
                })
            {
                return (StepOutcome::rejected(), None);
            }
            runtime
                .read_actor_position(actor)
                .ok()
                .map(|at| at.position())
        };
        let Some(here) = here else {
            return (StepOutcome::rejected(), None);
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return (StepOutcome::rejected(), None);
        };
        if self.reconcile_tutorial_position(fence, here).await.is_err() {
            return (StepOutcome::rejected(), None);
        }
        let (dx, dy) = match direction {
            StepDirection::North => (0, -1),
            StepDirection::East => (1, 0),
            StepDirection::South => (0, 1),
            StepDirection::West => (-1, 0),
        };
        if (here.x + dx, here.y + dy, here.floor) == ISLET_HOLE
            && super::tutorial_session::tutorial_cell(here)
        {
            self.states
                .tutorial_sessions
                .notice(session, TutorialNotice::StatusText(HOLE_TEXT.into()));
            return (
                StepOutcome {
                    disposition: StepDisposition::Blocked,
                    moved_to: None,
                },
                None,
            );
        }
        let outcome = self
            .ordinary_paced_step(actor, session, direction, expected)
            .await;
        if let Some(moved) = outcome.0.moved_to {
            let cell = moved.actor_position;
            let event = match (cell.x, cell.y, cell.floor) {
                FORCEFIELD_ARRIVAL => Some(TutorialEvent::Forcefield),
                (32561 | 32562, 32485, 7) => Some(TutorialEvent::Hunting),
                _ => None,
            };
            if let Some(event) = event
                && let Ok(Some(fence)) = self.current_quest_fence(session).await
                && self.tutorial_event(fence, event).await.is_err()
            {
                operator_event("tutorial_movement_progress_unavailable");
            }
        }
        outcome
    }
}
