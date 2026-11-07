#[test]
fn hireling_management_crosses_the_real_session_loop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::play::{Stepper, play_channel, run_session};
            use oteryn_client_runtime::raw_feature::FeatureCommand;
            use oteryn_session::{GatedCommandResult, SessionError};
            fn identity() -> [u8; 16] {
                let mut id = [1; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id
            }
            struct Features;
            fn result(capability: u32) -> Result<GatedCommandResult, SessionError> {
                Ok(GatedCommandResult {
                    command_id: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: if capability == 250 {
                        oteryn_client_ui::hireling_management::wire::Reply::State(vec![
                            oteryn_client_ui::hireling_management::wire::Entry {
                                hireling: identity(), name: "Alice".into(), sex: 2,
                                look_type: 1107, colors: [1, 2, 3, 4],
                                skills: 15, outfits: 1, position: Some((7, 9, -1)),
                            }
                        ]).encode().ok_or(SessionError::SessionUnusable)?
                    } else {
                        vec![255]
                    },
                })
            }
            impl Stepper for Features {
                async fn step(
                    &mut self,
                    _: oteryn_session::StepDirection,
                ) -> Result<oteryn_session::StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(&mut self, duration: std::time::Duration) -> Result<(), SessionError> {
                    tokio::time::sleep(duration).await;
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
                    capability == 250
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    use oteryn_client_ui::hireling_management::wire;
                    if let Some(wire::Request::Outfit { hireling, look_type, colors }) =
                        wire::Request::decode(&command.payload)
                    {
                        assert_eq!(hireling, identity());
                        assert_eq!(look_type, 1109);
                        assert_eq!(colors, [5, 6, 7, 8]);
                        let mut reply = result(command.capability)?;
                        reply.payload = vec![1];
                        return Ok(reply);
                    }
                    assert_eq!(wire::Request::decode(&command.payload), Some(wire::Request::List));
                    result(command.capability)
                }
            }
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    use oteryn_client_ui::hireling_management::{Intent, select};
                    let (link, commands, outbox) = play_channel();
                    outbox.hireling_management_initialize(true);
                    // The ordinary Ctrl+Shift+H entry point must choose owned identity
                    // management when capability 250 is admitted, before outfit submission.
                    use oteryn_input_actions::{
                        ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
                    };
                    assert!(link.hireling_input(&[NormalizedInputEvent::Key {
                        code: KeyCode::new(11)?,
                        state: ButtonState::Pressed,
                        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                        repeat: false,
                    }]));
                    assert!(link.hireling_management_panel().open);
                    assert!(!link.hireling_panel().open);
                    assert_eq!(
                        link.hireling_management_panel().pending,
                        Some(oteryn_client_ui::hireling_management::wire::Request::List),
                    );
                    let task = tokio::spawn(run_session(Features, commands, outbox));
                    async fn ready(link: &crate::play::PlayLink) -> Result<(), tokio::time::error::Elapsed> {
                        tokio::time::timeout(std::time::Duration::from_secs(3), async {
                            while link.hireling_management_panel().pending.is_some() {
                                tokio::task::yield_now().await;
                            }
                        }).await
                    }
                    ready(&link).await?;
                    let panel = link.hireling_management_panel();
                    assert_eq!(panel.entries.len(), 1);
                    assert_eq!(panel.entries[0].name, "Alice");
                    assert_eq!(panel.entries[0].colors, [1, 2, 3, 4]);
                    let text = oteryn_client_ui::hireling_management::panel_text(&panel);
                    assert!(
                        text.contains("Alice") && text.contains("Hireling identity and outfits")
                    );
                    assert!(select(&mut crate::play::mailbox::lock(&link.mailbox).hireling_management, 0));
                    link.hireling_management_request(Intent::Outfit { look_type: 1109, colors: [5, 6, 7, 8] });
                    ready(&link).await?;
                    let panel = link.hireling_management_panel();
                    assert_eq!(panel.entries[0].look_type, 1109);
                    assert_eq!(panel.entries[0].colors, [5, 6, 7, 8]);
                    assert_eq!(panel.feedback, Some("Outfit saved; refresh to read current server state"));
                    drop(link);
                    task.await?;
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        })?
        .join()
        .map_err(|_| "E12 session-loop test thread panicked")?
}
