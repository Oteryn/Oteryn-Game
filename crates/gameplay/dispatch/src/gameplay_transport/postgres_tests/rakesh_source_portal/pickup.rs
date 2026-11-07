//! The actual source room's walkable path and existing corpse TAKE owner.
use super::super::fixture::Harness;
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::world_spatial::StepDirection;
pub(super) async fn pickup(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    body: [u8; 16],
) -> TestResult<[u8; 16]> {
    let actor = admitted.runtime_actor.ok_or("actual scoped player")?;
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|corpse| corpse.item.item_instance_id == body)
        .ok_or("actual bound source corpse")?;
    walk(owner, admitted, corpse.position, true).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fresh current source item fence")?
        .ok_or("fresh current source item fence")?
        .item_fence();
    if owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?
        .is_none()
    {
        super::super::item_look::seed_main_backpack(h, &[]).await?;
    }
    let contents = owner
        .root
        .read_corpse_contents(owner.character, body)
        .await?
        .ok_or("real corpse contents")?;
    let entry = contents.first().ok_or(
        "fixed genuine fixture death must roll a real source operand; no synthetic reward",
    )?;
    let item = entry.item.item_instance_id;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 900,
        item_fence: Some(fence),
    };
    let result = owner
        .take_corpse_entry(
            actor,
            command,
            ItemKey::Entity(corpse.identity),
            ItemKey::Instance(item),
        )
        .await
        .map_err(|error| format!("actual Instance corpse TAKE: {error:?}"))?;
    assert!(matches!(
        result,
        crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
    ));
    let custody:(i64,i64)=sqlx::query_as("SELECT(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        custody,
        (1, 0),
        "same actual minted operand moves from corpse to player's backpack"
    );
    let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_transfer_receipts WHERE source_item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    let _ = owner
        .take_corpse_entry(
            actor,
            command,
            ItemKey::Entity(corpse.identity),
            ItemKey::Instance(item),
        )
        .await;
    let after:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_transfer_receipts WHERE source_item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        before, after,
        "bare replay cannot duplicate actual loot custody"
    );
    Ok(item)
}
pub(super) async fn walk(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    body: crate::foundation::MovementLocalPosition,
    adjacent: bool,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actual scoped player")?;
    let start = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(start.floor, body.floor);
    let scope = owner.movement_cells.scope();
    let index = owner.movement_cells.index();
    let mut queue = std::collections::VecDeque::from([(start.x, start.y)]);
    let mut previous = std::collections::BTreeMap::new();
    previous.insert((start.x, start.y), None);
    let mut finish = None;
    while let Some(at) = queue.pop_front() {
        if (at.0 - body.x).abs() + (at.1 - body.y).abs() == i32::from(adjacent) {
            finish = Some(at);
            break;
        }
        for (dx, dy, direction) in [
            (0, -1, StepDirection::North),
            (1, 0, StepDirection::East),
            (0, 1, StepDirection::South),
            (-1, 0, StepDirection::West),
        ] {
            let next = (at.0 + dx, at.1 + dy);
            if (adjacent && next == (33043, 32986))
                || previous.contains_key(&next)
                || index
                    .lookup(
                        scope,
                        LogicalCell {
                            x: next.0,
                            y: next.1,
                            z: i32::from(start.floor),
                        },
                    )
                    .ok()
                    != Some(CollisionClass::Walkable)
            {
                continue;
            }
            previous.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    let mut at = finish.ok_or("actual source B3 path from landing to corpse neighbour")?;
    let mut path = Vec::new();
    while let Some(Some((from, direction))) = previous.get(&at) {
        path.push(*direction);
        at = *from;
    }
    path.reverse();
    for direction in path {
        let (step, duration) = owner
            .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
            .await;
        assert_eq!(
            step.disposition,
            crate::gameplay_transport::world_spatial::StepDisposition::Moved,
            "real source path step"
        );
        if let Some(duration) = duration {
            tokio::time::sleep(duration).await;
        }
    }
    if !adjacent {
        return Ok(());
    }
    let actual = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((actual.x - body.x).abs() + (actual.y - body.y).abs(), 1);
    Ok(())
}
