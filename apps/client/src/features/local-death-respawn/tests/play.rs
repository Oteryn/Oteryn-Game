use crate::input::StepDir;
use crate::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{ActorVitals, AppliedDelta, SessionEvent};

#[test]
fn death_blocks_input_and_respawn_does_not_replay_pending_actions() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    let event = |health| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })
    };
    view.arrow(StepDir::North);
    outbox.push(vec![event(0)]);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.hud_status().contains("You are dead"));
    link.request_combat(crate::combat_input::CombatCommand::Attack(
        oteryn_session::EntityRef {
            identity: [7; 16],
            generation: 1,
        },
    ));
    link.request(StepDir::West);
    view.arrow(StepDir::East);
    view.cast(std::num::NonZeroU32::new(1).ok_or("spell")?);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert!(outbox.death_command(&mut commands).is_err());
    outbox.push(vec![event(150)]);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(!view.hud_status().contains("You are dead"));
    assert!(commands.try_recv().is_err());
    view.arrow(StepDir::South);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_ok());
    Ok(())
}

#[test]
fn coalesced_death_resets_targets_and_both_input_queues() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    link.request(StepDir::North);
    view.arrow(StepDir::West);
    view.in_flight = true;
    let target = crate::input::Targetable {
        tile: TileCoord::new(2, 1),
        kind: crate::input::TargetKind::Entity,
        glyph: true,
        entity: Some(oteryn_session::EntityRef {
            identity: [7; 16],
            generation: 1,
        }),
        item: None,
    };
    view.scene = crate::scene::Scene::centered_with(
        std::sync::Arc::new(crate::map::World::builtin().map_err(|e| e.to_string())?),
        None,
        TileCoord::new(1, 1),
        StepDir::South,
        &[],
        &[target],
    )
    .map_err(|e| e.to_string())?;
    view.scene
        .select_tile(target.tile)
        .map_err(|e| e.to_string())?;
    assert!(view.scene.target().is_some());
    for health in [0, 150] {
        outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })]);
    }
    assert!(outbox.death_command(&mut commands).is_err());
    assert_eq!(view.tick(&link), Ok(()));
    assert!(!view.death.dead);
    assert!(!view.in_flight);
    assert!(view.scene.target().is_none());
    assert!(view.inputs.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn default_vitals_are_not_death_and_death_clears_target_walk() {
    let (_, _, outbox) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&outbox.0);
    mailbox.apply_death_vitals(ActorVitals::default());
    assert!(!mailbox.death.dead);
    mailbox.target_walk.pending = true;
    mailbox.target_walk.feedback = Some("old target");
    mailbox.apply_death_vitals(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    });
    assert!(mailbox.death.dead);
    assert!(!mailbox.target_walk.pending);
    assert!(mailbox.target_walk.feedback.is_none());
}

#[test]
fn queued_world_actions_are_discarded_while_logout_remains_available() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    link.request_combat(crate::combat_input::CombatCommand::Cast(
        std::num::NonZeroU32::new(1).ok_or("spell")?,
    ));
    crate::play::mailbox::lock(&outbox.0).daily_reward.selected = true;
    link.daily_reward_request(oteryn_client_ui::daily_reward::Intent::Refresh);
    assert!(crate::play::mailbox::lock(&outbox.0).daily_reward.pending);
    link.request_logout();
    link.request(StepDir::North);
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    assert!(!crate::play::mailbox::lock(&outbox.0).daily_reward.pending);
    link.daily_reward_request(oteryn_client_ui::daily_reward::Intent::Refresh);
    assert!(!crate::play::mailbox::lock(&outbox.0).daily_reward.pending);
    assert_eq!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Logout)
    );
    assert!(outbox.death_command(&mut commands).is_err());
    Ok(())
}

#[test]
fn coalesced_respawn_before_reset_drain_cannot_leave_movement_pending() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    outbox.target_walk_initialize(true);
    for health in [0, 150] {
        outbox.vitals(Some(ActorVitals {
            health,
            max_health: 150,
            ..Default::default()
        }));
    }
    assert_eq!(view.tick(&link), Ok(()));
    view.arrow(StepDir::East);
    assert_eq!(view.tick(&link), Ok(()));
    // The view runs before the session loop drains its reset barrier.
    assert!(!view.in_flight);
    assert!(commands.try_recv().is_err());
    assert!(link.request_target_walk(oteryn_session::ActorPosition {
        x: 2,
        y: 1,
        floor: 0
    }));
    assert!(!crate::play::mailbox::lock(&outbox.0).target_walk.pending);
    assert!(outbox.death_command(&mut commands).is_err());
    assert_eq!(view.tick(&link), Ok(()));
    view.arrow(StepDir::East);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Step(StepDir::East))
    );
    outbox.stepped(crate::play::tests::moved_to(2, 1, 0));
    assert_eq!(view.tick(&link), Ok(()));
    assert!(!view.in_flight);
    assert!(link.request_target_walk(oteryn_session::ActorPosition {
        x: 3,
        y: 1,
        floor: 0
    }));
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::TargetWalk(_))
    ));
    Ok(())
}

