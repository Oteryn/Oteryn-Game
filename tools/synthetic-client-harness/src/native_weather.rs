//! C27: production initialization, task, mailbox, and weather scene projection.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::play::Stepper;
use oteryn_client::play::{PlayView, play_channel, run_session};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, AppliedDelta, SessionEvent, WorldSpatialObservation, world_weather::WorldWeather,
};
use oteryn_session::{LogoutResult, SessionError, StepDirection, StepOutcome};
use std::time::Duration;
use tokio::sync::mpsc;
pub(super) struct WeatherSession {
    pub(super) incoming: mpsc::UnboundedReceiver<SessionEvent>,
    pub(super) pending: Vec<SessionEvent>,
}

impl Stepper for WeatherSession {
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        if domain == 22 {
            oteryn_protocol_oteryn::world_weather::encode_world_weather(WorldWeather::Active {
                binding_token: [1; 32],
                absolute_slot: 0,
                kind: 3,
                intensity: 2,
            })
            .ok()
        } else {
            None
        }
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::Timeout("synthetic weather"))
    }
    async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
        self.pending.push(
            self.incoming
                .recv()
                .await
                .ok_or(SessionError::Timeout("synthetic weather"))?,
        );
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.pending)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::Timeout("synthetic weather"))
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Err(SessionError::Timeout("synthetic weather"))
    }
}

#[test]
fn native_weather_update_reaches_scene_and_clears()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (link, commands, outbox) = play_channel();
                    let (send, incoming) = tokio::sync::mpsc::unbounded_channel();
                    let task = tokio::spawn(run_session(
                        WeatherSession {
                            incoming,
                            pending: vec![],
                        },
                        commands,
                        outbox,
                    ));
                    let mut view = PlayView::new(TileCoord::new(0, 0), -7, &[])?;
                    let original = view.scene().atlas().clone();
                    let initial = oteryn_renderer::weather_atlas(&original, [65, 105, 155], 48);
                    tokio::time::timeout(Duration::from_secs(5), async {
                        loop {
                            view.tick(&link).map_err(|e| format!("{e:?}"))?;
                            if view.scene().atlas() == &initial {
                                break;
                            }
                            tokio::task::yield_now().await;
                        }
                        Ok::<_, String>(())
                    })
                    .await??;
                    // Floor changes travel through the production task and mailbox, while
                    // the weather observation remains active and unchanged.
                    for (revision, floor) in [(1, -8), (2, -7)] {
                        send.send(SessionEvent::WorldSpatial(AppliedDelta {
                            server_sequence: revision,
                            base_revision: revision - 1,
                            new_revision: revision,
                            value: WorldSpatialObservation {
                                content_generation: [1; 32],
                                actor_position: ActorPosition { x: 0, y: 0, floor },
                            },
                        }))?;
                        let expected = if floor == -8 { &original } else { &initial };
                        tokio::time::timeout(Duration::from_secs(5), async {
                            loop {
                                view.tick(&link).map_err(|e| format!("{e:?}"))?;
                                if view.scene().atlas() == expected {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            Ok::<_, String>(())
                        })
                        .await??;
                        assert_eq!(view.scene().atlas(), expected);
                    }
                    for (revision, intensity) in [(1, 1), (2, 3), (3, 0)] {
                        send.send(SessionEvent::WorldWeather(AppliedDelta {
                            server_sequence: revision + 2,
                            base_revision: revision - 1,
                            new_revision: revision,
                            value: if intensity == 0 {
                                WorldWeather::Unavailable
                            } else {
                                WorldWeather::Active {
                                    binding_token: [1; 32],
                                    absolute_slot: 0,
                                    kind: 3,
                                    intensity,
                                }
                            },
                        }))?;
                        let expected = if intensity == 0 {
                            original.clone()
                        } else {
                            oteryn_renderer::weather_atlas(
                                &original,
                                [65, 105, 155],
                                intensity * 24,
                            )
                        };
                        tokio::time::timeout(std::time::Duration::from_secs(5), async {
                            loop {
                                view.tick(&link).map_err(|e| format!("{e:?}"))?;
                                if view.scene().atlas() == &expected {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            Ok::<_, String>(())
                        })
                        .await??;
                        assert_eq!(view.scene().atlas(), &expected);
                    }
                    task.abort();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "native weather scenario thread failed")?
}
