//! Scripted owner outcomes through the real connection dispatch and VIS-2 interest query.
use super::*;
use crate::gameplay_transport::appearance_change::observation::{Observation, ObservationError};
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use crate::gameplay_transport::item_view::InventoryItems;
use oteryn_protocol_oteryn::appearance;
use oteryn_protocol_oteryn::guild_emblem::{CAPABILITY_GUILD_EMBLEM_V1, Emblem, GuildEmblem};
use oteryn_protocol_oteryn::inventory_capacity::{
    CAPABILITY_INVENTORY_CAPACITY_V1, InventoryCapacity,
};
use oteryn_protocol_oteryn::{decode_command_result, decode_state_delta};

struct AppearanceChannel {
    channel: Channel,
    outcome: Option<appearance::ChangeResult>,
    revision: Cell<u64>,
    calls: Cell<usize>,
    defer_input: bool,
    appearance_busy: Cell<bool>,
    names_busy: Cell<bool>,
}
impl FreshAdmissionAuthority for AppearanceChannel {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn activity_transfer_input(
        &self,
        _: AdmittedSession,
    ) -> Result<(), crate::durability::DurabilityError> {
        if self.defer_input {
            return Err(crate::durability::DurabilityError::RootUnavailable);
        }
        Ok(())
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.channel.observe(actor).await
    }
    async fn step(&self, actor: ExactActorRef, direction: StepDirection) -> StepOutcome {
        self.channel.step(actor, direction).await
    }
    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        let mut entities = self.channel.observe_visible_entities(actor).await?;
        for entity in &mut entities.entities {
            if entity.identity == [9; 16] {
                entity.health_percent = if self.revision.get() == 1 { 50 } else { 25 };
            }
        }
        Some(entities)
    }
    async fn observe_named_visible_entities_result(
        &self,
        actor: ExactActorRef,
    ) -> Result<ChannelEntities, AdmissionRefusal> {
        if self.names_busy.get() {
            return Err(AdmissionRefusal::Unavailable);
        }
        let mut channel = self
            .observe_visible_entities(actor)
            .await
            .ok_or(AdmissionRefusal::Rejected)?;
        for entity in &mut channel.entities {
            if entity.kind == VisibleKind::Player {
                let name = if entity.identity == channel.observer {
                    "Observer"
                } else {
                    "Entrant"
                };
                entity.display_name = Some(
                    oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(name)
                        .map_err(|_| AdmissionRefusal::Rejected)?,
                );
            }
        }
        Ok(channel)
    }
    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<InventoryItems> {
        Some(InventoryItems::default())
    }

    async fn observe_inventory_capacity(&self, _: AdmittedSession) -> Option<InventoryCapacity> {
        Some(InventoryCapacity {
            capacity: 40_000,
            free: 39_000,
        })
    }

    async fn observe_guild_emblems(&self, admitted: AdmittedSession) -> Option<Vec<GuildEmblem>> {
        Some(vec![GuildEmblem {
            entity: EntityRef {
                identity: admitted.runtime_actor?.placement_identity(),
                generation: 1,
            },
            emblem: Emblem::Member,
        }])
    }

    async fn appearance_change(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: appearance::Change,
    ) -> Option<appearance::ChangeResult> {
        self.calls.set(self.calls.get() + 1);
        if let Some(appearance::ChangeResult::Committed(revision)) = self.outcome {
            self.revision.set(revision);
        }
        self.outcome
    }
    async fn appearance_observation(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        entities: Vec<EntityRef>,
    ) -> Option<Observation> {
        let context = appearance::Context {
            content_generation: CONTENT,
            cosmetics_digest: [7; 32],
        };
        Some(Observation {
            skipped: Vec::new(),
            catalogue: appearance::Catalogue {
                context,
                root_revision: self.revision.get(),
                selected: None,
                palette: vec![[0, 0, 0]; 133],
                outfits: vec![appearance::OutfitOption {
                    key: "oteryn:outfit.citizen".into(),
                    look_type: 128,
                    permitted_addons: 0,
                }],
                mounts: Vec::new(),
            },
            bindings: entities
                .into_iter()
                .map(|entity| appearance::Binding {
                    context,
                    appearance_ref: 1,
                    entity,
                    root_revision: self.revision.get(),
                    condition_revision: 1,
                    owner_time_us: 1,
                    look: appearance::Look::Outfit {
                        look_type: if self.revision.get() == 1 { 128 } else { 129 },
                        colours: [0; 4],
                        addons: 0,
                    },
                    selection_revision: None,
                    forge_stacks: (entity.identity == [9; 16]).then_some(15),
                })
                .collect(),
        })
    }

    async fn appearance_observation_projected_result(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        _: bool,
        entities: Vec<EntityRef>,
    ) -> Result<Observation, ObservationError> {
        if self.appearance_busy.get() {
            return Err(ObservationError::HolderBusy);
        }
        self.appearance_observation(actor, session, entities)
            .await
            .ok_or(ObservationError::Unavailable)
    }
}