#[test]
fn death_notice_preserves_logout_feedback_and_server_warning() -> Result<(), String> {
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    view.logout_line = Some("Logout refused".into());
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: oteryn_session::ServerMessage::Warning("Server warning".into()),
    })]);
    assert_eq!(view.tick(&link), Ok(()));
    let status = view.hud_status();
    assert!(status.contains("You are dead"));
    assert!(status.contains("Logout refused"));
    assert!(status.contains("Server warning"));
    Ok(())
}

#[test]
fn death_protection_is_requested_at_login_and_after_respawn_and_shown_in_hud() -> Result<(), String>
{
    use oteryn_session::death_protection::{DeathProtection, DeathProtectionReply};
    let (link, mut commands, outbox) = play_channel();
    outbox.death_protection_initialize(true);
    let command = outbox
        .death_command(&mut commands)
        .map_err(|e| e.to_string())?;
    assert!(
        matches!(command, crate::combat_input::PlayCommand::Feature(ref value) if value.capability == 382 && value.kind == 664)
    );
    let value = DeathProtectionReply(Some(DeathProtection {
        occurrence: [7; 16],
        consumed: vec!["oteryn:blessing.twist_of_fate".into()],
    }));
    outbox.death_protection_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: value.encode().ok_or("encode")?,
    });
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(view.hud_status().contains("Twist Of Fate"));
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .death
            .protection
            .as_ref()
            .ok_or("death receipt")?
            .occurrence,
        [7; 16]
    );
    for health in [0, 150] {
        outbox.vitals(Some(ActorVitals {
            health,
            max_health: 150,
            ..Default::default()
        }));
    }
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Feature(_))
    ));
    outbox.death_protection_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![0, 1],
    });
    assert!(outbox.death_command(&mut commands).is_err());
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(
        view.hud_status().contains("Twist Of Fate"),
        "malformed reply must not invent consumption"
    );
    Ok(())
}

#[test]
fn coalesced_alive_vitals_refreshes_death_receipt_without_query_feedback() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    outbox.death_protection_initialize(true);
    assert!(outbox.death_command(&mut commands).is_ok());
    let result = |payload| oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload,
    };
    outbox.death_protection_result(&result(vec![0]));
    let alive = ActorVitals {
        health: 150,
        max_health: 150,
        ..Default::default()
    };
    // Publishing the cached current vitals after a command never causes feedback.
    outbox.vitals(Some(alive));
    assert!(outbox.death_command(&mut commands).is_err());
    // The wire may coalesce dead and respawn revisions into one alive delta.
    outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 3,
        value: alive,
    })]);
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Feature(_))
    ));
    assert!(
        outbox.death_command(&mut commands).is_err(),
        "single query in flight"
    );
    let value = oteryn_session::death_protection::DeathProtectionReply(Some(
        oteryn_session::death_protection::DeathProtection {
            occurrence: [8; 16],
            consumed: vec!["oteryn:blessing.spiritual_shielding".into()],
        },
    ));
    outbox.death_protection_result(&result(value.encode().ok_or("encode")?));
    outbox.vitals(Some(alive));
    assert!(outbox.death_command(&mut commands).is_err());
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(view.hud_status().contains("Spiritual Shielding"));
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .death
            .protection
            .as_ref()
            .ok_or("death receipt")?
            .occurrence,
        [8; 16]
    );
    Ok(())
}

