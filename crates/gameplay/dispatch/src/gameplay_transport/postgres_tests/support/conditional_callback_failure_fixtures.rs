use super::TestResult;

use super::fixture::{Harness, debug};

use super::quest_world_callbacks_fixtures::{local, wire};

use crate::durability::character_progression::CurrentCharacterGameplayFence;

use crate::foundation::ExactActorRef;

use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    expected_destination: Position,
) -> TestResult {
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED conditional_quest_failure server_version={version}");
    let payload = encode_step_intent(StepDirection::East);
    let reply = wire(owner, admitted, 1, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(after.position(), local(expected_destination));
    assert_eq!(wire(owner, admitted, 1, 1, payload).await?, reply);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        after
    );
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let after_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(after_receipts, receipts);
    Ok(())
}
