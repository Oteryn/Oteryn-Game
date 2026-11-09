use super::*;
use oteryn_protocol_oteryn::cyclopedia::atlas::{Atlas, Query};
pub(super) async fn verify(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let guide_x = {
        let mut runtime = owner.runtime.lock().await;
        let mut guide = None;
        for x in 11..=13 {
            if runtime
                .place_dynamic_npc(
                    "oteryn:npc.cipfried",
                    crate::foundation::NpcPlacement {
                        floor: -7,
                        x,
                        y: 2,
                        direction: crate::foundation::MovementFacing::South,
                    },
                )
                .is_ok()
            {
                guide = Some(x);
                break;
            }
        }
        guide.ok_or("guide fixture position")?
    };
    let payload = Query::default().encode().map_err(debug)?;
    let mut unselected = *admitted;
    unselected.continuity.selected_capabilities = SelectedCapabilities::NONE;
    assert!(matches!(
        owner.cyclopedia_wire(unselected, 2, 72, &payload).await,
        GuildReply::Rejected
    ));
    let id = admitted.continuity.next_command_id;
    let (status, bytes) = support::exchange(owner, admitted, id, 72, &payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let atlas = Atlas::decode(&bytes).map_err(debug)?;
    assert_eq!(atlas.query.floor, -7);
    assert!(atlas.cells.contains(&2));
    assert!(atlas.areas.iter().any(|a| a.id == 2 && a.parent == 1));
    assert!(
        atlas.markers.iter().any(|m| m.kind == 1
            && m.label == "oteryn:npc.cipfried"
            && m.x == guide_x
            && m.y == 2)
    );
    assert!(atlas.markers.iter().filter(|m| m.kind == 3).all(|m| {
        atlas
            .progress
            .iter()
            .flat_map(|p| &p.visible)
            .any(|p| p.0 == m.x && p.1 == m.y && p.2 == atlas.query.floor)
    }));
    owner
        .runtime
        .lock()
        .await
        .remove_dynamic_npc("oteryn:npc.cipfried")
        .map_err(debug)?;
    // Public map browsing also works from a tile without exploration membership.
    {
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let mut runtime = owner.runtime.lock().await;
        let view = crate::gameplay_transport::bundle_map_position(
            &mut runtime,
            owner.movement_cells(),
            actor,
        )
        .ok_or("map position")?;
        let base = view.map.overlay.base();
        let (floor, x, y, _) = base
            .tiles()
            .find(|(floor, x, y, tile)| {
                tile.walkable() && base.subareas.resolve(*x, *y, *floor).is_none()
            })
            .ok_or("uncovered fixture tile")?;
        let target = MovementLocalPosition {
            x: i32::from(x),
            y: i32::from(y),
            floor: -i16::from(floor),
        };
        let mut movement = runtime.borrow_movement_position();
        let mut current = movement.read(actor).map_err(debug)?;
        while current.position() != target {
            let at = current.position();
            let next = if at.x != target.x {
                MovementLocalPosition {
                    x: at.x + (target.x - at.x).signum(),
                    ..at
                }
            } else {
                MovementLocalPosition {
                    y: at.y + (target.y - at.y).signum(),
                    ..at
                }
            };
            current = movement.commit_cardinal(current, next).map_err(debug)?;
        }
    }
    let id = admitted.continuity.next_command_id;
    let (status, bytes) = support::exchange(owner, admitted, id, 72, &payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let uncovered = Atlas::decode(&bytes).map_err(debug)?;
    assert!(uncovered.progress.is_none());
    assert!(uncovered.cells.iter().any(|id| *id != 0));
    let mut stale = *admitted;
    if let Some(f) = &mut stale.item_fence {
        f.character_lease_generation += 1;
    }
    assert!(matches!(
        owner.cyclopedia_wire(stale, 2, 72, &payload).await,
        GuildReply::Rejected
    ));
    eprintln!(
        "POSTGRES_EXECUTED T079 atlas real caller, NPC, membership, POI visibility and fences"
    );
    Ok(())
}
