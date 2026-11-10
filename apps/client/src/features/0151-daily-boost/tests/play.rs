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
                    payload: if capability == 151 {
                        let mut payload = 42_u64.to_be_bytes().to_vec();
                        payload.extend([3, b'r', b'a', b't', 4, b'b', b'o', b's', b's']);
                        payload
                    } else {
                        capability.to_be_bytes().to_vec()
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
                    let mut reply = result(command.capability);
                    // Selected automatic queries precede the queued Daily Boost/Forge commands.
                    match command.capability {
                        450 => {
                            assert_eq!(command.kind, 736);
                            assert!(command.payload.is_empty());
                            reply.payload = oteryn_client_runtime::server_modal::Reply::None
                                .encode()
                                .map_err(|_| SessionError::SessionUnusable)?;
                        }
                        451 => {
                            assert_eq!(command.kind, 741);
                            assert!(command.payload.is_empty());
                            reply.payload = oteryn_client_runtime::personal_map_marks::PersonalMapMarks::default()
                                .encode()
                                .ok_or(SessionError::SessionUnusable)?;
                        }
                        _ => {}
                    }
                    Ok(reply)
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
            use oteryn_input_actions::{
                ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
            };
            assert!(link.daily_boost_input(&[NormalizedInputEvent::Key {
                code: KeyCode::KEY_B,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control),
                repeat: false,
            }]));
            link.item_forge_request(vec![2]);
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(run_session(Features, commands, outbox));
            let daily = link.daily_boost_panel();
            assert!(!daily.pending);
            assert_eq!(
                daily
                    .boost
                    .as_ref()
                    .map(|boost| boost.creature_key.as_str()),
                Some("rat")
            );
            assert!(oteryn_client_ui::daily_boost::panel_text(&daily).contains("Reset epoch: 42"));
            let forge = link.item_forge_panel();
            assert!(!forge.pending);
            assert_eq!(forge.response, 152_u32.to_be_bytes());
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "CL25J session-loop test thread panicked")?
}

#[test]
fn daily_boost_shell_input_queries_selected_session() {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut commands, _) = crate::play::play_channel();
    let event = NormalizedInputEvent::Key {
        code: KeyCode::KEY_B,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    assert!(link.daily_boost_input(std::slice::from_ref(&event)));
    assert!(commands.try_recv().is_err());
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.daily_boost_initialize(true);
    assert!(link.daily_boost_input(std::slice::from_ref(&event)));
    assert!(
        matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::Feature(c)) if c.capability == 151 && c.kind == 177 && c.payload.is_empty())
    );
    assert!(link.daily_boost_input(&[event]));
    assert!(commands.try_recv().is_err());
    let mut payload = 42_u64.to_be_bytes().to_vec();
    payload.extend([3, b'r', b'a', b't', 4, b'b', b'o', b's', b's']);
    outbox.daily_boost_result(&oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    });
    assert_eq!(
        oteryn_client_ui::daily_boost::panel_text(&link.daily_boost_panel()),
        "Daily spotlight\nReset epoch: 42\nBoosted creature: Rat\nrat\nBoosted boss: Boss\nboss\n"
    );
}
