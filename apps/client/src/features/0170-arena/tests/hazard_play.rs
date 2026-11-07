//! Native projection of authoritative Drome tile warnings, without invented Item/effect IDs.
use crate::play::{PlayView, Stepper, play_channel, run_session};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_session::{GatedCommandResult, SessionError};
type Error = Box<dyn std::error::Error + Send + Sync>;
#[test]
fn srx3_drome_hazards_cross_native_session_loop_and_expire_visually() -> Result<(), Error> {
    play(false)
}
#[test]
fn srx3_drome_selected_modifiers_have_native_legend_before_any_marks() -> Result<(), Error> {
    play(true)
}
fn play(selected_only: bool) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            struct Server { selected_only: bool }
            impl Stepper for Server {
                async fn step(
                    &mut self,
                    _: oteryn_session::StepDirection,
                ) -> Result<oteryn_session::StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(&mut self, _: std::time::Duration) -> Result<(), SessionError> {
                    tokio::task::yield_now().await;
                    Ok(())
                }
                fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
                    Vec::new()
                }
                async fn combat(
                    &mut self,
                    _: crate::combat_input::CombatCommand,
                ) -> Result<crate::combat_input::CombatReport, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn logout(
                    &mut self,
                ) -> Result<Option<oteryn_session::LogoutResult>, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                fn feature_selected(&self, capability: u32) -> bool {
                    capability == 170
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    assert_eq!(command.kind, 436);
                    let mut snapshot = oteryn_session::arena::DromeHazards {
                        selected: [0, 0],
                        observed_at_micros: 15_000_000,
                        entries: vec![
                            oteryn_session::arena::DromeHazard {
                                kind: oteryn_session::arena::DromeHazardKind::Lava,
                                expires_at_micros: 18_000_000,
                                positions: (0..100)
                                    .map(|index| oteryn_session::ActorPosition {
                                        x: index % 10,
                                        y: index / 10,
                                        floor: 0,
                                    })
                                    .collect(),
                            },
                            oteryn_session::arena::DromeHazard {
                                kind: oteryn_session::arena::DromeHazardKind::Sorrow,
                                expires_at_micros: 21_000_000,
                                positions: vec![oteryn_session::ActorPosition {
                                    x: 1,
                                    y: 1,
                                    floor: 0,
                                }],
                            },
                        ],
                    };
                    if self.selected_only {
                        snapshot.selected = [1, 4];
                        snapshot.entries.clear();
                    }
                    let payload = oteryn_session::arena::append_drome_hazards(
                        oteryn_session::arena::DromeResult {
                            disposition: 1,
                            phase: 2,
                            ..Default::default()
                        }
                        .encode(),
                        &snapshot,
                    )
                    .map_err(|_| SessionError::SessionUnusable)?;
                    let decoded=oteryn_session::arena::decode_drome_hazards(&payload).map_err(|_|SessionError::SessionUnusable)?;
                if self.selected_only {
                    assert_eq!(decoded.selected, [1, 4], "peer must emit actual field13 selected modifiers before native legend assertion");
                } else {
                    assert_eq!(decoded.entries.iter().map(|entry|entry.positions.len()).sum::<usize>(),101);
                }
                eprintln!("native Drome peer emitted verified field13: selected={:?}, hazard sets={}", decoded.selected, decoded.entries.len());
                Ok(GatedCommandResult {
                        command_id: 1,
                        status: oteryn_session::CommandStatus::Accepted,
                        server_sequence: 1,
                        payload,
                    })
                }
            }
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                let (link, commands, outbox) = play_channel();
                outbox.arena_initialize(true);
                link.arena_request(oteryn_client_ui::arena::Intent::Action(
                    oteryn_session::arena::ArenaAction::Drome(
                        oteryn_session::arena::DromeAction::Status,
                    ),
                ));
                let task = tokio::spawn(run_session(Server { selected_only }, commands, outbox));
                let mut view = PlayView::new(oteryn_renderer::TileCoord::new(4, 4), 0, &[])?;
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while link.arena_panel().drome.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                view.tick(&link)
                    .map_err(|error| format!("native arena render: {error:?}"))?;
                if selected_only {
                    let panel = link.arena_panel();
                    assert_eq!(panel.hazards.selected, [1, 4], "native reducer must decode selected modifier pair");
                    let text = oteryn_client_ui::arena::panel_text(&panel);
                    assert!(text.contains("Somersault"), "native legend missing selected Somersault: {text}");
                    assert!(text.contains("That Escalated Quickly"), "native legend missing selected Escalated: {text}");
                    assert_eq!(view.scene().drome_hazard_tiles(), 0);
                    drop(link);
                    task.await?;
                    return Ok::<(), Error>(());
                }
                assert_eq!(view.scene().drome_hazard_tiles(), 101);
                view.rebuild().map_err(|error|format!("native scene recenter: {error:?}"))?;
                assert_eq!(view.scene().drome_hazard_tiles(),101,"map/actor recenter must preserve current scope warnings");
                assert!(view.scene().sprites().instances().iter().filter(|quad|quad.uv[0]==quad.uv[2] && quad.uv[1]==quad.uv[3]).count()>=101,"authoritative warnings must produce native geometry");
                assert!(
                    oteryn_client_ui::arena::panel_text(&link.arena_panel()).contains("green seed")
                );
                view.project_arena_hazards(&link,std::time::Instant::now()+std::time::Duration::from_secs(7))
                    .map_err(|error|format!("native visual expiry: {error:?}"))?;
                assert_eq!(view.scene().drome_hazard_tiles(),0);
                assert_eq!(link.arena_panel().drome.map(|reply|reply.phase),Some(2),"visual expiry cannot end the server battle");
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "native Drome hazard play thread panicked")?
}
