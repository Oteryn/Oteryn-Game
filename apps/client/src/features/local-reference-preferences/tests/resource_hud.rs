#[test]
fn t011_server_resources_reach_configurable_hud_without_opening_windows()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::play::{Stepper, run_session};
    use oteryn_session::{GatedCommandResult, SessionError};
    struct Resources;
    impl Stepper for Resources {
        fn actor_vitals(&self) -> Option<oteryn_session::ActorVitals> {
            Some(oteryn_session::ActorVitals {
                health: 75,
                max_health: 100,
                mana: 30,
                max_mana: 50,
                soul: 22,
                ..Default::default()
            })
        }
        fn feature_selected(&self, capability: u32) -> bool {
            matches!(capability, 199 | 276 | 277)
        }
        async fn feature(
            &mut self,
            command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        ) -> Result<GatedCommandResult, SessionError> {
            let payload = match command.capability {
                199 => {
                    assert_eq!(command.kind, 388);
                    let request =
                        oteryn_session::cyclopedia_character::decode_tab_request(&command.payload)
                            .map_err(|_| SessionError::SessionUnusable)?;
                    assert_eq!(request.target, None);
                    oteryn_session::cyclopedia_character::TabView::GeneralStats(
                        oteryn_session::cyclopedia_character::GeneralStats {
                            stamina_minutes: 2460,
                            ..Default::default()
                        },
                    )
                    .encode()
                    .map_err(|_| SessionError::SessionUnusable)?
                }
                276 => oteryn_session::condition_status::ConditionStatus {
                    conditions: vec![oteryn_session::condition_status::ActiveCondition {
                        icon: 1,
                        sub_id: 0,
                        remaining_ms: 5000,
                    }],
                }
                .encode()
                .ok_or(SessionError::SessionUnusable)?,
                277 => oteryn_session::rest_status::RestStatus {
                    inside_resting_area: true,
                    streak: 7,
                    food_remaining_ms: 60000,
                    health_multiplier: 2,
                    mana_multiplier: 2,
                    stamina: true,
                    soul: true,
                }
                .encode()
                .ok_or(SessionError::SessionUnusable)?,
                _ => return Err(SessionError::SessionUnusable),
            };
            Ok(GatedCommandResult {
                command_id: 1,
                status: oteryn_session::CommandStatus::Accepted,
                server_sequence: 1,
                payload,
            })
        }
        async fn serve(&mut self, duration: std::time::Duration) -> Result<(), SessionError> {
            tokio::time::sleep(duration).await;
            Ok(())
        }
        fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
            vec![]
        }
        async fn step(
            &mut self,
            _: oteryn_session::StepDirection,
        ) -> Result<oteryn_session::StepOutcome, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn combat(
            &mut self,
            _: crate::combat_input::CombatCommand,
        ) -> Result<crate::combat_input::CombatReport, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn logout(&mut self) -> Result<Option<oteryn_session::LogoutResult>, SessionError> {
            Ok(Some(oteryn_session::LogoutResult::Accepted))
        }
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                let (link, commands, outbox) = crate::play::play_channel();
                outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
                    oteryn_session::AppliedDelta {
                        server_sequence: 1,
                        base_revision: 0,
                        new_revision: 1,
                        value: oteryn_session::ActorVitals {
                            health: 75,
                            max_health: 100,
                            mana: 30,
                            max_mana: 50,
                            soul: 22,
                            ..Default::default()
                        },
                    },
                )]);
                let task = tokio::spawn(run_session(Resources, commands, outbox));
                let mut play = view()?;
                play.enable_resource_hud(true);
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    loop {
                        play.tick(&link).map_err(|_| "session ended")?;
                        let status = play.resource_status();
                        if status.contains("41:00")
                            && status.contains("Poison")
                            && status.contains("Rest active")
                        {
                            break Ok::<_, Box<dyn std::error::Error + Send + Sync>>(());
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await??;
                assert!(play.resource_status().contains("Soul: 22"));
                assert!(!link.condition_status_panel().open);
                assert!(!link.rest_status_panel().open);
                assert!(!link.cyclopedia_character_panel().open);
                let mut preferences = play.preferences().clone();
                preferences.resource_hud.conditions = false;
                preferences.resource_hud.soul = false;
                play.set_preferences(preferences)?;
                assert!(!play.resource_status().contains("Poison"));
                assert!(!play.resource_status().contains("Soul:"));
                drop(link);
                task.await?;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "HUD test thread failed")?
}
