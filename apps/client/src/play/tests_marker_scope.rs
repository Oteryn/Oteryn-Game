//! Automatic reads and queued feature commands must publish the destination barrier.
use super::*;
use oteryn_session::activity_scope::{ActivityScope, ActivityScopeSnapshot, encode_activity_scope};

#[path = "tests_marker_scope/connection_handoff.rs"]
mod connection_handoff;
#[path = "tests_marker_scope/guild_projection.rs"]
mod guild_projection;
#[path = "tests_marker_scope/presentation_gate.rs"]
mod presentation_gate;
#[path = "tests_marker_scope/wire_handoff.rs"]
mod wire_handoff;

struct AutomaticQueryHandoff {
    scope: ActivityScopeSnapshot,
    changed: bool,
    activity: bool,
    capability: u32,
    spell_info_queried: bool,
    pending: Vec<SessionEvent>,
    serves: Arc<AtomicUsize>,
}
impl Stepper for AutomaticQueryHandoff {
    fn daily_reward_selected(&self) -> bool {
        self.capability == 182
    }
    fn feature_selected(&self, capability: u32) -> bool {
        match capability {
            31 | 75 => true,
            287 | 500 if self.capability == 182 => !self.changed && !self.activity,
            450 if self.capability == 182 => !self.changed,
            287 => self.capability == 500,
            451 if self.changed => self.scope.selection.contains(&451),
            _ => capability == self.capability,
        }
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        if self.capability == 500 && (!self.spell_info_queried || self.changed) {
            self.spell_info_queried = true;
            use oteryn_session::server_info::{Reply, SpellInfo};
            let info = SpellInfo {
                cast_index: 1,
                description: Some(
                    if self.changed {
                        "Destination"
                    } else {
                        "Source"
                    }
                    .into(),
                ),
                cooldown_ms: 1000,
                group_cooldowns_ms: vec![(1, 2000)],
            };
            return Ok(oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: Reply::Spells(vec![info])
                    .encode()
                    .ok_or(SessionError::SessionUnusable)?,
            });
        }
        if command.capability == 451 && !self.feature_selected(451) {
            return Err(SessionError::CapabilityNotSelected { capability: 451 });
        }
        if self.changed && self.capability == 182 && command.capability == 182 {
            use oteryn_session::daily_reward::returner::Reply;
            return Ok(oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: Reply {
                    state: 1,
                    id: Some(self.scope.session),
                }
                .encode()
                .map_err(|_| SessionError::SessionUnusable)?,
            });
        }
        assert_eq!(
            command.capability, self.capability,
            "a later feature query or modal poll ran before the Returner scope barrier"
        );
        assert!(
            !self.changed,
            "scope transition must not replay its source query"
        );
        self.changed = true;
        if self.activity {
            if self.capability == 450 {
                let mut admission = self.scope.clone();
                admission.session = self.scope.source_session;
                self.pending.push(SessionEvent::ActivityScope(AppliedDelta {
                    server_sequence: 1,
                    base_revision: 0,
                    new_revision: 1,
                    value: admission,
                }));
            }
            self.pending.push(SessionEvent::ServerMessage(AppliedDelta {
                server_sequence: 1000,
                base_revision: 0,
                new_revision: 1,
                value: oteryn_session::ServerMessage::Warning("source".into()),
            }));
            self.pending.push(SessionEvent::Presentation(AppliedDelta {
                server_sequence: 1001,
                base_revision: 0,
                new_revision: 1,
                value: vec![
                    oteryn_session::PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: 1,
                        kind: oteryn_session::PresentationEventKind::MagicEffect {
                            position: oteryn_session::ActorPosition {
                                x: 32000,
                                y: 200,
                                floor: 0,
                            },
                            effect_id: 1,
                            source: oteryn_session::PresentationSource::Own,
                        },
                    },
                    oteryn_session::PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: 2,
                        kind: oteryn_session::PresentationEventKind::Sound {
                            cue_id: 1,
                            secondary_cue_id: 0,
                            position: oteryn_session::ActorPosition {
                                x: 32000,
                                y: 200,
                                floor: 0,
                            },
                            source: oteryn_session::PresentationSource::Own,
                        },
                    },
                ],
            }));
            self.pending.push(SessionEvent::ServerMessage(AppliedDelta {
                server_sequence: 0,
                base_revision: 1,
                new_revision: 1,
                value: oteryn_session::ServerMessage::StatusText("MOTD".into()),
            }));
            // The destination spatial domain can precede domain 99.
            self.pending.push(spatial_event(0, 32000));
            let payload = encode_activity_scope(&self.scope)
                .map_err(|_| SessionError::ActivityScopeChanged)?;
            self.pending
                .push(SessionEvent::GatedSnapshotReplaced(AppliedDelta {
                    server_sequence: 0,
                    base_revision: 0,
                    new_revision: 1,
                    value: oteryn_session::GatedSnapshot {
                        domain_id: 99,
                        snapshot_type: 1,
                        revision: 1,
                        payload,
                    },
                }));
        } else {
            self.pending.push(SessionEvent::ActivityScope(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: self.scope.clone(),
            }));
        }
        if !self.activity {
            self.pending.push(spatial_event(2, 32000));
        }
        self.pending.push(SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: 2,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ServerMessage::StatusText("destination".into()),
        }));
        Err(if self.activity {
            SessionError::ActivityScopeChanged
        } else {
            SessionError::ScopeChanged
        })
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        if domain == 100 && matches!(self.capability, 182 | 500) {
            return oteryn_session::spell_catalog::encode_catalog(&[
                oteryn_session::spell_catalog::SpellCatalogEntry {
                    cast_index: 1,
                    name: "Light Healing".into(),
                    words: "exura".into(),
                    level: 8,
                    mana: 20,
                    mana_percent: false,
                    soul: 0,
                    learned: true,
                    available: true,
                    cooldown_groups: vec![1],
                    rune_item: None,
                },
            ])
            .ok();
        }
        if self.changed && domain == 99 {
            encode_activity_scope(&self.scope).ok()
        } else {
            None
        }
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.pending)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.serves.fetch_add(1, Ordering::SeqCst);
        if self.capability == 500 && !self.changed {
            return self
                .feature(&oteryn_client_runtime::raw_feature::FeatureCommand {
                    capability: 500,
                    kind: 0,
                    payload: Vec::new(),
                })
                .await
                .map(|_| ());
        }
        tokio::time::sleep(duration).await;
        Ok(())
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Ok(CombatReport::Stop(AttackIntentDisposition::Ok))
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Ok(moved_to(32003, 200, 0))
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Some(LogoutResult::Busy))
    }
}

