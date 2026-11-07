/// A positioned actor on a one-row room: east of the start is walkable, every other
/// direction is blocked.
struct StepAuthority {
    steps: RefCell<Vec<StepDirection>>,
    tutorial_notices: super::super::tutorial_session::TutorialSessions,
}

impl FreshAdmissionAuthority for StepAuthority {
    fn take_tutorial_notice(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        self.tutorial_notices.take(session)
    }
    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
        let outcome = self.step(actor, direction).await;
        let notice = if direction == StepDirection::East {
            TutorialNotice::Hint(2)
        } else {
            TutorialNotice::StatusText(oteryn_gameplay_quests::tutorial::HOLE_TEXT.into())
        };
        self.tutorial_notices.notice(session, notice);
        (outcome, None)
    }
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn step(&self, _actor: ExactActorRef, direction: StepDirection) -> StepOutcome {
        self.steps.borrow_mut().push(direction);
        if direction == StepDirection::East {
            StepOutcome {
                disposition: StepDisposition::Moved,
                moved_to: Some(at(1)),
            }
        } else {
            StepOutcome {
                disposition: StepDisposition::Blocked,
                moved_to: None,
            }
        }
    }
}
