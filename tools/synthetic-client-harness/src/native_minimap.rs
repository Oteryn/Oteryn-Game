//! Native task scenario: live map -> overlay click -> capability-64 server walk.
use super::native_map::MapSession;
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::{PlayView, Stepper, play_channel, run_session};
use oteryn_client_ui::target_move::{WalkDisposition, WalkResponse};
use oteryn_renderer::TileCoord;
use oteryn_session::world_map::{MapViewHeader, WorldMapViewSnapshot};
use oteryn_session::{
    ActorPosition, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
use tokio::sync::mpsc;

struct MinimapSession {
    map: MapSession,
    walks: mpsc::UnboundedSender<ActorPosition>,
}

impl Stepper for MinimapSession {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 64
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        self.map.feature_snapshot(domain)
    }
    async fn target_walk(&mut self, position: ActorPosition) -> Result<WalkResponse, SessionError> {
        self.walks
            .send(position)
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(WalkResponse {
            status: oteryn_session::CommandStatus::Accepted,
            disposition: Some(WalkDisposition::Accepted),
        })
    }
    async fn step(&mut self, direction: StepDirection) -> Result<StepOutcome, SessionError> {
        self.map.step(direction).await
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        // Unlike the push-only map peer, this peer must service queued commands
        // when its idle slice expires, matching the production Stepper contract.
        match tokio::time::timeout(duration, self.map.serve(duration)).await {
            Ok(result) => result,
            Err(_) => Ok(()),
        }
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        self.map.drain()
    }
    async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
        self.map.combat(command).await
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        self.map.logout().await
    }
}

#[test]
fn native_minimap_click_reaches_the_server_walk_path()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let own = ActorPosition {
                        x: 100,
                        y: 200,
                        floor: -7,
                    };
                    let initial = WorldMapViewSnapshot {
                        header: MapViewHeader {
                            content_generation: [0; 32],
                            bundle_digest: [0; 32],
                            reset_epoch: 1,
                            origin: own,
                            first_visible_floor: -7,
                        },
                        tiles: vec![],
                    };
                    let (link, commands, outbox) = play_channel();
                    let (_send, incoming) = mpsc::unbounded_channel();
                    let (walks, mut received) = mpsc::unbounded_channel();
                    let task = tokio::spawn(run_session(
                        MinimapSession {
                            map: MapSession {
                                appearances: Vec::new(),
                                pending: vec![SessionEvent::WorldMap(
                                    oteryn_session::AppliedDelta {
                                        server_sequence: 1,
                                        base_revision: 1,
                                        new_revision: 2,
                                        value: oteryn_session::world_map::WorldMapViewDelta {
                                            header: initial.header,
                                            tiles: vec![],
                                            cleared: vec![],
                                        },
                                    },
                                )],
                                initial,
                                incoming,
                            },
                            walks,
                        },
                        commands,
                        outbox,
                    ));
                    let mut view = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
                    tokio::time::timeout(Duration::from_secs(5), async {
                        loop {
                            view.tick(&link).map_err(|e| format!("{e:?}"))?;
                            if link.target_walk_state().selected
                                && view
                                    .scene()
                                    .sprites()
                                    .instances()
                                    .iter()
                                    .any(|q| q.size == [4.0; 2])
                            {
                                break;
                            }
                            tokio::task::yield_now().await;
                        }
                        Ok::<_, String>(())
                    })
                    .await??;
                    assert!(view.minimap_click(66.0, 50.0));
                    view.tick(&link).map_err(|e| format!("{e:?}"))?;
                    let goal = tokio::time::timeout(Duration::from_secs(5), received.recv())
                        .await?
                        .ok_or("walk channel ended")?;
                    assert_eq!(
                        goal,
                        ActorPosition {
                            x: 101,
                            y: 200,
                            floor: -7
                        }
                    );
                    assert_eq!(view.own(), TileCoord::new(100, 200));
                    task.abort();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "minimap scenario thread failed")?
}
