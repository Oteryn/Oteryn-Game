//! A terminal peer leaves after visibility was captured, before appearance takes its census.
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::appearance_change::observation::{Observation, ObservationError};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, FreshAdmissionAttempt, FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn::appearance;

struct DepartingPeer<'o, 'a, 'f, 's> {
    owner: &'o ComposedFreshAdmission<'a, 'f, 's>,
    peer: ExactActorRef,
    session: GameSessionId,
    armed: Cell<bool>,
    skipped: Cell<bool>,
    busy_at_join: Cell<bool>,
}
impl FreshAdmissionAuthority for DepartingPeer<'_, '_, '_, '_> {
    async fn observe_character_inventory(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<crate::gameplay_transport::item_view::InventoryItems> {
        Some(crate::gameplay_transport::item_view::InventoryItems::default())
    }
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn observe(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.owner.observe(actor).await
    }
    async fn observe_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::ChannelEntities> {
        self.owner.observe_visible_entities(actor).await
    }
    fn appearance_fallback_catalogue(&self) -> Option<appearance::Catalogue> {
        self.owner.appearance_fallback_catalogue()
    }
    async fn appearance_observation_projected(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        empty: bool,
        entities: Vec<appearance::EntityRef>,
    ) -> Option<Observation> {
        let held = if self.busy_at_join.replace(false) {
            Some(self.owner.root.try_issue_semantic_pass().expect("fixture holder"))
        } else {
            None
        };
        let observed = self.owner.appearance_observation_projected(actor, session, empty, entities).await;
        if let Some(held) = held {
            assert!(observed.is_none(), "held past the advisory budget");
            held.run(|_, _| Box::pin(async { Ok(()) })).await.expect("return fixture holder");
        }
        observed
    }
    async fn appearance_observation_projected_result(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        empty: bool,
        entities: Vec<appearance::EntityRef>,
    ) -> Result<Observation, ObservationError> {
        if self.armed.replace(false) {
            let peer = appearance::EntityRef {
                identity: self.peer.placement_identity(),
                generation: self.peer.actor_local_generation(),
            };
            assert!(
                entities.contains(&peer),
                "visibility must precede the departure"
            );
            // retire() uses this exact removal after proving the session TERMINAL.
            self.owner
                .runtime
                .lock()
                .await
                .remove_terminal_session(self.session, self.peer)
                .map_err(|_| ObservationError::Unavailable)?;
        }
        let observation = self
            .owner
            .appearance_observation_projected_result(actor, session, empty, entities)
            .await?;
        self.skipped.set(
            self.skipped.get()
                || observation
                    .skipped
                    .iter()
                    .any(|entity| entity.identity == self.peer.placement_identity()),
        );
        Ok(observation)
    }
}

#[test]
fn appearance_census_departure_race_keeps_live_connection_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look_fixtures::large_stack(|| scenario(false))
}

#[test]
fn appearance_join_degrades_on_holder_deadline_and_recovers_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("appearance-join-deadline", || scenario(true))
}

