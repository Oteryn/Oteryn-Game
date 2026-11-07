// Drive the play input, command queue, runtime result routing and visible projection.
#[test]
fn world_event_shell_query_crosses_the_session_loop(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{play_channel, run_session, Stepper};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
            use oteryn_session::{GatedCommandResult, SessionError};
            struct Features;
            fn result(_: u32) -> Result<GatedCommandResult, SessionError> {
                let mut reply = GatedCommandResult {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: vec![
                        8, 9, 16, 1, 26, 4, 70, 97, 105, 114, 34, 16, 8, 1, 16, 1, 24, 208, 235,
                        149, 215, 6, 32, 208, 212, 165, 215, 6, 34, 14, 8, 2, 24, 208, 168, 185,
                        216, 6, 32, 208, 229, 220, 217, 6, 34, 14, 8, 3, 24, 208, 222, 233, 219, 6,
                        32, 208, 176, 137, 220, 6,
                    ],
                };
                let mut state =
                    oteryn_session::world_event::decode_world_event_state(&reply.payload)
                        .map_err(SessionError::Chat)?;
                state.cake = Some(oteryn_session::world_event::CakeProgress {
                    stage: 0,
                    substances: 200,
                    golems: 200,
                    layer: 4,
                    layer_progress: 1116,
                    layer_goal: 1116,
                    occurrence_start: 1_799_000_000,
                    occurrence_end: 1_799_518_400,
                    window_end: None,
                    bonus_until: Some(1_800_604_800),
                    observed_unix: 1_800_100_000,
                });
                reply.payload = oteryn_session::world_event::encode_world_event_state(&state)
                    .map_err(SessionError::Chat)?;
                Ok(reply)
            }
            impl Stepper for Features {
                async fn step(
                    &mut self,
                    _: oteryn_session::StepDirection,
                ) -> Result<oteryn_session::StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(&mut self, _: std::time::Duration) -> Result<(), SessionError> {
                    Err(SessionError::SessionUnusable)
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
                fn feature_selected(&self, _: u32) -> bool {
                    true
                }
                // oteryn-choice: the all-capability fixture must provide the selected catalogue.
                fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
                    (domain == 100).then(Vec::new)
                }
                fn server_mechanic_selected(&self, _: u32) -> bool {
                    true
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    assert_eq!((command.capability, command.kind), (53, 64));
                    assert!(command.payload.is_empty());
                    result(command.capability)
                }
                async fn server_mechanic(
                    &mut self,
                    command: &ServerMechanicCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    result(command.capability)
                }
            }
            let (link, commands, outbox) = play_channel();
            outbox.world_event_initialize(true, true);
            let event = oteryn_input_actions::NormalizedInputEvent::Key {
                code: oteryn_input_actions::KeyCode::new(17)?,
                state: oteryn_input_actions::ButtonState::Pressed,
                modifiers: oteryn_input_actions::Modifiers::one(
                    oteryn_input_actions::Modifier::Control,
                ),
                repeat: false,
            };
            assert!(link.world_event_input(&[event]));
            assert!(link.world_event_panel().pending);
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Features, commands, outbox));
            let panel = link.world_event_panel();
            assert!(!panel.pending);
            assert!(panel.open);
            let text = oteryn_client_ui::seasonal_events::panel_text(&panel);
            assert!(text.contains("Fair"));
            assert!(text.contains("Halloween: active"));
            assert!(text.contains("2026-10-31 05:00 UTC"));
            assert!(text.contains("Christmas — Santa and presents: scheduled"));
            assert!(text.contains("Birthday Cake — A Piece of Cake: scheduled"));
            assert!(text.contains("A Piece of Cake: event finished"));
            assert!(text.contains("Premium experience +50%; bonus regeneration +25%"));
            assert!(text.contains("Server observation:"));
            assert!(!text.contains("seconds remaining"));
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "world-event session-loop test thread panicked")?
}
