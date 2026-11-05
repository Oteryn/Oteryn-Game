//! C20: production mailbox and visible HUD consume the existing lifecycle deltas.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::input::StepDir;
use oteryn_client::play::{PlayView, Stepper, play_channel, run_session};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, ActorVitals, AppliedDelta, CommandStatus, LogoutResult, SessionError,
    SessionEvent, StepDirection, StepDisposition, StepOutcome, WorldSpatialObservation,
};
use std::time::{Duration, Instant};

struct RespawnSession(std::sync::mpsc::Sender<StepDirection>);

impl Stepper for RespawnSession {
    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    async fn step(&mut self, direction: StepDirection) -> Result<StepOutcome, SessionError> {
        self.0
            .send(direction)
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(StepOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: StepDisposition::Moved,
            result_server_sequence: 1,
            world_spatial_delta: None,
            world_object_overlay_delta: None,
            gate_message_id: None,
        })
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Accepted))
    }
}

#[test]
fn death_notice_blocks_world_input_until_server_respawn() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[]).map_err(|e| e.to_string())?;
    let vitals = |health, revision| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: revision,
            base_revision: revision - 1,
            new_revision: revision,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })
    };
    outbox.push(vec![vitals(150, 1)]);
    assert_eq!(view.tick(&link), Ok(()));
    view.arrow(StepDir::North);
    outbox.push(vec![vitals(0, 2)]);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.hud_status().contains("You are dead"));
    view.arrow(StepDir::East);
    view.click(TileCoord::new(101, 200))
        .map_err(|e| e.to_string())?;
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let (steps, received_steps) = std::sync::mpsc::channel();
    let task = runtime.spawn(run_session(RespawnSession(steps), commands, outbox.clone()));
    outbox.push(vec![
        SessionEvent::WorldSpatial(AppliedDelta {
            server_sequence: 3,
            base_revision: 1,
            new_revision: 2,
            value: WorldSpatialObservation {
                content_generation: [1; 32],
                actor_position: ActorPosition {
                    x: 20,
                    y: 30,
                    floor: 7,
                },
            },
        }),
        vitals(150, 4),
    ]);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(view.own(), TileCoord::new(20, 30));
    let deadline = Instant::now() + Duration::from_secs(3);
    while view.hud_status().contains("You are dead") && Instant::now() < deadline {
        assert_eq!(view.tick(&link), Ok(()));
        std::thread::yield_now();
    }
    assert!(!view.hud_status().contains("You are dead"));
    assert!(received_steps.try_recv().is_err());
    view.arrow(StepDir::South);
    assert_eq!(view.tick(&link), Ok(()));
    while received_steps.try_recv().is_err() {
        if Instant::now() >= deadline {
            return Err("post-respawn step stuck".into());
        }
        assert_eq!(view.tick(&link), Ok(()));
        std::thread::yield_now();
    }
    drop(link);
    runtime.block_on(task).map_err(|e| e.to_string())?;
    Ok(())
}
