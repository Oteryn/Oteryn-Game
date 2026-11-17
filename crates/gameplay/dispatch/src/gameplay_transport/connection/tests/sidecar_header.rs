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

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(self.x.get()))
    }

    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        let own = self.observe(actor).await?;
        let identity = actor.placement_identity();
        Some(ChannelEntities {
            content_generation: own.content_generation,
            observer: identity,
            entities: vec![ChannelEntity {
                motion: None,
                display_name: None,
                health_percent: 100,
                direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                kind: VisibleKind::Player,
                identity,
                generation: 1,
                position: own.actor_position,
                revision: u64::try_from(own.actor_position.x).ok()?,
            }],
            lights: EntityLights::new(),
            observed_at_ms: 0,
        })
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

    async fn observe_party_icons(&self, admitted: AdmittedSession) -> Option<Vec<PartyIcon>> {
        self.relocate(Relocating::Party);
        Some(vec![PartyIcon {
            entity: Self::own(admitted)?,
            shield: 1,
            skull: 0,
        }])
    }
}

/// Joins, runs the first visibility ticks and returns, for every sidecar delta, its header
/// position and the own position the client holds when it arrives.
fn sidecar_headers(
    during: Relocating,
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
                };
                let ids = [6, 134, 508, 509];
                let offered = ids.map(|id| OfferedCapability { id, requires: &[] });
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(&offered, &ids).ok_or("capabilities")?;
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
    let headers = sidecar_headers(during)?;
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
