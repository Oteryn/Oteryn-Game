// W17: native actions and decoded results must cross the production session loop.
#[test]
fn arena_controls_cross_the_session_loop() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, play_channel, run_session};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_session::{GatedCommandResult, SessionError};
            struct Features;
            fn result(kind: u32) -> GatedCommandResult {
                GatedCommandResult {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: if kind == 240 {
                        oteryn_session::arena::encode_catalogue().unwrap_or_default()
                    } else {
                        vec![8, 3, 16, 1]
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
                    capability == 170
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    Ok(result(command.kind))
                }
            }
            let (link, commands, outbox) = play_channel();
            outbox.arena_initialize(true);
            link.arena_request(oteryn_client_ui::arena::Intent::Catalogue);
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Features, commands, outbox));
            let panel = link.arena_panel();
            assert!(panel.pending.is_none());
            assert_eq!(panel.challenges.len(), 3);
            assert!(oteryn_client_ui::arena::panel_text(&panel).contains("Greenhorn"));
            let (link, commands, outbox) = play_channel();
            outbox.arena_initialize(true);
            link.arena_request(oteryn_client_ui::arena::Intent::Action(
                oteryn_session::arena::ArenaAction::Enter,
            ));
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Features, commands, outbox));
            let panel = link.arena_panel();
            assert_eq!(
                panel.result.map(|r| r.disposition),
                Some(oteryn_session::arena::ArenaDisposition::InsufficientGold)
            );
            assert!(oteryn_client_ui::arena::panel_text(&panel).contains("Insufficient gold"));
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "arena session-loop test thread panicked")?
}
