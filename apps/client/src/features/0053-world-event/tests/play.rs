// Drive the play input, command queue, runtime result routing and visible projection.
#[test]
fn world_event_shell_query_crosses_the_session_loop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, play_channel, run_session};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
            use oteryn_session::{GatedCommandResult, SessionError};
            struct Features;
            fn result(_: u32) -> GatedCommandResult {
                GatedCommandResult {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: vec![8, 9, 16, 1, 26, 4, b'F', b'a', b'i', b'r'],
                }
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
                fn feature_selected(&self, capability: u32) -> bool {
                    capability != oteryn_client_runtime::server_modal::CAPABILITY_SERVER_MODAL_V1
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
                    // All-capability sessions also refresh the retained death receipt at login.
                    if command.capability == 382 {
                        assert_eq!(command.kind, 664);
                        assert!(command.payload.is_empty());
                        return Ok(GatedCommandResult {
                            payload: vec![0],
                            ..result(command.capability)
                        });
                    }
                    // The selected marker capability is queried before queued shell commands.
                    if command.capability == 451 {
                        assert_eq!(command.kind, 741);
                        assert!(command.payload.is_empty());
                        return Ok(GatedCommandResult {
                            payload: oteryn_client_runtime::personal_map_marks::PersonalMapMarks::default()
                                .encode()
                                .ok_or(SessionError::SessionUnusable)?,
                            ..result(command.capability)
                        });
                    }
                    assert_eq!((command.capability, command.kind), (53, 64));
                    assert!(command.payload.is_empty());
                    Ok(result(command.capability))
                }
                async fn server_mechanic(
                    &mut self,
                    command: &ServerMechanicCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    Ok(result(command.capability))
                }
            }
            let (link, commands, outbox) = play_channel();
            outbox.world_event_initialize(true);
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
            assert!(oteryn_client_ui::world_event::panel_text(&panel).contains("Fair"));
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "world-event session-loop test thread panicked")?
}