#[test]
fn appearance_change_real_dispatch_orders_stationary_updates_and_excludes_hidden_players()
-> TestResult {
    run(async {
        let outcome = Some(appearance::ChangeResult::Committed(2));
        scenario(&[6, 25], outcome).await
    })
}
#[test]
fn appearance_change_rejected_unresolved_and_downgraded_clients_publish_nothing() -> TestResult {
    run(async {
        scenario(&[6, 25], Some(appearance::ChangeResult::Rejected)).await?;
        scenario(&[6, 25], None).await?;
        scenario(&[6], Some(appearance::ChangeResult::Committed(2))).await
    })
}

#[test]
fn busy_appearance_keeps_committed_step_and_defers_new_player() -> TestResult {
    committed_step_during_busy_projection(false, true)
}

#[test]
fn busy_names_keep_committed_step_and_published_identity() -> TestResult {
    committed_step_during_busy_projection(true, true)
}

#[test]
fn busy_names_alone_keep_committed_step_and_defer_new_player() -> TestResult {
    committed_step_during_busy_projection(true, false)
}

fn committed_step_during_busy_projection(named: bool, appearance_busy: bool) -> TestResult {
    use oteryn_protocol_oteryn::entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2;
    use oteryn_protocol_oteryn::world_spatial_entities::{
        EntityFields, decode_world_spatial_entities_delta_with_fields,
    };
    run(async {
        let authority = AppearanceChannel {
            channel: Channel::new(Vec::new()),
            outcome: None,
            revision: Cell::new(1),
            calls: Cell::new(0),
            defer_input: false,
            appearance_busy: Cell::new(false),
            names_busy: Cell::new(false),
        };
        let mut admitted = session(&[6], SessionContinuity::FRESH);
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            if named {
                BUNDLE_WORLD_OFFERED_CAPABILITIES
            } else {
                PRODUCTION_OFFERED_CAPABILITIES
            },
            true,
        );
        let mut selected = vec![6, 25];
        if named {
            selected.push(CAPABILITY_WORLD_SPATIAL_ENTITIES_V2);
        }
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &selected).ok_or("capabilities")?;
        if named {
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(CAPABILITY_WORLD_SPATIAL_ENTITIES_V2)
            );
        }
        let (mut client, mut server) = tokio::io::duplex(1 << 16);
        let serving = async {
            serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
            drop(server);
            TestResult::Ok(())
        };
        let observing = async {
            for _ in 0..3 {
                read_one(&mut client).await?;
            }
            let mut entrant = creature([9; 16], at(102, 100), 1);
            entrant.kind = VisibleKind::Player;
            authority.channel.others.borrow_mut().push(entrant);
            authority.appearance_busy.set(appearance_busy);
            authority.names_busy.set(named);
            let step = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    payload: &encode_step_intent(StepDirection::East),
                },
            )?;
            write_frame(&mut client, &step).await?;
            let result = read_one(&mut client).await?;
            assert_eq!(
                decode_command_result(decode_wire_envelope(&result)?.payload())?.payload,
                &encode_step_result(StepDisposition::Moved)
            );
            let moved = read_one(&mut client).await?;
            let delta = decode_state_delta(decode_wire_envelope(&moved)?.payload())?;
            assert_eq!(delta.domain_id, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY);
            let (spatial, _) = decode_world_spatial_entities_delta_with_fields(
                delta.payload,
                EntityFields::selected(&selected),
            )
            .map_err(|error| format!("{error:?}"))?;
            assert_eq!(spatial.actor_position, at(101, 100));
            assert!(spatial.enter.is_empty());
            if named {
                assert!(spatial.update.iter().any(|entity| {
                    entity
                        .display_name
                        .is_some_and(|name| matches!(name.as_str(), Ok("Observer")))
                }));
            }

            authority.appearance_busy.set(false);
            authority.names_busy.set(false);
            let mapping = read_one(&mut client).await?;
            let delta = decode_state_delta(decode_wire_envelope(&mapping)?.payload())?;
            assert_eq!(delta.domain_id, appearance::STATE_DOMAIN_APPEARANCE);
            let entered = read_one(&mut client).await?;
            let delta = decode_state_delta(decode_wire_envelope(&entered)?.payload())?;
            assert_eq!(delta.domain_id, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY);
            let (spatial, _) = decode_world_spatial_entities_delta_with_fields(
                delta.payload,
                EntityFields::selected(&selected),
            )
            .map_err(|error| format!("{error:?}"))?;
            assert_eq!(spatial.actor_position, at(101, 100));
            assert!(spatial.enter.iter().any(|entity| {
                entity.entity.identity == [9; 16]
                    && (!named
                        || entity
                            .display_name
                            .is_some_and(|name| matches!(name.as_str(), Ok("Entrant"))))
            }));
            client.shutdown().await?;
            TestResult::Ok(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            tokio::try_join!(serving, observing)
        })
        .await??;
        Ok(())
    })
}

