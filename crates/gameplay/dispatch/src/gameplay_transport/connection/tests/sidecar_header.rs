//! A header-only sidecar delta (fields 6-8) names the own position the session last
//! published, even when the actor was relocated (a staff `Bring`) after the projection was sent
//! and before the sidecar observation finished; the client refuses any other header.
use super::*;
use crate::gameplay_transport::capabilities::OfferedCapability;
use crate::gameplay_transport::world_spatial::{ActorPosition, EntityKind, EntityRef};
use oteryn_protocol_oteryn::guild_emblem::{Emblem, GuildEmblem};
use oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity;
use oteryn_protocol_oteryn::party_skull::PartyIcon;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, decode_world_spatial_entities_delta_with_sidecar,
};
use std::time::Duration;

/// The sidecar observation inside which the actor is relocated.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Relocating {
    Party,
    Capacity,
    Emblem,
}

/// Relocates the own actor one tile east inside the sidecar observation `during` names.
struct RelocatingAuthority {
    x: Cell<i32>,
    during: Relocating,
    party_peer: Option<EntityRef>,
    party_shield: Cell<u8>,
    visibility_busy: Cell<bool>,
    party_updates: Option<tokio::sync::watch::Sender<()>>,
}

impl RelocatingAuthority {
    fn relocate(&self, here: Relocating) {
        if here == self.during {
            self.x.set(self.x.get() + 1);
        }
    }

    fn own(admitted: AdmittedSession) -> Option<EntityRef> {
        Some(EntityRef {
            identity: admitted.runtime_actor?.placement_identity(),
            generation: 1,
        })
    }
}

impl FreshAdmissionAuthority for RelocatingAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn house_scope_ack(&self, _: AdmittedSession) -> Result<Option<(u64, Vec<u8>)>, ()> {
        Ok(Some((1, vec![1])))
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(self.x.get()))
    }

    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        let own = self.observe(actor).await?;
        let identity = actor.placement_identity();
        let observer = ChannelEntity {
            source_icons: Default::default(),
            motion: None,
            display_name: None,
            health_percent: 100,
            direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
            kind: VisibleKind::Player,
            identity,
            generation: 1,
            position: own.actor_position,
            revision: u64::try_from(own.actor_position.x).ok()?,
        };
        let mut entities = vec![observer];
        if let Some(peer) = self.party_peer {
            entities.push(ChannelEntity {
                identity: peer.identity,
                generation: peer.generation,
                position: at(1).actor_position,
                ..observer
            });
        }
        Some(ChannelEntities {
            content_generation: own.content_generation,
            observer: identity,
            entities,
            lights: EntityLights::new(),
            observed_at_ms: 0,
        })
    }

    async fn observe_named_visible_entities_result(
        &self,
        actor: ExactActorRef,
    ) -> Result<ChannelEntities, AdmissionRefusal> {
        use oteryn_protocol_oteryn::entity_names::EntityDisplayName;

        if self.visibility_busy.get() {
            return Err(AdmissionRefusal::Unavailable);
        }
        let mut channel = self
            .observe_visible_entities(actor)
            .await
            .ok_or(AdmissionRefusal::Rejected)?;
        for entry in &mut channel.entities {
            let name = if entry.identity == channel.observer {
                "Observer"
            } else {
                "Party Peer"
            };
            entry.display_name =
                Some(EntityDisplayName::new(name).map_err(|_| AdmissionRefusal::Rejected)?);
        }
        Ok(channel)
    }

    async fn observe_inventory_capacity(&self, _: AdmittedSession) -> Option<InventoryCapacity> {
        self.relocate(Relocating::Capacity);
        Some(InventoryCapacity {
            capacity: 40_000,
            free: 39_000,
        })
    }

    async fn observe_guild_emblems(&self, admitted: AdmittedSession) -> Option<Vec<GuildEmblem>> {
        self.relocate(Relocating::Emblem);
        Some(vec![GuildEmblem {
            entity: Self::own(admitted)?,
            emblem: Emblem::Member,
        }])
    }

    async fn observe_party_icons(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Vec<PartyIcon>, crate::durability::DurabilityError> {
        self.relocate(Relocating::Party);
        Ok(vec![PartyIcon {
            entity: self
                .party_peer
                .or_else(|| Self::own(admitted))
                .ok_or(crate::durability::DurabilityError::Unavailable)?,
            shield: self.party_shield.get(),
            skull: 0,
        }])
    }

    fn party_icon_updates(&self, _: AdmittedSession) -> Option<tokio::sync::watch::Receiver<()>> {
        self.party_updates
            .as_ref()
            .map(|updates| updates.subscribe())
    }
}

