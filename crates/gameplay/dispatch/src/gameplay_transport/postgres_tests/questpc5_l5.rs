//! L5 paired plate departure, through the real STEP command.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

pub(super) async fn exercise_plate(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    actor: crate::foundation::ExactActorRef,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    trigger: Position,
) -> TestResult {
    use super::fixture::debug;
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
        decode_step_result, encode_step_intent,
    };
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    owner.runtime.lock().await.bind_attacker_lease(actor, admitted.game_session_id,
        crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        ).map_err(debug)?,
    ).map_err(debug)?;
    let before = owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)?;
    let count = || sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid"
    ).bind(fence.character_id.as_bytes().as_slice());
    let receipts = count().fetch_one(&h.pool).await?;
    let map = owner.bundle_map().ok_or("map")?;
    let passage = &callback.passages[0];
    let stairs = crate::map::view::placement_key(
        crate::map::overlay::TilePos { x: passage.position.x, y: passage.position.y, floor: passage.position.floor }, 1,
    ).ok_or("stairs key")?;
    let plate = crate::map::view::placement_key(
        crate::map::overlay::TilePos { x: trigger.x, y: trigger.y, floor: trigger.floor }, 1,
    ).ok_or("plate key")?;
    let appearance = |key| -> TestResult<u16> {
        let (position, ordinal) = crate::map::view::placement_of(key).ok_or("placement")?;
        let tile = map.overlay.base().tile(position.x, position.y, position.floor).ok_or("tile")?;
        let id = tile.ids().iter().zip(tile.depths()).filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal)).map(|(id, _)| *id).ok_or("palette")?;
        Ok(map.facts.base_entry(position, ordinal, id).ok_or("facts")?.appearance_id)
    };
    for (command, direction, expected_stairs, expected_plate) in [
        (1, StepDirection::East, 7767, 430),
        (2, StepDirection::West, 429, 431),
        (3, StepDirection::East, 7767, 430),
        (4, StepDirection::West, 429, 431),
    ] {
        let response = super::quest_world_callbacks::wire(owner, admitted, command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(direction)).await?;
        assert_eq!(decode_step_result(&response).map_err(debug)?, StepDisposition::Moved);
        assert_eq!(appearance(stairs)?, expected_stairs, "stairs after command {command}");
        assert_eq!(appearance(plate)?, expected_plate, "plate after command {command}");
    }
    let after = owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)?;
    assert_eq!(before, after);
    assert_eq!(count().fetch_one(&h.pool).await?, receipts);
    Ok(())
}

#[test]
fn questpc5_l5_thais_plate_departure_restores_stairs_without_quest_writes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("l5-thais-plate", || {
        super::quest_world_callbacks::scenario(
            "l5-thais-plate",
            Position { x: 32224, y: 32282, floor: -9 },
            false,
        )
    })?;
    eprintln!("POSTGRES_CASE_COMPLETED questpc5_l5_thais_plate_departure_restores_stairs_without_quest_writes duration={:?}", started.elapsed());
    Ok(())
}
