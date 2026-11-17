impl Stepper for Pushing {
    async fn combat(&mut self, _command: CombatCommand) -> Result<CombatReport, SessionError> {
        Ok(CombatReport::Cast(SpellCastDisposition::NotEnoughMana))
    }

    async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
        Ok(moved_to(100, 199, 0))
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Busy))
    }

    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        for _ in 0..200 {
            self.sequence += 1;
            self.queue.push(SessionEvent::WorldSpatial(AppliedDelta {
                server_sequence: self.sequence,
                base_revision: self.sequence,
                new_revision: self.sequence + 1,
                value: WorldSpatialObservation {
                    content_generation: [0; 32],
                    actor_position: ActorPosition {
                        x: 100,
                        y: 200,
                        floor: 0,
                    },
                },
            }));
        }
        if self.queue.len() > MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        tokio::time::sleep(duration).await;
        Ok(())
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.queue)
    }
}
