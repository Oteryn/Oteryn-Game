use oteryn_client_ui::daily_boost::{DailyBoostPanel, Intent, request};
#[test]
fn daily_boost_maps_supported_command() {
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    let command = request(
        &mut panel,
        Intent {
            command: 177,
            payload: Vec::new(),
        },
    );
    assert_eq!(
        command.map(|value| (value.capability, value.kind)),
        Some((151, 177))
    );
}

// CL25J exercises both newly composed command paths through the actual session loop.
#[test]
fn cl25j_feature_commands_cross_the_session_loop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, play_channel, run_session};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
            use oteryn_session::{GatedCommandResult, SessionError};
            struct Features;
            fn result(capability: u32) -> GatedCommandResult {
                GatedCommandResult {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: capability.to_be_bytes().to_vec(),
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
                fn feature_selected(&self, _: u32) -> bool {
                    true
                }
                fn server_mechanic_selected(&self, _: u32) -> bool {
                    true
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
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
            outbox.daily_boost_initialize(true);
            outbox.item_forge_initialize(true);
            link.daily_boost_request(Intent {
                command: 177,
                payload: vec![],
            });
            link.item_forge_request(vec![2]);
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Features, commands, outbox));
            let daily = link.daily_boost_panel();
            assert!(!daily.pending);
            assert_eq!(daily.result, 151_u32.to_be_bytes());
            let forge = link.item_forge_panel();
            assert!(!forge.pending);
            assert_eq!(forge.response, 152_u32.to_be_bytes());
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "CL25J session-loop test thread panicked")?
}
