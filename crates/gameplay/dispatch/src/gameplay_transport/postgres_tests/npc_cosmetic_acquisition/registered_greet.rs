//! Actual original source scenes; no earned quest predecessor is initialized.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) fn entry(npc: &str) -> (i32, i32, i16) {
    match npc {
        "oteryn:npc.bron" => (32365, 31628, 7),
        "oteryn:npc.bozo" => (32312, 32183, 7),
        "oteryn:npc.a_majestic_warwolf" => (33354, 31991, 8),
        _ => unreachable!("finite registered greet source"),
    }
}
pub(super) fn world(
    npc_key: &str,
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_REGISTERED_GREET_BUNDLE")
            .ok_or("original registered greet scenes")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let n = b
        .npcs
        .npcs
        .iter()
        .find(|n| n.key == npc_key)
        .ok_or("actual registered greet source NPC")?;
    let expected = match npc_key {
        "oteryn:npc.bron" => (32366, 31628, -7),
        "oteryn:npc.bozo" => (32313, 32183, -7),
        "oteryn:npc.a_majestic_warwolf" => (33355, 31991, -8),
        _ => return Err("unknown registered greet source".into()),
    };
    assert_eq!(n.placements.len(), 1);
    assert_eq!(
        (n.placements[0].x, n.placements[0].y, n.placements[0].floor),
        expected
    );
    let table = NpcPlacementTable {
        npcs: vec![NpcPlacements {
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
        }],
    };
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let (x, y, z) = entry(npc_key);
    let index = native
        .item_index()
        .ok_or("registered greet native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:u16::try_from(x).map_err(debug)?,y:u16::try_from(y).map_err(debug)?,floor:i8::try_from(-z).map_err(debug)?}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{let rev=index.revision(key)?;let p=native.item_policy(key,rev).map(|p|p.record());Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:p.and_then(|p|p.attributes.blocks_movement),blocks_projectile:p.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:p.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:p.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})}).map_err(debug)?;
    Ok((boot, table))
}
fn conversations(
    frames: &[Vec<u8>],
) -> TestResult<Vec<oteryn_protocol_oteryn::npc_service::NpcConversation>> {
    let mut replies = Vec::new();
    for frame in frames {
        let e = crate::foundation::decode_wire_envelope(frame).map_err(debug)?;
        if e.message_type() != crate::foundation::MessageType::StateDelta {
            continue;
        }
        let d = oteryn_protocol_oteryn::decode_state_delta(e.payload()).map_err(debug)?;
        if d.domain_id == oteryn_protocol_oteryn::npc_service::STATE_DOMAIN_NPC_CONVERSATION {
            replies.push(
                oteryn_protocol_oteryn::npc_service::decode_npc_conversation(d.payload)
                    .map_err(debug)?,
            );
        }
    }
    Ok(replies)
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    key: &str,
) -> TestResult {
    let commands: &[(u64, &str)] = if key == "oteryn:npc.bron" {
        &[(1, "hi"), (2, "outfit"), (3, "yes"), (4, "yes")]
    } else {
        &[(1, "hi")]
    };
    let frames =
        super::super::grizzly_npc::wire::talk_appearance(owner, admitted, npc, commands).await?;
    let replies = conversations(&frames)?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    match key {
        "oteryn:npc.bron" => {
            assert!(
                replies.iter().any(|r| r.open.is_some()
                    && r.lines
                        .iter()
                        .any(|s| s == &format!("Welcome to my humble hut, {name}."))),
                "Bron must use genuine default greeting before any fighting spirit was delivered"
            );
            let actual:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/quest/u7_8/barbarian_outfits/barbarian_addon'").bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
            assert_eq!(
                actual,
                Some(1),
                "actual Bron dialogue creates the first Barbarian acquisition predecessor"
            );
        }
        "oteryn:npc.a_majestic_warwolf" => {
            assert!(
                replies
                    .iter()
                    .any(|r| r.open.is_none() && r.lines.iter().any(|s| s == "GRRRRRRRRRRRRR")),
                "unqualified fresh player must receive registered growl and refusal"
            );
            assert!(
                !replies.iter().any(|r| r.open.is_some()),
                "Warwolf refuses conversation before actual stage9"
            );
        }
        "oteryn:npc.bozo" => {
            let sex: i16 = sqlx::query_scalar(
                "SELECT sex FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
            )
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                sex, 1,
                "female was authenticated at initial Character creation"
            );
            assert!(
                replies.iter().any(|r| r.open.is_some()
                    && r.lines.iter().any(|s| s
                        == &format!(
                            "Hello, hello, hello, little lady {name}! What brings you {{here}}?"
                        ))),
                "actual female Character receives Bozo registered female greeting"
            );
        }
        _ => return Err("unknown registered greet test".into()),
    }
    Ok(())
}
