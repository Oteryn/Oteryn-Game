#[test]
fn t175_t176_confirmed_conditions_reach_independent_hud_and_status_displays()
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
            matches!(capability, 117 | 199 | 276 | 277)
        }
        async fn feature(
            &mut self,
            command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        ) -> Result<GatedCommandResult, SessionError> {
            let payload = match command.capability {
                117 => {
                    use oteryn_session::property_services::boss_encounter::{
                        BossTaints, append_boss_taints, encode_cooldown_list,
                    };
                    assert_eq!(command.payload, vec![16, 1]);
                    let mut payload = encode_cooldown_list(1, &[], false)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    append_boss_taints(
                        &mut payload,
                        BossTaints {
                            soul_war: 3,
                            rotten_blood: 2,
                            rotten_blood_flags: 3,
                            rotten_blood_final: true,
                        },
                    )
                    .map_err(|_| SessionError::SessionUnusable)?;
                    payload
                }
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
                    conditions: (1..=8)
                        .map(|icon| oteryn_session::condition_status::ActiveCondition {
                            icon,
                            sub_id: 0,
                            remaining_ms: 5000,
                        })
                        .collect(),
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
                            && status.contains("Soul War: 3")
                        {
                            break Ok::<_, Box<dyn std::error::Error + Send + Sync>>(());
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await??;
                let layout = play
                    .hud_arc_layout()
                    .ok_or("condition HUD missing with arcs off")?;
                assert!(layout.strokes.is_empty());
                assert_eq!(layout.alpha, (85_u16 * 255 / 100) as u8);
                assert!(
                    play.resource_status()
                        .contains("Rotten Blood: 2 taints + final")
                );
                assert!(!link.boss_encounter_panel().open);
                assert_eq!(layout.condition_icons.len(), 7);
                assert_eq!(layout.condition_icons[0], "☠");
                assert_eq!(layout.condition_icons[6], "…");
                let mut preferences = play.preferences().clone();
                preferences.arcs.health = true;
                preferences.arcs.mana = true;
                preferences.arcs.opacity = 42;
                preferences.special_condition_order = preferences.ordered_condition_keys();
                preferences
                    .special_condition_order
                    .retain(|key| key != "Soul War");
                preferences
                    .special_condition_order
                    .insert(0, "Soul War".into());
                preferences.condition_status_hidden = vec!["Soul War".into()];
                play.set_preferences(preferences.clone())?;
                let with_arcs = play.hud_arc_layout().ok_or("arcs and conditions")?;
                assert_eq!(with_arcs.strokes.len(), 2);
                assert_eq!(with_arcs.alpha, (42_u16 * 255 / 100) as u8);
                assert_eq!(with_arcs.condition_icons[0], "S3");
                assert!(!play.resource_status().contains("Soul War:"));
                preferences.arcs.health = false;
                preferences.arcs.mana = false;
                preferences.special_condition_order.clear();
                preferences.condition_hud_hidden = vec![
                    "Poison".into(),
                    "Burning".into(),
                    "Resting Area".into(),
                    "Soul War".into(),
                    "Rotten Blood".into(),
                ];
                preferences.condition_status_hidden = vec!["Energy".into()];
                preferences.condition_order = (1..=27).rev().collect();
                play.set_preferences(preferences)?;
                let layout = play.hud_arc_layout().ok_or("filtered condition HUD")?;
                assert_eq!(layout.condition_icons.len(), 6);
                assert_eq!(layout.condition_icons[0], "☾");
                assert!(!play.resource_status().contains("Energy:"));
                assert!(play.resource_status().contains("Poison:"));
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
                assert!(play.hud_arc_layout().is_none());
                drop(link);
                task.await?;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "HUD test thread failed")?
}

#[test]
fn t011_server_resources_reach_configurable_hud_without_opening_windows()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server_resources_reach_hud(false)
}

#[test]
fn stamina_hud_with_vocation_extension_reaches_tick()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server_resources_reach_hud(true)
}

fn server_resources_reach_hud(
    vocation: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    use crate::play::{Stepper, run_session};
    use oteryn_session::{GatedCommandResult, SessionError};
    struct Resources(bool);
    impl Stepper for Resources {
        fn feature_selected(&self, capability: u32) -> bool {
            matches!(capability, 199 | 276 | 277) || (capability == 487 && self.0)
        }
        async fn feature(
            &mut self,
            command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        ) -> Result<GatedCommandResult, SessionError> {
            let payload = match command.capability {
                199 => oteryn_session::cyclopedia_character::TabView::GeneralStats(
                    oteryn_session::cyclopedia_character::GeneralStats {
                        stamina_minutes: 2460,
                        ..Default::default()
                    },
                )
                .encode_extended(&oteryn_session::cyclopedia_character::TabExtension {
                    vocation_id: self.0.then_some(3),
                    deaths: None,
                })
                .map_err(|_| SessionError::SessionUnusable)?,
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
        .spawn(move || -> Result<(), Error> {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                let (link, commands, outbox) = crate::play::play_channel();
                let task = tokio::spawn(run_session(Resources(vocation), commands, outbox));
                let mut play = view()?;
                play.enable_resource_hud(true);
                // The HUD feed is driven by the real tick caller: it queries the conditions,
                // rest and stamina wires on its own and never opens their windows.
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    loop {
                        play.tick(&link).map_err(|_| "session ended")?;
                        let status = play.resource_status();
                        if status.contains("Stamina: 41:00")
                            && status.contains("Poison")
                            && status.contains("Rest active")
                        {
                            break Ok::<_, Box<dyn std::error::Error + Send + Sync>>(());
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await??;
                let status = play.resource_status();
                assert!(status.contains("Food: 60s"), "{status}");
                assert!(!link.condition_status_panel().open);
                assert!(!link.rest_status_panel().open);
                assert!(!link.cyclopedia_character_panel().open);
                let mut preferences = play.preferences().clone();
                preferences.resource_hud.stamina = false;
                play.set_preferences(preferences.clone())?;
                let status = play.resource_status();
                assert!(!status.contains("Stamina:"), "{status}");
                assert!(status.contains("Rest active") && status.contains("Poison"));
                preferences.resource_hud.rest = false;
                play.set_preferences(preferences.clone())?;
                let status = play.resource_status();
                assert!(!status.contains("Rest active"), "{status}");
                assert!(status.contains("Poison"), "{status}");
                preferences.resource_hud.conditions = false;
                play.set_preferences(preferences)?;
                assert!(!play.resource_status().contains("Poison"));
                drop(link);
                task.await?;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "HUD test thread failed")?
}