async fn scenario(selected: &[u32], outcome: Option<appearance::ChangeResult>) -> TestResult {
    scenario_actor(selected, outcome, selected.contains(&104)).await
}
async fn scenario_actor(
    selected: &[u32],
    outcome: Option<appearance::ChangeResult>,
    as_creature: bool,
) -> TestResult {
    let mut shown = creature([9; 16], at(102, 100), 1);
    if !as_creature {
        shown.kind = VisibleKind::Player;
    }
    shown.health_percent = 50;
    let mut hidden = creature([8; 16], at(1000, 1000), 1);
    hidden.kind = VisibleKind::Player;
    let authority = AppearanceChannel {
        channel: Channel::new(vec![shown, hidden]),
        outcome,
        revision: Cell::new(1),
        calls: Cell::new(0),
        defer_input: false,
        appearance_busy: Cell::new(false),
        names_busy: Cell::new(false),
    };
    let mut admitted = session(&[6], SessionContinuity::FRESH);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        PRODUCTION_OFFERED_CAPABILITIES,
        true,
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, selected).ok_or("capabilities")?;
    let (mut client, mut server) = tokio::io::duplex(256 * 1024);
    let change = appearance::Change {
        context: appearance::Context {
            content_generation: CONTENT,
            cosmetics_digest: [7; 32],
        },
        expected_root_revision: 1,
        prior: None,
        selection: appearance::Selection {
            outfit_key: "oteryn:outfit.citizen".into(),
            colours: [1, 2, 3, 4],
            addons: 0,
            mount_key: None,
        },
        content_revision: "r1".into(),
    };
    let payload = change.encode().map_err(|e| format!("{e:?}"))?;
    let command = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: 1,
            command_type: 25,
            payload: &payload,
        },
    )?;
    let serving = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS);
    let binding_count = if as_creature && !selected.contains(&104) {
        1
    } else {
        2
    };
    let observing = async {
        // Consume the actual join framing. Domain 21 must precede VIS-2 in its body.
        let _begin = read_one(&mut client).await?;
        let chunk = read_one(&mut client).await?;
        let envelope = decode_wire_envelope(&chunk)?;
        let framing = oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())?;
        let domains = oteryn_protocol_oteryn::decode_snapshot_body(framing.1)?;
        if selected.contains(&25) {
            assert_eq!(domains[0].domain_id, 21);
            let snapshot =
                appearance::Snapshot::decode(domains[0].payload).map_err(|e| format!("{e:?}"))?;
            assert_eq!(snapshot.catalogue.root_revision, 1);
            assert!(snapshot.catalogue.selected.is_none());
            assert_eq!(snapshot.catalogue.outfits.len(), 1);
            assert_eq!(snapshot.catalogue.outfits[0].look_type, 128);
            assert_eq!(snapshot.bindings.len(), binding_count);
            let shown = snapshot
                .bindings
                .iter()
                .find(|b| b.entity.identity == [9; 16]);
            if binding_count == 1 {
                assert!(shown.is_none());
            } else {
                assert_eq!(
                    shown.ok_or("shown binding")?.forge_stacks,
                    selected.contains(&104).then_some(15)
                );
            }
            assert!(
                !snapshot
                    .bindings
                    .iter()
                    .any(|b| b.entity.identity == [8; 16])
            );
        } else {
            assert!(!domains.iter().any(|d| d.domain_id == 21));
        }
        let spatial = domains.iter().find(|d| d.domain_id == 1).ok_or("spatial")?;
        let spatial =
            oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot(
                spatial.payload,
            )
            .map_err(|e| format!("{e:?}"))?;
        let shown = spatial
            .entities
            .iter()
            .find(|e| e.entity.identity == [9; 16])
            .ok_or("shown")?;
        assert!(matches!(
            shown.detail,
            oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
                health_percent: 50,
                ..
            }
        ));
        let _commit = read_one(&mut client).await?;
        write_frame(&mut client, &command).await?;
        if selected.contains(&25) && outcome.is_none() {
            assert!(read_one(&mut client).await.is_err());
        } else {
            let result = read_one(&mut client).await?;
            let envelope = decode_wire_envelope(&result)?;
            let result = decode_command_result(envelope.payload())?;
            if selected.contains(&25) {
                assert_eq!(
                    appearance::ChangeResult::decode(result.payload)
                        .map_err(|e| format!("{e:?}"))?,
                    outcome.ok_or("outcome")?
                );
            } else {
                assert!(result.payload.is_empty());
            }
            if selected.contains(&25)
                && matches!(outcome, Some(appearance::ChangeResult::Committed(_)))
            {
                let catalogue = read_one(&mut client).await?;
                let envelope = decode_wire_envelope(&catalogue)?;
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!(
                    (delta.domain_id, delta.delta_type),
                    (21, appearance::DELTA_TYPE_APPEARANCE_CATALOGUE)
                );
                let catalogue =
                    appearance::Catalogue::decode(delta.payload).map_err(|e| format!("{e:?}"))?;
                assert_eq!(catalogue.root_revision, 2);
                assert_eq!(catalogue.context, change.context);
                let mappings = read_one(&mut client).await?;
                let envelope = decode_wire_envelope(&mappings)?;
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!((delta.domain_id, delta.delta_type), (21, 1));
                assert_eq!(
                    appearance::Mappings::decode(delta.payload)
                        .map_err(|e| format!("{e:?}"))?
                        .upserts
                        .len(),
                    binding_count
                );
                let spatial = read_one(&mut client).await?;
                let envelope = decode_wire_envelope(&spatial)?;
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!(delta.domain_id, 1);
                let delta=oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta(delta.payload).map_err(|e|format!("{e:?}"))?;
                assert_eq!(delta.actor_position, at(100, 100));
                assert_eq!(delta.update.len(), 2);
                assert!(delta.enter.is_empty());
                let shown = delta
                    .update
                    .iter()
                    .find(|e| e.entity.identity == [9; 16])
                    .ok_or("shown health update")?;
                assert!(matches!(
                    shown.detail,
                    oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
                        health_percent: 25,
                        ..
                    }
                ));
            } else {
                assert!(
                    tokio::time::timeout(
                        std::time::Duration::from_millis(150),
                        read_one(&mut client)
                    )
                    .await
                    .is_err()
                );
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn Error>>(())
    };
    let observed = RefCell::new(None);
    let end = crate::gameplay_transport::first(serving, async {
        observed.replace(Some(observing.await));
    })
    .await;
    drop(server);
    if let Some(result) = observed.into_inner() {
        result?;
    } else {
        assert!(selected.contains(&25) && outcome.is_none());
        let Some(ConnectionEnd::AdmittedThenDisconnected(admitted)) = end else {
            return Err("unresolved connection".into());
        };
        assert_eq!(admitted.continuity.next_command_id, 1);
        assert_eq!(admitted.continuity.server_sequence, 0);
        assert!(read_one(&mut client).await.is_err());
    }
    assert_eq!(authority.calls.get(), usize::from(selected.contains(&25)));
    Ok(())
}

