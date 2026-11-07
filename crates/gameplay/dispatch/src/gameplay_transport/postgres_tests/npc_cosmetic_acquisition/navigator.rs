//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
#[path = "navigator/positive.rs"]
mod positive;
std::thread_local! { static OWNED:std::cell::Cell<bool>=const{std::cell::Cell::new(false)}; static SOURCE_ENTRY:std::cell::Cell<bool>=const{std::cell::Cell::new(false)}; }
pub(super) fn owned() -> bool {
    OWNED.with(std::cell::Cell::get)
}
pub(super) fn entry() -> (i32, i32, i16) {
    if SOURCE_ENTRY.with(std::cell::Cell::get) {
        (33504, 31253, 11)
    } else {
        ENTRY
    }
}
fn positive_scenario(source_entry: bool) -> TestResult {
    OWNED.with(|x| x.set(true));
    SOURCE_ENTRY.with(|x| x.set(source_entry));
    let result = super::scenario_for("oteryn:npc.navigator");
    OWNED.with(|x| x.set(false));
    SOURCE_ENTRY.with(|x| x.set(false));
    result
}
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (33640, 31379, 9);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_NAVIGATOR_BUNDLE").ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| npc.key == "oteryn:npc.navigator")
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
    let navigator = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.navigator")
        .ok_or("real Navigator source placement")?;
    assert_eq!(navigator.placements.len(), 1);
    assert_eq!(
        (
            navigator.placements[0].x,
            navigator.placements[0].y,
            navigator.placements[0].floor
        ),
        (33639, 31377, -9)
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
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:u16::try_from(entry().0)?,y:u16::try_from(entry().1)?,floor:i8::try_from(-entry().2)?}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}

/// Actual private-room source staging; neither an earned anchor nor a completed
/// Soul Net mission is claimed. There is deliberately no anchor operand.
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    if owned() {
        return positive::exercise(
            h,
            owner,
            admitted,
            npc,
            actor,
            SOURCE_ENTRY.with(std::cell::Cell::get),
        )
        .await;
    }
    let before: i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/navigator'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        before, 0,
        "fresh Navigator story must begin with an absent flag, not a synthetic stage3"
    );
    let anchors:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i14019' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(anchors, 0, "this counterexample has no Small Golden Anchor");
    super::super::grizzly_npc::wire::talk_paced(
        owner,
        admitted,
        npc,
        &[
            (1, "hi"),
            (2, "flou"),
            (3, "explain"),
            (4, "yes"),
            (5, "no"),
            (6, "yes"),
            (7, "yes"),
            (8, "yes"),
            (9, "helmet"),
            (10, "yes"),
        ],
    )
    .await?;
    let grants:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:appearance-grant/npc/navigator/deepling/addon2'").fetch_one(&h.pool).await?;
    assert_eq!(
        grants, 0,
        "Navigator must require return of the actual Small Golden Anchor before giving the helmet addon"
    );
    let acknowledged:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/navigator' AND value=4").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        acknowledged, 0,
        "missing anchor cannot acknowledge the final source exchange"
    );
    Ok(())
}
#[test]
fn srv7_navigator_actual_private_room_story_requires_returned_anchor() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| super::scenario_for("oteryn:npc.navigator").map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Navigator source test panicked")??;
    Ok(())
}

#[test]
fn srv7_navigator_historical_owned_anchor_real_exchange_cost_and_replay() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| positive_scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Navigator positive panicked")??;
    Ok(())
}
#[test]
fn srv7_navigator_same_anchor_native13941_entry_then_real_exchange() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| positive_scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Navigator source entry panicked")??;
    Ok(())
}