fn scenario(busy_at_join: bool) -> TestResult {
    use super::appearance_change_postgres_fixtures::observer_visibility_tests;
    run(async move |admin| {
        let harness = Harness::create_appearance(admin, "appearance_census_departure").await?;
        prepare_current_session(&harness).await?;
        harness.seed_appearance_observer().await?;
        observer_visibility_tests::admit(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = controller.active();
        {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    dependencies.actor,
                    dependencies.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        dependencies.initialize(&owner).await;
        let observer = observer_visibility_tests::initialize(&dependencies, &owner).await?;
        let observer_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
        let visible = owner
            .observe_visible_entities(observer)
            .await
            .ok_or("observer visibility")?;
        let players = visible
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == crate::gameplay_transport::world_spatial::VisibleKind::Player
            })
            .map(|entity| appearance::EntityRef {
                identity: entity.identity,
                generation: entity.generation,
            })
            .collect::<Vec<_>>();
        assert_eq!(players.len(), 2);
        let dead = appearance::EntityRef {
            identity: dependencies.actor.placement_identity(),
            generation: dependencies.actor.actor_local_generation(),
        };
        assert!(players.contains(&dead));
        let mut admitted = dependencies.admitted;
        admitted.game_session_id = observer_session;
        admitted.runtime_actor = Some(observer);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(observer_session)
            .await?;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                &offered,
                &[4, 6, 19, 25],
            )
            .ok_or("appearance capabilities")?;
        let projection = DepartingPeer {
            owner: &owner,
            peer: dependencies.actor,
            session: dependencies.admitted.game_session_id,
            armed: Cell::new(false),
            skipped: Cell::new(false),
            busy_at_join: Cell::new(busy_at_join),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        tokio::time::timeout(Duration::from_secs(10), async {
            let serving = crate::gameplay_transport::connection::serve_admitted(&mut server, admitted, &projection, crate::gameplay_transport::connection::IDLE_LIVENESS);
            tokio::pin!(serving);
            let initial = tokio::select! {
                end = &mut serving => return Err(format!("initial connection ended: {end:?}").into()),
                value = read_appearance_snapshot(&mut client) => value?,
            };
            assert_eq!(initial.bindings.len(), 2);
            if busy_at_join {
                assert!(initial.bindings.iter().all(|b| b.look == appearance::Look::EmptyOutfit));
                assert!(initial.catalogue.outfits.is_empty());
                let recovered = tokio::select! {
                    end = &mut serving => return Err(format!("degraded join ended: {end:?}").into()),
                    value = read_appearance_mappings(&mut client) => value?,
                };
                assert_eq!(recovered.upserts.len(), 2);
                assert!(recovered.upserts.iter().all(|b| matches!(b.look, appearance::Look::Outfit { .. })));
                return Ok(());
            }
            projection.armed.set(true);
            let forgotten = tokio::select! {
                end = &mut serving => return Err(format!("census departure disconnected observer: {end:?}").into()),
                value = read_appearance_mappings(&mut client) => value?,
            };
            assert_eq!(forgotten.forgets, vec![dead]);
            assert!(projection.skipped.get());
            // A later visibility pass still answers a live peer's liveness ACK.
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            loop {
                let length = usize::try_from(tokio::select! {
                    end = &mut serving => return Err(format!("connection ended after skip: {end:?}").into()),
                    value = client.read_u32() => value?,
                })?;
                let mut bytes = vec![0; length];
                client.read_exact(&mut bytes).await?;
                let envelope = crate::foundation::decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == crate::foundation::MessageType::LivenessProbe {
                    let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
                    let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, probe, envelope.server_sequence()).map_err(debug)?;
                    client.write_u32(u32::try_from(ack.len())?).await?;
                    client.write_all(&ack).await?;
                    break;
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        }).await??;
        drop(owner);
        drop(authority);
        drop(seal);
        eprintln!("PG_EXECUTED appearance_census_departure skipped=1 connection_live=1");
        harness.cleanup().await
    })
}

async fn read_appearance_snapshot(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<appearance::Snapshot> {
    use tokio::io::AsyncReadExt;
    let mut appearance = None;
    loop {
        let length = usize::try_from(client.read_u32().await?)?;
        assert!(length <= 1 << 20);
        let mut bytes = vec![0; length];
        client.read_exact(&mut bytes).await?;
        let envelope = crate::foundation::decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            crate::foundation::MessageType::SnapshotChunk => {
                let (_, body) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())
                        .map_err(debug)?;
                for domain in oteryn_protocol_oteryn::decode_snapshot_body(body).map_err(debug)? {
                    if domain.domain_id == 21 {
                        appearance =
                            Some(appearance::Snapshot::decode(domain.payload).map_err(debug)?);
                    }
                }
            }
            crate::foundation::MessageType::SnapshotCommit => {
                return appearance.ok_or_else(|| "appearance snapshot missing".into());
            }
            _ => {}
        }
    }
}

async fn read_appearance_mappings(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<appearance::Mappings> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    loop {
        let length = usize::try_from(client.read_u32().await?)?;
        assert!(length <= 1 << 20);
        let mut bytes = vec![0; length];
        client.read_exact(&mut bytes).await?;
        let envelope = crate::foundation::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == crate::foundation::MessageType::LivenessProbe {
            let probe =
                oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
            let ack =
                oteryn_protocol_oteryn::encode_liveness_ack(1, probe, envelope.server_sequence())
                    .map_err(debug)?;
            client.write_u32(u32::try_from(ack.len())?).await?;
            client.write_all(&ack).await?;
        } else if envelope.message_type() == crate::foundation::MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 21
                && delta.delta_type == appearance::DELTA_TYPE_APPEARANCE_MAPPINGS
            {
                return appearance::Mappings::decode(delta.payload).map_err(debug);
            }
        }
    }
}
