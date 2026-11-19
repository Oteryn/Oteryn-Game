use super::fixture::{Harness, debug};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests_fixtures as classic_quests,
};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) async fn after_greenhorn(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
) -> TestResult {
    super::arena_match_fixtures::finish_barbarian_test(owner, admitted.game_session_id).await?;
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![super::qcx61_halvar_fixtures::callback()?])
        .map_err(|_| "Arena native callback")?;
    {
        // Model the walk back to Halvar without sending a client Arena Leave.
        let mut runtime = owner.runtime.lock().await;
        let current = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .relocate_arena_player(
                actor,
                admitted.game_session_id,
                current,
                MovementLocalPosition {
                    x: 32246,
                    y: 31098,
                    floor: 7,
                },
            )
            .map_err(debug)?;
    }
    let [halvar] = classic_quests::install(owner, admitted, ["oteryn:npc.halvar"]).await?;
    classic_quests::talk(owner, admitted, halvar, 20300, "hi").await?;
    let quote = classic_quests::talk(owner, admitted, halvar, 20301, "fight").await?;
    assert!(
        quote
            .iter()
            .any(|line| line.contains("Scrapper") && line.contains("5000"))
    );
    let paid = classic_quests::talk(owner, admitted, halvar, 20302, "yes").await?;
    assert!(
        paid.iter().any(|line| line.contains("pass the door")),
        "{paid:?}"
    );

    owner.clock_origin -= std::time::Duration::from_secs(2);
    let (step, _) = owner
        .ordinary_paced_step(
            actor,
            admitted.game_session_id,
            StepDirection::North,
            None,
            None,
        )
        .await;
    let landing = step
        .moved_to
        .ok_or("Scrapper forcefield step")?
        .actor_position;
    let (x, y) = crate::durability::arena_match::donor::CENTERS[0];
    assert_eq!((landing.x, landing.y, landing.floor), (x, y, 7));

    let (completed, active, pit): (i32, i32, i32) = sqlx::query_as(
        "SELECT (body->>'completed')::int, (body->>'rank')::int, (body->>'pit')::int
         FROM game_character_arena_matches",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((completed, active, pit), (1, 2, 1));
    let fees: Vec<i64> = sqlx::query_scalar(
        "SELECT fee_gold_units FROM game_item_fee_burns WHERE cause_kind=17 ORDER BY fee_gold_units",
    ).fetch_all(&h.pool).await?;
    assert_eq!(fees, [1000, 5000]);
    Ok(())
}