#[test]
fn srv7_dead_house_uses_real_exit_intent_before_native_temple_resume()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::combat_input::{CombatCommand, CombatReport};
            use crate::play::{PlayView, Stepper, play_channel, run_session};
            use oteryn_client_runtime::{house_transfer as wire, raw_feature::FeatureCommand};
            use oteryn_session::{
                ActorVitals, CommandStatus, GatedCommandResult, LogoutResult, SessionError,
                SessionEvent, StepDirection, StepOutcome,
            };
            use std::sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            };
            struct DeadHouse {
                scope: wire::HouseTransferAck,
                scope_payload: Vec<u8>,
                done: Arc<AtomicBool>,
            }
            impl Stepper for DeadHouse {
                fn feature_selected(&self, capability: u32) -> bool {
                    capability == 86
                }
                fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
                    (domain == wire::STATE_DOMAIN_HOUSE_SCOPE && !self.done.load(Ordering::SeqCst))
                        .then(|| self.scope_payload.clone())
                }
                fn own_house_vitals_current(&self) -> bool {
                    true
                }
                fn actor_vitals(&self) -> Option<ActorVitals> {
                    Some(ActorVitals {
                        health: if self.done.load(Ordering::SeqCst) {
                            150
                        } else {
                            0
                        },
                        max_health: 150,
                        ..Default::default()
                    })
                }
                async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn serve(
                    &mut self,
                    duration: std::time::Duration,
                ) -> Result<(), SessionError> {
                    tokio::time::sleep(duration).await;
                    Ok(())
                }
                fn drain(&mut self) -> Vec<SessionEvent> {
                    Vec::new()
                }
                async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
                    Err(SessionError::SessionUnusable)
                }
                async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
                    Ok(None)
                }
                async fn feature(
                    &mut self,
                    command: &FeatureCommand,
                ) -> Result<GatedCommandResult, SessionError> {
                    assert_eq!((command.capability, command.kind), (86, 102));
                    let intent = wire::decode_house_transfer_intent(&command.payload)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    assert!(intent.exit);
                    assert_eq!(Some(intent.house.as_str()), self.scope.house.as_deref());
                    let mut temple = self.scope.clone();
                    temple.house = None;
                    temple.position = [32369, 32241, 7];
                    temple.connection_generation += 1;
                    temple.lease_generation += 1;
                    temple.session[15] += 1;
                    let payload = wire::encode_house_transfer_ack(&temple)
                        .map_err(|_| SessionError::SessionUnusable)?;
                    self.done.store(true, Ordering::SeqCst);
                    Ok(GatedCommandResult {
                        command_id: 1,
                        server_sequence: 1,
                        status: CommandStatus::Accepted,
                        payload,
                    })
                }
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let mut id = [0; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id[15] = 1;
                let scope = wire::HouseTransferAck {
                    transfer: id,
                    session: id,
                    world: id,
                    channel: id,
                    house: Some("oteryn:content.house.test".into()),
                    scope_generation: 2,
                    connection_generation: 1,
                    lease_generation: 1,
                    selection: vec![86],
                    position: [100, 200, 7],
                };
                let scope_payload = wire::encode_house_transfer_ack(&scope)
                    .map_err(|e| format!("House snapshot fixture: {e:?}"))?;
                let done = Arc::new(AtomicBool::new(false));
                let (link, commands, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    DeadHouse {
                        scope,
                        scope_payload,
                        done: done.clone(),
                    },
                    commands,
                    outbox,
                ));
                let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    loop {
                        // Editable fields are never authority for the internal death exit.
                        link.house_transfer_edit(0, "oteryn:content.house.forged".into());
                        assert!(view.tick(&link).is_ok());
                        if done.load(Ordering::SeqCst) && !view.death.dead {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .map_err(|_| "native dead House never issued the existing exit/Temple handoff")?;
                assert_eq!(link.house_transfer_panel().feedback, "Transfer committed");
                assert!(
                    link.house_transfer_panel()
                        .summary
                        .contains("32369,32241,7")
                );
                assert!(!view.hud_status().contains("You are dead"));
                task.abort();
                let _ = task.await;
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            })
        })?
        .join()
        .map_err(|_| "native House death thread panicked")?
}

#[test]
fn srv7_house_death_exit_is_once_and_discards_revived_or_changed_scope()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::house_transfer as wire;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 1;
    let mut ack = wire::HouseTransferAck {
        transfer: id,
        session: id,
        world: id,
        channel: id,
        house: Some("oteryn:content.house.test".into()),
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![86],
        position: [100, 200, 7],
    };
    let payload =
        wire::encode_house_transfer_ack(&ack).map_err(|e| format!("House binding: {e:?}"))?;
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    outbox.house_death_scope(false, Some(payload.clone()));
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    outbox.house_death_vitals_current(true);
    assert!(view.tick(&link).is_ok());
    assert!(
        commands.try_recv().is_err(),
        "an unselected House route never becomes a death exit"
    );
    crate::play::mailbox::lock(&outbox.0)
        .properties
        .house_transfer
        .selected = true;
    outbox.house_death_scope(true, Some(payload));
    outbox.house_death_vitals_current(true);
    crate::play::mailbox::lock(&outbox.0).world_context.pending = true;
    assert!(view.tick(&link).is_ok());
    assert!(
        commands.try_recv().is_err(),
        "the current USE must complete first"
    );
    crate::play::mailbox::lock(&outbox.0).world_context.pending = false;
    assert!(view.tick(&link).is_ok());
    let PlayCommand::HouseDeathExit {
        house_session,
        connection_generation,
        scope_generation,
        lease_generation,
        house,
        death_generation,
        ..
    } = commands.try_recv()?
    else {
        return Err("bound House death exit".into());
    };
    assert!(view.tick(&link).is_ok());
    assert!(commands.try_recv().is_err(), "one request per death");
    outbox.vitals(Some(ActorVitals {
        health: 150,
        max_health: 150,
        ..Default::default()
    }));
    outbox.house_death_vitals_current(true);
    assert!(view.tick(&link).is_ok());
    assert!(
        !outbox.house_death_exit_current(
            house_session,
            connection_generation,
            scope_generation,
            lease_generation,
            &house,
            death_generation
        ),
        "a revived actor cannot send the obsolete exit"
    );
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    outbox.house_death_vitals_current(true);
    assert!(view.tick(&link).is_ok());
    let PlayCommand::HouseDeathExit {
        house_session,
        connection_generation,
        scope_generation,
        lease_generation,
        house,
        death_generation,
        ..
    } = commands.try_recv()?
    else {
        return Err("new observed House death".into());
    };
    ack.session[15] += 1;
    ack.connection_generation += 1;
    outbox.house_death_scope(
        true,
        Some(
            wire::encode_house_transfer_ack(&ack)
                .map_err(|e| format!("new House binding: {e:?}"))?,
        ),
    );
    assert!(
        !outbox.house_death_exit_current(
            house_session,
            connection_generation,
            scope_generation,
            lease_generation,
            &house,
            death_generation
        ),
        "a new server House session invalidates the older exit"
    );
    assert!(
        !crate::play::mailbox::lock(&outbox.0)
            .properties
            .house_transfer
            .pending
    );
    Ok(())
}

