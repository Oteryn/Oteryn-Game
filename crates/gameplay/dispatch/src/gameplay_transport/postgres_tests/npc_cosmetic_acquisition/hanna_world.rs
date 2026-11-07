//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (32408, 32221, 7);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_HORSE_BUNDLE").ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| npc.key == "oteryn:npc.hanna")
            .map(|n| NpcPlacements {
                key: n.key.clone(),
                placements: n
                    .placements
                    .iter()
                    .map(|p| NpcPlacement {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                        direction: match p.direction {
                            oteryn_world_bundle::npc::Direction::North => MovementFacing::North,
                            oteryn_world_bundle::npc::Direction::East => MovementFacing::East,
                            oteryn_world_bundle::npc::Direction::South => MovementFacing::South,
                            oteryn_world_bundle::npc::Direction::West => MovementFacing::West,
                        },
                    })
                    .collect(),
            })
            .collect(),
    };
    let hanna = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.hanna")
        .ok_or("real Hanna source placement")?;
    assert_eq!(hanna.placements.len(), 1);
    assert_eq!(
        (
            hanna.placements[0].x,
            hanna.placements[0].y,
            hanna.placements[0].floor
        ),
        (32408, 32220, -7)
    );
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("retailer native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32408,y:32221,floor:-7}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}

fn catalogues(
    frames: &[Vec<u8>],
) -> TestResult<Vec<oteryn_protocol_oteryn::appearance::Catalogue>> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use oteryn_protocol_oteryn::appearance as app;
    let mut output = Vec::new();
    let mut begin = None;
    let mut chunks = Vec::new();
    for frame in frames {
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotBegin => {
                assert!(begin.is_none());
                begin = Some(
                    oteryn_protocol_oteryn::decode_snapshot_begin(envelope.payload())
                        .map_err(debug)?,
                );
            }
            MessageType::SnapshotChunk => {
                let header = begin.ok_or("appearance chunk before begin")?;
                assert_eq!(
                    oteryn_protocol_oteryn::decode_snapshot_id(envelope.payload())
                        .map_err(debug)?,
                    header.snapshot_id
                );
                let (index, body) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())
                        .map_err(debug)?;
                assert_eq!(usize::try_from(index)?, chunks.len());
                chunks.push(body.to_vec());
            }
            MessageType::SnapshotCommit => {
                let header = begin.take().ok_or("appearance commit before begin")?;
                assert_eq!(
                    oteryn_protocol_oteryn::decode_snapshot_id(envelope.payload())
                        .map_err(debug)?,
                    header.snapshot_id
                );
                assert_eq!(u32::try_from(chunks.len())?, header.chunk_count);
                let body: Vec<u8> = chunks.drain(..).flatten().collect();
                assert_eq!(u64::try_from(body.len())?, header.total_encoded_bytes);
                for domain in oteryn_protocol_oteryn::decode_snapshot_body(&body).map_err(debug)? {
                    if domain.domain_id == app::STATE_DOMAIN_APPEARANCE {
                        output.push(
                            app::Snapshot::decode(domain.payload)
                                .map_err(debug)?
                                .catalogue,
                        );
                    }
                }
            }
            MessageType::StateDelta => {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                if delta.domain_id == app::STATE_DOMAIN_APPEARANCE
                    && delta.delta_type == app::DELTA_TYPE_APPEARANCE_CATALOGUE
                {
                    output.push(app::Catalogue::decode(delta.payload).map_err(debug)?);
                }
            }
            _ => {}
        }
    }
    assert!(begin.is_none() && chunks.is_empty());
    Ok(output)
}
pub(super) fn assert_catalogues(frames: &[Vec<u8>], before: bool, after: bool) -> TestResult {
    let entries = catalogues(frames)?;
    let first = entries
        .first()
        .ok_or("real appearance bootstrap catalogue")?;
    let last = entries
        .last()
        .ok_or("real appearance catalogue after source")?;
    let owns = |catalogue: &oteryn_protocol_oteryn::appearance::Catalogue| {
        catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.key == "oteryn:content.outfit.citizen_outfits")
            .is_some_and(|outfit| outfit.permitted_addons & 2 != 0)
    };
    assert_eq!(owns(first), before, "actual bootstrap Citizen addon2");
    assert_eq!(owns(last), after, "actual published Citizen addon2");
    assert_eq!(
        first.context, last.context,
        "same authoritative content/cosmetics context"
    );
    Ok(())
}
pub(super) fn effects(frames: &[Vec<u8>]) -> TestResult<Vec<Vec<u8>>> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    let mut effects = Vec::new();
    for frame in frames {
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS {
                effects.push(delta.payload.to_vec());
            }
        }
    }
    Ok(effects)
}
pub(super) async fn capture(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    grant_frames: &[Vec<u8>],
    reconnect_frames: &[Vec<u8>],
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let hex = |bytes: &[u8]| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let path = std::env::var_os("OTERYN_SRV7_HANNA_WARDROBE_CAPTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(format!(
                "/tmp/srv7-hanna-wardrobe-{}.json",
                std::process::id()
            ))
        });
    let (appearance_context, cosmetics) = owner
        .appearance_context()
        .ok_or("actual appearance content context")?;
    let active = owner
        .active_generation
        .ok_or("actual active client artifact generation")?;
    let actor = admitted.runtime_actor.ok_or("actual source actor")?;
    let character = admitted
        .item_fence
        .ok_or("actual source character fence")?
        .character_id;
    let room_pin = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().client_artifact_digest(),
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().map_revision_digest(),
        )
    };
    let body = serde_json::json!({
        "qualification":"Actual source World Hanna and command7 material grant; original server snapshot/delta envelopes. Admission Accepted is not captured; this artifact alone does not prove Session::admit or terrain presentation.",
        "npc_identity":hex(&npc.identity),"npc_generation":npc.generation,
        "character_id":hex(character.as_bytes()),"actor_identity":hex(&actor.placement_identity()),"actor_generation":actor.actor_local_generation(),
        "session":hex(admitted.game_session_id.as_bytes()),"connection_generation":admitted.continuity.connection_generation,
        "selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),
        "content_revision":cosmetics.content_revision(),
        "client_artifact_digest":hex(&active.identity().client_artifact_digest()),
        "appearance_content_generation":hex(&appearance_context.content_generation),"cosmetics_digest":hex(&appearance_context.cosmetics_digest),
        "room_client_digest":hex(&room_pin.0),"server_artifact_digest":hex(&room_pin.1),"map_revision_digest":hex(&room_pin.2),
        "award":{"outfit_key":"oteryn:content.outfit.citizen_outfits","addons":2,"command_id":9},
        "reconnect_qualification":"Existing serve_admitted stream rebootstrap/duplicate replay; no new resume-generation claim",
        "grant_frames":grant_frames.iter().map(|frame|hex(frame)).collect::<Vec<_>>(),
        "reconnect_frames":reconnect_frames.iter().map(|frame|hex(frame)).collect::<Vec<_>>()
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&body)?)?;
    eprintln!("SRV7_HANNA_WARDROBE_CAPTURE {}", path.display());
    Ok(())
}
