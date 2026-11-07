//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (33257, 31840, 7);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_SANDRA_BUNDLE").ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| npc.key == "oteryn:npc.sandra")
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
    let sandra = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.sandra")
        .ok_or("real Sandra source placement")?;
    assert_eq!(sandra.placements.len(), 1);
    assert_eq!(
        (
            sandra.placements[0].x,
            sandra.placements[0].y,
            sandra.placements[0].floor
        ),
        (33257, 31839, -7)
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
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:33257,y:31840,floor:-7}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}
/// Controlled historical source-owned flask operand; actual Start dialogue owns
/// all quest flags. This fixture does not claim to execute the potion producer.
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    super::super::item_forge::support::seed_items(h, &[(90, "oteryn:item.tibia.i283", 100, 0)])
        .await?;
    super::super::grizzly_npc::wire::talk_paced(
        owner,
        admitted,
        npc,
        &[
            (1, "hi"),
            (2, "ticket"),
            (3, "yes"),
            (4, "yes"),
            (5, "ticket"),
            (6, "yes"),
        ],
    )
    .await?;
    // Accepted command envelopes alone do not prove the source programme ran.
    // Prove its actual Start predecessor before interpreting a missing ticket.
    let start_receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_cosmetic_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=4 AND source_key='oteryn:npc-cosmetic/sandra/belt-start'")
        .bind(id(41).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        start_receipts, 1,
        "actual native command4 must commit source Sandra Start"
    );
    let start_tracks: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key")
        .bind(id(41).as_slice()).bind(vec!["oteryn:quest-progress/outfit_quest/default_start","oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt"]).fetch_all(&h.pool).await?;
    assert_eq!(
        start_tracks,
        vec![
            ("oteryn:quest-progress/outfit_quest/default_start".into(), 1),
            (
                "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt".into(),
                1
            )
        ],
        "source dialogue must create both genuine predecessor facts"
    );
    let tickets:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i5957' AND e.character_id=encode($1,'hex')::uuid AND i.quantity=1").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        tickets, 1,
        "actual source Sandra deposit must mint its native backpack ticket"
    );
    let remaining:i64=sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1").bind(id(90).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        remaining, 0,
        "exact100same-kind vials are consumed atomically"
    );
    Ok(())
}
