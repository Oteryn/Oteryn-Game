#[test]
fn hireling_management_crosses_the_real_session_loop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, play_channel, run_session};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_session::{GatedCommandResult, SessionError};
            struct Features;
            fn result(capability: u32) -> GatedCommandResult {
                GatedCommandResult {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: if capability == 250 {
                        vec![0, 0]
                    } else {
                        vec![255]
                    },
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
                    capability == 250
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    Ok(result(command.capability))
                }
            }
            let (link, commands, outbox) = play_channel();
            outbox.hireling_management_initialize(true);
            link.hireling_management_request(oteryn_client_ui::hireling_management::Intent::List);
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Features, commands, outbox));
            let panel = link.hireling_management_panel();
            assert!(panel.pending.is_none());
            assert!(panel.entries.is_empty());
            assert!(panel.feedback.is_none());
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "E12 session-loop test thread panicked")?
}
