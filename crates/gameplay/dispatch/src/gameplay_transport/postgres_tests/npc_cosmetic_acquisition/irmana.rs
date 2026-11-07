//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (32952, 32110, 5);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_IRMANA_BUNDLE").ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| npc.key == "oteryn:npc.irmana")
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
    let irmana = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.irmana")
        .ok_or("real Irmana source placement")?;
    assert_eq!(irmana.placements.len(), 1);
    assert_eq!(
        (
            irmana.placements[0].x,
            irmana.placements[0].y,
            irmana.placements[0].floor
        ),
        (32952, 32109, -5)
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
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32952,y:32110,floor:-5}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}
/// Historical owned cash is only a payment operand; the real source dialogue
/// must create the order predecessor. No quest/appearance flags are seeded.
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    super::super::item_forge::support::seed_items(h, &[(90, "oteryn:item.tibia.i3043", 15, 0)])
        .await?;
    let sex: Option<i16> = sqlx::query_scalar(
        "SELECT sex FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let female = match sex {
        Some(1) => true,
        Some(2) => false,
        _ => return Err("genuine creation sex".into()),
    };
    let mut commands = vec![
        (1, "hi"),
        (2, "addon"),
        (3, "yes"),
        (4, if female { "dress" } else { "coat" }),
        (5, "yes"),
        (6, "yes"),
        (7, "150000"),
        (8, "addon"),
    ];
    if female {
        commands.push((9, "yes"));
    }
    super::super::grizzly_npc::wire::talk_paced(owner, admitted, npc, &commands).await?;
    let ordered:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_cosmetic_receipts WHERE character_id=encode($1,'hex')::uuid AND command_id=7 AND source_key=ANY($2::text[])").bind(id(41).as_slice()).bind(vec!["oteryn:npc-cosmetic/irmana/noble-female-1-prompt-5","oteryn:npc-cosmetic/irmana/noble-male-1-prompt-5"]).fetch_one(&h.pool).await?;
    assert_eq!(
        ordered, 1,
        "real source dialogue must commit order before diagnosing missing fee"
    );
    let purchased:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_cosmetic_receipts WHERE character_id=encode($1,'hex')::uuid AND source_key=ANY($2::text[])").bind(id(41).as_slice()).bind(vec!["oteryn:npc-cosmetic/irmana/noble-female-1-buy","oteryn:npc-cosmetic/irmana/noble-male-1-buy"]).fetch_one(&h.pool).await?;
    assert_eq!(
        purchased, 1,
        "actual ordinary cash-only source must commit paid pickup"
    );
    let remaining:i64=sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1").bind(id(90).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        remaining, 0,
        "exact fifteen source crystal coins pay150000cash"
    );
    Ok(())
}
