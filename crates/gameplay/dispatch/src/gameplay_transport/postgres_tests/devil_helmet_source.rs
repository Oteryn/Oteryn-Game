//! UID25030 source plate, temporary435, departure cleanup and native USE.
//! Reuses questpc6 pressure_plate_sewer.rs source cases through generic callbacks.
use super::{TestResult, fixture::debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    actor: ExactActorRef,
    trigger: Position,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let map = owner.bundle_map().ok_or("map")?;
    let position = TilePos {
        x: 32482,
        y: 32170,
        floor: -14,
    };
    let count = || -> TestResult<usize> {
        Ok(
            crate::map::view::compose_tile(&map.overlay, &map.facts, position)
                .map_err(debug)?
                .ok_or("sewer tile")?
                .entries
                .iter()
                .filter(|e| e.facts.appearance_id == 435)
                .count(),
        )
    };
    assert_eq!(count()?, usize::from(name.ends_with("existing")));
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    for (command, direction, expected) in [
        (1, StepDirection::East, 1),
        (2, StepDirection::West, 0),
        (3, StepDirection::East, 1),
        (4, StepDirection::West, 0),
    ] {
        let payload = encode_step_intent(direction);
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command, 1, payload.clone())
                .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            count()?,
            if name.ends_with("unrelated") {
                0
            } else {
                expected
            }
        );
        let entries = map.facts.movement_items(position);
        assert_eq!(
            super::quest_world_callbacks::wire(owner, admitted, command, 1, payload).await?,
            reply
        );
        assert_eq!(
            map.facts.movement_items(position),
            entries,
            "STEP replay retains exact object identity"
        );
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    }
    if !name.ends_with("unrelated") {
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            5,
            1,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(count()?, 1);
        let results = super::source_dropdown::wire_created_grate(owner, *admitted).await?;
        if name.ends_with("blocked") {
            assert!(results.iter().all(|d| *d == UseDisposition::NotPossible));
            assert_eq!(count()?, 1);
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                super::quest_world_callbacks::local(trigger)
            );
        } else {
            assert_eq!(results[0], UseDisposition::Committed);
            assert_ne!(results[1], UseDisposition::Committed);
            assert_eq!(count()?, 0, "USE departure removes the temporary grate");
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                super::quest_world_callbacks::local(Position {
                    x: 32482,
                    y: 32170,
                    floor: -15
                })
            );
        }
    }
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}