/// Joins, runs the first visibility ticks and returns, for every sidecar delta, its header
/// position and the own position the client holds when it arrives.
fn sidecar_headers(
    during: Relocating,
    house: bool,
) -> Result<Vec<(ActorPosition, ActorPosition)>, Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let captured = RefCell::new(Vec::new());
            run(async {
                tokio::time::pause();
                let authority = RelocatingAuthority {
                    x: Cell::new(0),
                    during,
                    party_peer: None,
                    party_shield: Cell::new(1),
                    visibility_busy: Cell::new(false),
                    party_updates: None,
                };
                let ids = [6, 134, 508, 509];
                let offered = ids.map(|id| OfferedCapability { id, requires: &[] });
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(&offered, &ids).ok_or("capabilities")?;
                if house {
                    admitted.item_fence = Some(
                        crate::durability::item_transfer::CurrentCharacterItemFence {
                            character_id: crate::domain::CharacterId::from_bytes(CHARACTER)
                                .map_err(|error| format!("{error:?}"))?,
                            game_session_id: admitted.game_session_id,
                            connection_generation: crate::foundation::ConnectionGeneration::new(1)
                                .map_err(|error| format!("{error:?}"))?,
                            character_lease_generation: 1,
                            runtime_scope: crate::foundation::RuntimeScopeRefV1::instance(
                                admitted.world_id,
                                uuid_v7(0x66),
                            )
                            .map_err(|error| format!("{error:?}"))?,
                            scope_ownership_generation:
                                crate::foundation::ScopeOwnershipGeneration::new(1)
                                    .map_err(|error| format!("{error:?}"))?,
                        },
                    );
                }
                let fields = EntityFields::selected(&ids);
                let (mut server, mut client) = tokio::io::duplex(256 * 1024);
                let serving = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS);
                let headers = RefCell::new(Vec::new());
                let driving = async {
                    // The join snapshot shows the own actor where `observe` first put it.
                    let mut own = at(0).actor_position;
                    let deadline = tokio::time::Instant::now() + Duration::from_millis(1100);
                    while tokio::time::Instant::now() < deadline {
                        let Ok(frame) = tokio::time::timeout(
                            Duration::from_millis(50),
                            crate::gameplay_transport::tcp_tls::read_frame(&mut client),
                        )
                        .await
                        else {
                            continue;
                        };
                        let frame = frame?;
                        let envelope = decode_wire_envelope(&frame)?;
                        if envelope.message_type() != MessageType::StateDelta {
                            continue;
                        }
                        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                        if delta.domain_id != STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                            continue;
                        }
                        let (delta, _, sidecar) =
                            decode_world_spatial_entities_delta_with_sidecar(delta.payload, fields)
                                .map_err(|error| format!("{error:?}"))?;
                        if sidecar.is_some() {
                            headers.borrow_mut().push((delta.actor_position, own));
                        } else if let Some(entity) = delta
                            .enter
                            .iter()
                            .chain(&delta.update)
                            .find(|entity| entity.kind == EntityKind::Player)
                        {
                            own = entity.position;
                        }
                    }
                    client.shutdown().await?;
                    Ok::<_, Box<dyn Error>>(())
                };
                let (_end, result) = tokio::join!(serving, driving);
                result?;
                *captured.borrow_mut() = headers.into_inner();
                Ok(())
            })
            .map_err(|error| error.to_string())?;
            Ok::<_, String>(captured.into_inner())
        })?
        .join()
        .map_err(|_| "sidecar header connection test panicked")?
        .map_err(Into::into)
}

fn assert_headers_at_published_position(during: Relocating) -> Result<(), Box<dyn Error>> {
    let headers = sidecar_headers(during, false)?;
    assert_eq!(
        headers.len(),
        3,
        "one party, one capacity and one emblem sidecar: {headers:?}"
    );
    for (header, held) in headers {
        assert_eq!(
            header, held,
            "the sidecar header must name the own position the client holds"
        );
    }
    Ok(())
}

#[test]
fn inventory_capacity_sidecar_keeps_the_published_position_after_a_relocation()
-> Result<(), Box<dyn Error>> {
    assert_headers_at_published_position(Relocating::Capacity)
}

