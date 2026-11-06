//! Generic movement regression: entry opens static walls, departure restores them.
use super::{TestResult, fixture::debug};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
const TRIGGER: Position = Position {
    x: 33293,
    y: 31680,
    floor: -15,
};

#[test]
fn committed_departure_restores_static_passages_without_quest_writes() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            let started = std::time::Instant::now();
            super::quest_world_callbacks::scenario(
                "departure-map-regression", TRIGGER, false,
            ).map_err(|error| error.to_string())?;
            println!("POSTGRES_CASE_COMPLETED committed_departure_restores_static_passages_without_quest_writes duration={:?}", started.elapsed());
            Ok(())
        })?.join().map_err(|_| "departure scenario thread panicked")??;
    Ok(())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    actor: crate::foundation::ExactActorRef,
    callback: &Callback,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
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
    let receipts = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid"
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let map = owner.bundle_map().ok_or("map")?;
    let passages = callback
        .passages
        .iter()
        .map(|p| {
            oteryn_world_bundle::bundle::placement_key(
                p.position.floor,
                p.position.x,
                p.position.y,
                1,
            )
            .ok_or("passage placement")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(passages.len(), 5);
    let root = oteryn_world_bundle::bundle::placement_key(TRIGGER.floor, TRIGGER.x, TRIGGER.y, 1)
        .ok_or("root placement")?;
    let mut pressed = None;
    for (command, direction, hidden) in [
        (1, StepDirection::East, true),
        (2, StepDirection::West, false),
        (3, StepDirection::East, true),
        (4, StepDirection::West, false),
    ] {
        let response = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&response).map_err(debug)?,
            StepDisposition::Moved
        );
        for &key in &passages {
            assert_eq!(
                map.facts.base_hidden(key),
                hidden,
                "passage after command {command}"
            );
        }
        assert_eq!(map.facts.object_revision(root), command);
        if command == 3 {
            pressed = Some(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?,
            );
        }
    }
    let entered = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let departure = owner
        .states
        .quest_callbacks
        .fixture
        .get()
        .ok_or("fixture callbacks")?
        .iter()
        .find(|callback| callback.key.ends_with("departure-map-regression"))
        .ok_or("departure callback")?;
    // A different projection can supersede a cancelled static departure.
    // Reconciliation must retire that old event without overwriting newer map facts.
    owner.states.chat.admit(
        admitted.game_session_id,
        "Departure Tester",
        owner.owner_now(),
    );
    assert!(owner.states.chat.drain(admitted.game_session_id).is_empty());
    assert!(owner.queue_departure_for_test(
        actor,
        admitted.game_session_id,
        pressed.ok_or("pressed snapshot")?,
        entered,
        departure,
        (root, 3)
    ));
    assert!(
        owner
            .reconcile_quest_callbacks(admitted.game_session_id)
            .await,
        "superseded static departure must not pin the session"
    );
    assert!(
        owner
            .states
            .chat
            .drain(admitted.game_session_id)
            .iter()
            .any(|line| {
                matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
            if text == "The destination entry was processed.")
            }),
        "departure recovery must continue destination entry"
    );
    assert_eq!(map.facts.object_revision(root), 4);
    for &key in &passages {
        assert!(!map.facts.base_hidden(key));
    }
    let after = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid"
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, receipts);
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    Ok(())
}