#[test]
fn review_scoped_admission_delivers_motd_before_domain_records() -> Result<(), String> {
    let (link, _, outbox) = play_channel();
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut source = id;
    source[15] = 2;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: source,
        session: id,
        scope: ActivityScope::Channel {
            world: id,
            channel: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![92, 117],
        requires_ack: false,
    })
    .map_err(|error| format!("{error:?}"))?;
    outbox.push(vec![
        SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: 0,
            base_revision: 1,
            new_revision: 1,
            value: oteryn_session::ServerMessage::StatusText("Welcome to the channel".into()),
        }),
        SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 0,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::GatedSnapshot {
                domain_id: 99,
                snapshot_type: 1,
                revision: 1,
                payload,
            },
        }),
    ]);
    let mut view = view().map_err(|error| error.to_string())?;
    view.tick(&link).map_err(|error| format!("{error:?}"))?;
    assert!(
        view.server_messages()
            .any(|message| matches!(&message.value,
        oteryn_session::ServerMessage::StatusText(text) if text == "Welcome to the channel"))
    );
    let console = link.chat_panel().console;
    assert!(
        console
            .notices
            .text(console.filters, console.display)
            .contains("Welcome to the channel")
    );

    Ok(())
}

fn automatic_query_handoff_continues(capability: u32, activity: bool) -> Result<(), String> {
    query_handoff_continues(capability, activity, None, false)
}