#[path = "house_retry.rs"]
mod house_retry;
#[test]
fn srv7_house_death_exit_cannot_rebind_cached_zero_health_to_new_lease()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::house_transfer as wire;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 1;
    let mut ack = wire::HouseTransferAck {
        transfer: id,
        session: id,
        world: id,
        channel: id,
        house: Some("oteryn:content.house.test".into()),
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![86],
        position: [100, 200, 7],
    };
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    crate::play::mailbox::lock(&outbox.0)
        .properties
        .house_transfer
        .selected = true;
    outbox.house_death_scope(
        true,
        Some(wire::encode_house_transfer_ack(&ack).map_err(|e| format!("ack: {e:?}"))?),
    );
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    outbox.house_death_vitals_current(true);
    view.tick(&link)
        .map_err(|error| format!("native tick: {error:?}"))?;
    let _old = commands.try_recv()?;
    ack.session[15] += 1;
    ack.connection_generation += 1;
    ack.scope_generation += 1;
    ack.lease_generation += 1;
    outbox.house_death_scope(
        true,
        Some(wire::encode_house_transfer_ack(&ack).map_err(|e| format!("new ack: {e:?}"))?),
    );
    // A cached actor_vitals() poll does not prove this newer House lease is dead.
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    view.tick(&link)
        .map_err(|error| format!("native tick: {error:?}"))?;
    assert!(
        commands.try_recv().is_err(),
        "old cached zero health must await actual own-vitals in the new House binding"
    );
    Ok(())
}

#[test]
fn srv7_house_death_exit_rejects_superseded_scope_and_lease_generations()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::house_transfer as wire;
    for lease in [false, true] {
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 1;
        let mut ack = wire::HouseTransferAck {
            transfer: id,
            session: id,
            world: id,
            channel: id,
            house: Some("oteryn:content.house.test".into()),
            scope_generation: 2,
            connection_generation: 1,
            lease_generation: 1,
            selection: vec![86],
            position: [100, 200, 7],
        };
        let (link, mut commands, outbox) = play_channel();
        let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
        crate::play::mailbox::lock(&outbox.0)
            .properties
            .house_transfer
            .selected = true;
        outbox.house_death_scope(
            true,
            Some(wire::encode_house_transfer_ack(&ack).map_err(|e| format!("ack: {e:?}"))?),
        );
        outbox.vitals(Some(ActorVitals {
            health: 0,
            max_health: 150,
            ..Default::default()
        }));
        outbox.house_death_vitals_current(true);
        view.tick(&link)
            .map_err(|error| format!("native tick: {error:?}"))?;
        let PlayCommand::HouseDeathExit {
            house_session,
            connection_generation,
            scope_generation,
            lease_generation,
            house,
            death_generation,
            ..
        } = commands.try_recv()?
        else {
            return Err("expected automatic House exit".into());
        };
        if lease {
            ack.lease_generation += 1;
        } else {
            ack.scope_generation += 1;
        }
        outbox.house_death_scope(
            true,
            Some(
                wire::encode_house_transfer_ack(&ack)
                    .map_err(|e| format!("superseding ack: {e:?}"))?,
            ),
        );
        assert!(
            !outbox.house_death_exit_current(
                house_session,
                connection_generation,
                scope_generation,
                lease_generation,
                &house,
                death_generation
            ),
            "scope/lease generation must cancel old queued exit (lease={lease})"
        );
    }
    Ok(())
}
