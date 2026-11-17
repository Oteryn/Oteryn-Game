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

#[test]
fn tibiadrome_controls_cross_session_and_preserve_server_authority()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, play_channel, run_session};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_client_ui::arena::{DromeAction, Intent, panel_text};
            use oteryn_session::{GatedCommandResult, SessionError};
            struct Server {
                request: Vec<u8>,
                response: Vec<u8>,
            }
            impl Stepper for Server {
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
                    assert_eq!((command.capability, command.kind), (170, 436));
                    assert_eq!(command.payload, self.request);
                    Ok(GatedCommandResult {
                        command_id: 1,
                        status: oteryn_session::CommandStatus::Accepted,
                        server_sequence: 1,
                        payload: self.response.clone(),
                    })
                }
            }
            let mut actions = vec![
                (DromeAction::Status, vec![8, 6, 16, 1]),
                (DromeAction::Enter, vec![8, 6, 16, 2]),
                (DromeAction::Leave, vec![8, 6, 16, 3]),
                (DromeAction::LowerStart, vec![8, 6, 16, 4]),
                (DromeAction::ClaimPotion, vec![8, 6, 16, 5]),
            ];
            for selector in 1..=18 {
                actions.push((DromeAction::Buy(selector), vec![8, 6, 16, 6, 24, selector]));
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            for (index, (action, request)) in actions.into_iter().enumerate() {
                let (link, commands, outbox) = play_channel();
                outbox.arena_initialize(true);
                let (control, selected) = if index < 5 {
                    (200 + index, -1)
                } else {
                    (205, (index - 5) as isize)
                };
                let intent = oteryn_client_ui::arena::drome_intent(control, selected)
                    .ok_or("native control")?;
                assert_eq!(
                    intent,
                    Intent::Action(oteryn_client_runtime::arena::ArenaAction::Drome(action))
                );
                link.arena_request(intent);
                assert!(link.arena_panel().drome.is_none());
                let phase = (index % 3) as u8;
                let mut response = vec![8, 2, 16, 8, 24, 15, 32, 10, 40, 150, 1, 80, 2, 88, 3];
                response.extend_from_slice(&[48, phase]);
                runtime.block_on(run_session(Server { request, response }, commands, outbox));
                let panel = link.arena_panel();
                let state = panel.drome.ok_or("missing Drome reply")?;
                assert_eq!(
                    (
                        state.level,
                        state.start,
                        state.points,
                        state.rank,
                        state.potions
                    ),
                    (15, 10, 150, 2, 3)
                );
                assert!(panel.pending.is_none());
                assert!(panel.result.is_none());
                let text = panel_text(&panel);
                assert!(text.contains("Inventory full"));
                assert!(text.contains("Drome points: 150"));
                assert!(text.contains("Unclaimed potions: 3"));
                assert!(text.contains(match phase {
                    0 => "Idle: wave",
                    1 => "Preparation: wave",
                    _ => "Fighting: wave",
                }));
            }
            // Invalid wire data must release pending without publishing a bogus score.
            let (link, commands, outbox) = play_channel();
            outbox.arena_initialize(true);
            link.arena_request(Intent::Action(
                oteryn_client_runtime::arena::ArenaAction::Drome(DromeAction::Status),
            ));
            runtime.block_on(run_session(
                Server {
                    request: vec![8, 6, 16, 1],
                    response: vec![8, 2, 16, 9],
                },
                commands,
                outbox,
            ));
            let panel = link.arena_panel();
            assert!(panel.drome.is_none());
            assert!(panel.pending.is_none());
            assert!(panel_text(&panel).contains("Invalid arena reply"));
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "Tibiadrome play test thread panicked")?
}

#[test]
fn arena_pending_controls_close_on_disconnect_and_logout() -> Result<(), &'static str> {
    use oteryn_client_ui::arena::Intent;
    for logout in [false, true] {
        let (link, _commands, outbox) = crate::play::play_channel();
        outbox.arena_initialize(true);
        link.arena_request(Intent::Action(oteryn_session::arena::ArenaAction::Drome(
            oteryn_session::arena::DromeAction::Status,
        )));
        assert!(link.arena_panel().pending.is_some());
        if logout {
            outbox.logged_out();
        } else {
            outbox.end(crate::play::public_class(
                &oteryn_session::SessionError::SessionUnusable,
            ));
        }
        let panel = link.arena_panel();
        assert!(
            panel.pending.is_none(),
            "terminal session leaves the arena waiting"
        );
        assert!(!panel.selected);
        assert_eq!(panel.feedback, Some("The session has ended"));
        assert!(panel.drome.is_none());
    }
    Ok(())
}

#[path="hazard_play.rs"]
mod hazard_play;