#[test]
fn guild_emblem_sidecar_keeps_the_published_position_after_a_relocation()
-> Result<(), Box<dyn Error>> {
    assert_headers_at_published_position(Relocating::Emblem)
}

#[test]
fn party_icon_sidecar_keeps_the_published_position_after_a_relocation() -> Result<(), Box<dyn Error>>
{
    assert_headers_at_published_position(Relocating::Party)
}

#[test]
fn party_leader_shield_republishes_while_visibility_is_busy() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::world_spatial_entities::DeltaSidecar;

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let entity = EntityRef {
                    identity: uuid_v7(0x66),
                    generation: 1,
                };
                let authority = RelocatingAuthority {
                    x: Cell::new(0),
                    during: Relocating::Capacity,
                    party_peer: Some(entity),
                    party_shield: Cell::new(3),
                    visibility_busy: Cell::new(false),
                    party_updates: None,
                };
                let ids = [6, 42, 134, 286];
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &ids)
                        .filter(|selected| selected.as_slice() == ids)
                        .ok_or("party shield capabilities")?;
                let fields = EntityFields::selected(&ids);
                let (mut server, mut client) = tokio::io::duplex(256 * 1024);
                let serving = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS);
                let driving = async {
                    let observed = tokio::time::timeout(Duration::from_secs(3), async {
                        loop {
                            let frame =
                                crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            if envelope.message_type() != MessageType::StateDelta {
                                continue;
                            }
                            let delta =
                                oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                            if delta.domain_id != STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                                continue;
                            }
                            let (_, _, sidecar) = decode_world_spatial_entities_delta_with_sidecar(
                                delta.payload,
                                fields,
                            )
                            .map_err(|error| format!("{error:?}"))?;
                            if let Some(DeltaSidecar::PartyIcons(icons)) = sidecar {
                                let shield = icons
                                    .iter()
                                    .find(|icon| icon.entity == entity)
                                    .ok_or("peer shield")?
                                    .shield;
                                if authority.visibility_busy.get() {
                                    return Ok::<_, Box<dyn Error>>(shield);
                                }
                                assert_eq!(shield, 3, "initial member shield");
                                authority.party_shield.set(4);
                                authority.visibility_busy.set(true);
                            }
                        }
                    })
                    .await;
                    client.shutdown().await?;
                    assert_eq!(
                        observed.ok().transpose()?,
                        Some(4),
                        "leadership change must publish against the last visible incarnation"
                    );
                    Ok::<_, Box<dyn Error>>(())
                };
                let (_, result) = tokio::join!(serving, driving);
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "party shield connection test panicked")?
        .map_err(Into::into)
}

#[test]
fn leadership_shield_write_times_out_within_visibility_budget() -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let (updates, _) = tokio::sync::watch::channel(());
                let authority = RelocatingAuthority {
                    x: Cell::new(0),
                    during: Relocating::Capacity,
                    party_peer: None,
                    party_shield: Cell::new(4),
                    visibility_busy: Cell::new(false),
                    party_updates: Some(updates.clone()),
                };
                let ids = [6, 42, 134];
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &ids)
                        .filter(|selected| selected.as_slice() == ids)
                        .ok_or("party shield capabilities")?;
                // The peer stops reading after the join. Only the next frame's
                // length prefix fits, so a retry on this stream would corrupt it.
                let (mut server, mut client) = tokio::io::duplex(4);
                let serving = async move {
                    tokio::time::timeout(
                        VISIBILITY_REFRESH * 2,
                        serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS),
                    )
                    .await
                };
                let join = async {
                    loop {
                        let frame =
                            crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
                        if decode_wire_envelope(&frame)?.message_type()
                            == MessageType::SnapshotCommit
                        {
                            updates.send_replace(());
                            return Ok::<_, Box<dyn Error>>(());
                        }
                    }
                };
                let (end, joined) = tokio::join!(serving, join);
                joined?;
                assert!(
                    matches!(end, Ok(ConnectionEnd::AdmittedThenControlLost(_))),
                    "a stalled leadership shield write must end within the visibility budget: {end:?}"
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "shield backpressure test panicked")?
        .map_err(Into::into)
}

#[test]
fn house_tick_sends_negotiated_capacity_and_emblem() -> Result<(), Box<dyn Error>> {
    let headers = sidecar_headers(Relocating::Capacity, true)?;
    assert_eq!(headers.len(), 2);
    assert!(headers.iter().all(|(header, held)| header == held));
    Ok(())
}
