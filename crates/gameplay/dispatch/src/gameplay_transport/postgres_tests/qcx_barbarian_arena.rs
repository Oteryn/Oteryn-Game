use super::TestResult;
use super::arena_match::{NativeScenario, command, scenario_route};
use super::fixture::Harness;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::arena::{ArenaAction as A, ArenaDisposition as D};

#[test]
fn one_reward_choice_and_inscribed_goblet_per_tier() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_route(true, Some(NativeScenario::RewardParity)).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Barbarian Arena reward scenario panicked")??;
    eprintln!("QCX61 POSTGRES rewards duration={:?}", started.elapsed());
    Ok(())
}

pub(super) async fn check_rewards(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // Seed a completed arena receipt to exercise claims without thirty unrelated fights.
    command(owner, admitted, A::Status).await?;
    let session = admitted.game_session_id;
    let mut tx = harness.pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_arena_match_receipts
           (character_id,game_session_id,command_id,payload,before_state,after_state,reply)
         SELECT character_id,game_session_id,999999,payload,after_state,
                after_state || jsonb_build_object('completed',3,'phase',3),reply
         FROM game_arena_match_receipts WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_arena_matches(character_id,body,game_session_id,command_id)
         SELECT character_id,after_state,game_session_id,command_id
         FROM game_arena_match_receipts
         WHERE game_session_id=encode($1,'hex')::uuid AND command_id=999999
         ON CONFLICT (character_id) DO UPDATE SET body=EXCLUDED.body,command_id=EXCLUDED.command_id",
    )
    .bind(session.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // TibiaWiki Barbarian Arena Quest/Spoiler, curid 34102, oldid 668035.
    for (rank, first, second) in [(1, 3, 1), (2, 1, 2), (3, 2, 3)] {
        let claim = |category| A::ClaimReward {
            rank,
            category,
            choice: 1,
        };
        assert_eq!(
            command(owner, admitted, claim(first)).await?.disposition,
            D::Applied
        );
        assert_eq!(
            command(owner, admitted, claim(second)).await?.disposition,
            D::AlreadyClaimed
        );
        assert_eq!(
            command(owner, admitted, claim(4)).await?.disposition,
            D::Applied
        );
    }
    let goblets: Vec<(String, String)> = sqlx::query_as(
        "SELECT i.definition_production_key, t.text
         FROM game_item_instances i JOIN game_item_texts t USING (item_instance_id)
         WHERE i.definition_production_key IN ('oteryn:item.tibia.i5805','oteryn:item.tibia.i5806','oteryn:item.tibia.i5807')
         ORDER BY i.definition_production_key",
    )
    .fetch_all(&harness.pool)
    .await?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(
        admitted
            .item_fence
            .ok_or("item fence")?
            .character_id
            .as_bytes()
            .as_slice(),
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(
        goblets,
        [5805, 5806, 5807].map(|id| (
            format!("oteryn:item.tibia.i{id}"),
            format!("Awarded to {name}.")
        )),
    );
    Ok(())
}