fn query_handoff_continues(
    capability: u32,
    activity: bool,
    queued: Option<crate::combat_input::PlayCommand>,
    dropped_marks: bool,
) -> Result<(), String> {
    let mut source = [1; 16];
    source[6] = 0x70;
    source[8] = 0x80;
    let mut destination = source;
    destination[15] = 2;
    let scope = ActivityScopeSnapshot {
        handoff: source,
        source_session: source,
        session: destination,
        scope: ActivityScope::Instance {
            world: source,
            instance: destination,
        },
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 2,
        selection: if dropped_marks {
            vec![31, 75, 117]
        } else {
            vec![31, 75, 117, capability]
        },
        requires_ack: false,
    };
    let serves = Arc::new(AtomicUsize::new(0));
    let (link, commands, outbox) = play_channel();
    if let Some(command) = queued {
        link.commands
            .send(command)
            .map_err(|error| error.to_string())?;
    }
    let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
    runtime
        .spawn(run_session(
            AutomaticQueryHandoff {
                scope,
                changed: false,
                activity,
                capability,
                spell_info_queried: false,
                pending: if activity {
                    vec![SessionEvent::ServerMessage(AppliedDelta {
                        server_sequence: 0,
                        base_revision: 1,
                        new_revision: 1,
                        value: oteryn_session::ServerMessage::StatusText("Source admission".into()),
                    })]
                } else {
                    Vec::new()
                },
                serves: Arc::clone(&serves),
            },
            commands,
            outbox,
        ))
        .map_err(|error| error.to_string())?;
    let mut view = view().map_err(|error| error.to_string())?;
    let namespace = view.message_namespace;
    let mut requested = false;
    let deadline =
        std::time::Instant::now() + Duration::from_secs(if dropped_marks { 8 } else { 1 });
    while std::time::Instant::now() < deadline {
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        if !requested && view.own().x == 32000 {
            assert_ne!(view.message_namespace, namespace);
            assert!(view.visuals.is_empty() && view.sounds.is_empty());
            view.arrow(StepDir::East);
            requested = true;
        }
        if view.own().x == 32003
            && serves.load(Ordering::SeqCst) > if dropped_marks { 270 } else { 1 }
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(requested);

    assert_eq!(
        view.own().x,
        32003,
        "destination step must reach the session"
    );
    assert_eq!(
        view.server_messages()
            .map(|m| m.server_sequence)
            .collect::<Vec<_>>(),
        if activity { vec![0, 2] } else { vec![2] }
    );
    let console = link.chat_panel().console;
    let notices = console.notices.text(console.filters, console.display);
    assert!(!notices.contains("Source admission"));
    if activity {
        assert!(notices.contains("MOTD"));
    }
    assert!(serves.load(Ordering::SeqCst) > 1);
    if dropped_marks {
        assert!(serves.load(Ordering::SeqCst) > 270);
    }
    if capability == 500 {
        let panel = link.spell_catalog_panel();
        assert_eq!(
            panel
                .info
                .get(&1)
                .and_then(|info| info.description.as_deref()),
            Some("Destination")
        );
        assert!(
            link.cooldown_bar_icons()
                .iter()
                .any(|icon| icon.group && icon.id == 1 && icon.tooltip.contains("2000"))
        );
    }
    if capability == 182 {
        assert_eq!(
            link.daily_reward_panel().returner_notified,
            Some(destination),
            "interrupted Returner query was not retried after presentation"
        );
    }
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn review_typed_scope_stays_connected_after_dropping_personal_marks() -> Result<(), String> {
    query_handoff_continues(451, false, None, true)
}

#[test]
fn review_scope_requeries_spell_metadata() -> Result<(), String> {
    automatic_query_handoff_continues(500, false)
}

#[test]
fn marker_refresh_activity_scope_changed_publishes_destination_and_continues() -> Result<(), String>
{
    automatic_query_handoff_continues(451, true)
}
#[test]
fn marker_refresh_scope_changed_publishes_destination_and_continues() -> Result<(), String> {
    automatic_query_handoff_continues(451, false)
}

#[test]
fn modal_refresh_activity_scope_changed_publishes_destination_and_continues() -> Result<(), String>
{
    automatic_query_handoff_continues(450, true)
}

#[test]
fn modal_refresh_scope_changed_publishes_destination_and_continues() -> Result<(), String> {
    automatic_query_handoff_continues(450, false)
}

#[test]
fn initial_returner_scope_changed_publishes_before_other_queries() -> Result<(), String> {
    automatic_query_handoff_continues(182, false)
}

#[test]
fn initial_returner_activity_scope_changed_publishes_before_modal_poll() -> Result<(), String> {
    automatic_query_handoff_continues(182, true)
}

fn support_delivery_handoff_continues(activity: bool) -> Result<(), String> {
    use oteryn_client_runtime::support_intake::{CAPABILITY_SUPPORT_INTAKE_V1, CRASH};
    let command = oteryn_client_ui::support_intake::submission(
        CRASH,
        "Reference: scope-transition test",
        true,
    )
    .ok_or("valid support submission")?;
    let fixture = tempfile::tempdir().map_err(|e| e.to_string())?;
    query_handoff_continues(
        CAPABILITY_SUPPORT_INTAKE_V1,
        activity,
        Some(crate::combat_input::PlayCommand::SupportDelivery(
            command,
            fixture.path().join("scope-transition-report.txt"),
        )),
        false,
    )
}

#[test]
fn support_delivery_scope_changed_publishes_destination_and_continues() -> Result<(), String> {
    support_delivery_handoff_continues(false)
}

#[test]
fn support_delivery_activity_scope_changed_publishes_destination_and_continues()
-> Result<(), String> {
    support_delivery_handoff_continues(true)
}

fn publish_activity_handoff(outbox: &Outbox) {
    let mut id = [1; 16];
    id[6] = 0x71;
    id[8] = 0x81;
    let mut destination = id;
    destination[15] = 2;
    let scope = ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session: destination,
        scope: ActivityScope::Channel {
            world: id,
            channel: id,
        },
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![31, 75, 117],
        requires_ack: false,
    };
    let pending = vec![
        SessionEvent::ActivityScope(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: scope.clone(),
        }),
        spatial_event(2, 32000),
    ];
    super::super::session_stepper::publish(
        &mut AutomaticQueryHandoff {
            scope,
            changed: true,
            activity: false,
            capability: 117,
            spell_info_queried: false,
            pending,
            serves: Arc::new(AtomicUsize::new(0)),
        },
        outbox,
        &mut Vec::new(),
    );
}