#[test]
fn forge_creature_real_dispatch_exposes_negotiated_marker_with_stationary_health() -> TestResult {
    run(async { scenario(&[6, 25, 104], Some(appearance::ChangeResult::Committed(2))).await })
}
#[test]
fn forge_creature_real_dispatch_downgrade_omits_creature_bindings() -> TestResult {
    run(async {
        scenario_actor(&[6, 25], Some(appearance::ChangeResult::Committed(2)), true).await
    })
}

#[test]
fn qualification_session_decodes_served_visibility_and_guarded_step() -> TestResult {
    use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
    use oteryn_protocol_oteryn::actor_motion::{ActorMotion, CAPABILITY_ACTOR_MOTION_V1};
    use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
    use oteryn_protocol_oteryn::world_spatial::CAPABILITY_PACED_MOVEMENT_V1;
    use oteryn_protocol_oteryn::world_spatial_entities::{
        CAPABILITY_NATIVE_CREATURE_ICONS_V1, NativeCreatureIcon, NativeCreatureIcons,
    };
    use oteryn_protocol_oteryn::{ServerAcceptedValue, encode_server_accepted};
    use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, SessionEvent};

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let motion = ActorMotion {
                    step_duration_ms: 200,
                    ..ActorMotion::default()
                };
                let icons = NativeCreatureIcons::new(&[NativeCreatureIcon {
                    category: 1,
                    id: 6,
                    count: 15,
                }])
                .map_err(|error| format!("{error:?}"))?;
                let mut shown = creature([9; 16], at(102, 100), 1);
                shown.motion = Some(motion);
                shown.source_icons = icons;
                let authority = AppearanceChannel {
                    channel: Channel::new(vec![shown]),
                    outcome: None,
                    revision: Cell::new(1),
                    calls: Cell::new(0),
                    defer_input: true,
                    appearance_busy: Cell::new(false),
                    names_busy: Cell::new(false),
                };
                let bundle_appearance =
                    crate::gameplay_transport::appearance_change::offer::offered(
                        BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        true,
                    );
                let spatial = [
                    CAPABILITY_ITEM_VIEW_MOVE_V1,
                    CAPABILITY_WORLD_SPATIAL_ENTITIES,
                    CAPABILITY_PACED_MOVEMENT_V1,
                    appearance::CAPABILITY_APPEARANCE_V1,
                    CAPABILITY_NATIVE_CREATURE_ICONS_V1,
                    CAPABILITY_ACTOR_MOTION_V1,
                    CAPABILITY_INVENTORY_CAPACITY_V1,
                    CAPABILITY_GUILD_EMBLEM_V1,
                ];
                let spatial_offers: Vec<_> = bundle_appearance
                    .into_iter()
                    .filter(|offer| spatial.contains(&offer.id))
                    .collect();
                let selected =
                    SelectedCapabilities::select(&spatial_offers, CLIENT_SUPPORTED_CAPABILITIES)
                        .ok_or("qualification capability selection")?;
                assert_eq!(selected.as_slice(), spatial);
                let mut admitted = session(&[6], SessionContinuity::FRESH);
                admitted.continuity.selected_capabilities = selected;
                let (mut server, client) = tokio::io::duplex(256 * 1024);
                let serving = async {
                    // Bootstrap only the already-authorized identity. The join snapshot
                    // itself comes from the Channel connection's production writer.
                    read_one(&mut server).await?;
                    let accepted = encode_server_accepted(&ServerAcceptedValue {
                        game_session_id: admitted.game_session_id,
                        world_id: admitted.world_id,
                        channel_id: admitted.channel_id,
                        connection_generation: 1,
                        current_server_sequence: 0,
                        next_command_id: 1,
                        schema_revision: 1,
                        selected_capabilities: selected.as_slice(),
                    })?;
                    crate::gameplay_transport::tcp_tls::write_frame(&mut server, &accepted).await?;
                    serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
                    Ok::<_, Box<dyn Error>>(())
                };
                let joining = async {
                    let mut session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: oteryn_protocol_oteryn::CharacterId::decode(&uuid_v7(
                                0x55,
                            ))?,
                            admission_material: b"qualification join",
                            client_build_id: "node-boot",
                            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                            deadline: std::time::Duration::from_secs(5),
                        },
                    )
                    .await
                    .map_err(|error| format!("qualification join snapshot: {error:?}"))?;
                    let entity = session
                        .world_entities()
                        .ok_or("type-2 visibility snapshot")?
                        .get(&EntityRef {
                            identity: shown.identity,
                            generation: shown.generation,
                        })
                        .ok_or("visible creature")?;
                    assert_eq!(entity.motion, Some(motion));
                    assert!(matches!(
                        entity.detail,
                        EntityDetail::ActorWithIcons { icons: actual, .. } if actual == icons
                    ));
                    let mut capacity = None;
                    let mut emblems = None;
                    let deadline = tokio::time::Duration::from_secs(2);
                    tokio::time::timeout(deadline, async {
                        while capacity.is_none() || emblems.is_none() {
                            for event in session.wait_for_events(deadline).await? {
                                match event {
                                    SessionEvent::InventoryCapacity(delta) => {
                                        capacity = Some(delta.value)
                                    }
                                    SessionEvent::GuildEmblems(delta) => {
                                        emblems = Some(delta.value)
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok::<_, oteryn_session::SessionError>(())
                    })
                    .await?
                    .map_err(|error| format!("qualification visibility delta: {error:?}"))?;
                    assert_eq!(
                        capacity,
                        authority.observe_inventory_capacity(admitted).await
                    );
                    assert_eq!(emblems, authority.observe_guild_emblems(admitted).await);

                    // M04 walks to the spawn with Session::step; it can report the same
                    // spatial error from a STEP receipt even after visibility decoded.
                    let step = session
                        .step(StepDirection::East)
                        .await
                        .map_err(|error| format!("qualification STEP result: {error:?}"))?;
                    assert_eq!(step.status, CommandStatus::Rejected);
                    assert_eq!(step.disposition, StepDisposition::Rejected);
                    Ok::<_, Box<dyn Error>>(())
                };
                tokio::try_join!(serving, joining)?;
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "qualification join test thread panicked")?
        .map_err(Into::into)
}
