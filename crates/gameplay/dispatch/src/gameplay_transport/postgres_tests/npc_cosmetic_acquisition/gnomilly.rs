//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
#[path = "pipeline.rs"]
mod pipeline;
std::thread_local! { static FULL: std::cell::Cell<bool> = const {std::cell::Cell::new(false)}; }
pub(super) fn full() -> bool {
    FULL.with(std::cell::Cell::get)
}
pub(super) fn full_scenario() -> TestResult {
    FULL.with(|flag| flag.set(true));
    let result = super::scenario_for("oteryn:npc.gnomilly");
    FULL.with(|flag| flag.set(false));
    result
}

use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (32243, 32597, 9);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_SUPPORT_BUNDLE").ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| {
                npc.key == "oteryn:npc.gnomilly" || (full() && npc.key == "oteryn:npc.gnommander")
            })
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
    let gnomilly = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.gnomilly")
        .ok_or("real Gnomilly source placement")?;
    assert_eq!(gnomilly.placements.len(), 1);
    assert_eq!(
        (
            gnomilly.placements[0].x,
            gnomilly.placements[0].y,
            gnomilly.placements[0].floor
        ),
        (32242, 32599, -9)
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
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32243,y:32597,floor:-9}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}

/// Source gnomilly.lua125–141: no assigned kill task reports the honest refusal.
/// This deliberately creates no task/Fame/kill/XP predecessor facts.
pub(super) async fn exercise(
    _h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    if full() {
        return pipeline::exercise(_h, owner, admitted, npc).await;
    }
    use crate::gameplay_transport::connection::UseCommand;
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let actor = admitted.runtime_actor.ok_or("actual Gnomilly player")?;
    let mut final_lines = Vec::new();
    for (command_id, text) in [(1, "hi"), (2, "report"), (3, "killing")] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("NPC source clock")?;
        let outcome = owner
            .npc_talk(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id,
                    item_fence: admitted.item_fence,
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(
            outcome.disposition,
            NpcIntentDisposition::Ok,
            "Gnomilly:{text}"
        );
        assert!(!outcome.unknown, "Gnomilly source topic:{text}");
        final_lines = outcome
            .conversation
            .ok_or("Gnomilly native dialogue")?
            .lines;
    }
    assert_eq!(
        final_lines,
        vec!["You have not started that mission."],
        "actual source NPC must expose report→killing before any assignment/backend assertion"
    );
    Ok(())
}