#[test]
fn typed_handoff_resets_presented_message_namespace() -> Result<(), String> {
    let (link, _, outbox) = play_channel();
    let mut view = super::PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 0, &[])
        .map_err(|error| error.to_string())?;
    let source_namespace = view.message_namespace;
    publish_activity_handoff(&outbox);
    view.tick(&link).map_err(|error| format!("{error:?}"))?;
    assert_ne!(source_namespace, view.message_namespace);
    Ok(())
}

#[test]
fn handoff_retains_shared_frontend_and_admitted_byte_counters() {
    let (link, _, outbox) = play_channel();
    link.use_shared_frontend();
    link.set_traffic(Arc::default());
    publish_activity_handoff(&outbox);
    assert_eq!(
        (link.uses_shared_frontend(), link.traffic_bytes()),
        (true, Some((0, 0)))
    );
}

#[test]
fn handoff_preserves_saved_chat_mode_on_settings_tick() {
    use crate::play::feature_local_reference_preferences::settings::ChatMode;
    for mode in [ChatMode::Off, ChatMode::Auto] {
        let (link, _, outbox) = play_channel();
        let mut settings = super::SettingsState::new(None);
        settings.preferences.chat_mode = Some(mode);
        settings.sync_chat_context(&link);
        publish_activity_handoff(&outbox);
        settings.sync_chat_context(&link);
        assert_eq!(settings.preferences.chat_mode, Some(mode));
    }
}

#[cfg(bevy_shell)]
#[test]
fn destination_look_uses_a_fresh_receipt() -> Result<(), &'static str> {
    use super::{feature_local_text_messages::view::record_look, mailbox::lock};
    use oteryn_client_runtime::world_interaction::Action;
    let (link, _, outbox) = play_channel();
    record_look(&mut lock(&outbox.0), Action::Look, "Source");
    let source = link.look_message().ok_or("source look")?;
    publish_activity_handoff(&outbox);
    assert!(link.look_message().is_none());
    record_look(&mut lock(&outbox.0), Action::Look, "Destination");
    let destination = link.look_message().ok_or("destination look")?;
    assert!(destination.receipt > source.receipt);
    assert_eq!(destination.text, "Destination");
    Ok(())
}

#[test]
fn review_scope_preserves_admission_identity() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    link.party_own_character("Ari");
    link.house_world("01010101-0101-7101-8101-010101010101");
    crate::play::mailbox::lock(&outbox.0)
        .properties
        .house
        .model
        .selected = true;
    link.house_edit(0, String::new());
    link.house_edit(1, "oteryn:content.area.city.thais".into());
    link.house_request(0);
    assert!(commands.try_recv().is_err());
    publish_activity_handoff(&outbox);
    assert_eq!(link.party_panel().own_character_name, "Ari");
    assert!(link.notices_panel().tutorial_selected && link.notices_panel().save_selected);
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .properties
            .house
            .fields[0],
        "01010101-0101-7101-8101-010101010101"
    );
    view()?.tick(&link).map_err(|error| format!("{error:?}"))?;
    crate::play::mailbox::lock(&outbox.0)
        .properties
        .house
        .model
        .selected = true;
    link.house_edit(1, "oteryn:content.area.city.thais".into());
    link.house_request(0);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("expected a house catalogue query".into());
    };
    let query = oteryn_protocol_oteryn::house::decode_house_query(&command.payload)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(query.world[15], 1);
    Ok(())
}

#[test]
fn party_notice_refresh_survives_activity_scope_handoff() -> Result<(), String> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::party::{PartyCommand, party};

    for during_handoff in [false, true] {
        let (link, mut commands, outbox) = play_channel();
        let mut view = view().map_err(|error| error.to_string())?;
        let push_notice = || {
            outbox.push(vec![SessionEvent::PartyMembershipNotice(AppliedDelta {
                server_sequence: 3,
                base_revision: 0,
                new_revision: 1,
                value: party::PartyMembershipNotice {
                    member_handle: 9,
                    change: party::PartyMembershipChange::Left,
                },
            })]);
        };
        outbox.party_initialize(true, false);
        if !during_handoff {
            push_notice();
        }
        publish_activity_handoff(&outbox);
        if during_handoff {
            push_notice();
        }
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        assert!(commands.try_recv().is_err());
        assert!(outbox.scope_needs_initialize());
        // Even after the session task claims initialization, Party is still uninitialized.
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        assert!(commands.try_recv().is_err());
        outbox.party_initialize(true, false);
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            commands.try_recv().map_err(|error| error.to_string())?,
            PlayCommand::Party(PartyCommand::query())
        );
    }
    Ok(())
}
