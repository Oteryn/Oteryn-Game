//! Source Item admission through real mint and carried custody owners, without use effects.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const IDS: [u32; 6] = [43501, 43502, 43503, 43504, 43899, 43900];

#[test]
fn rotten_source_items_materialize_into_exact_character_custody() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("ROTTEN_SOURCE_ITEMS_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc2-rotten-source-items", || {
        super::quest_world_callbacks::scenario(
            "questpc2-rotten-source-items",
            Position { x: 33294, y: 31677, floor: -15 },
            false,
        )
    })?;
    eprintln!("ROTTEN_SOURCE_ITEMS POSTGRES_CASE_COMPLETED {:?}", started.elapsed());
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool).await?;
    eprintln!("ROTTEN_SOURCE_ITEMS POSTGRES_CASE_OBSERVED server_version_num={version}");
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "current Item fence unavailable")?.ok_or("current Item fence missing")?;
    let keys: Vec<String> = IDS.iter().map(|id| format!("oteryn:item.tibia.i{id}")).collect();
    for (ordinal, key) in keys.iter().enumerate() {
        let offering = Offering {
            position: Position { x: 33293, y: 31677, floor: -15 },
            item: key.clone(), count: 1,
        };
        super::quest_world_callbacks::input_item(
            owner, fence.item_fence(), &offering, 800 + ordinal as u64, true,
        ).await.map_err(|error| format!("{key}: real mint/custody refused: {error}"))?;
    }
    let (distinct, quantity): (i64, i64) = sqlx::query_as(
        "SELECT count(DISTINCT i.definition_production_key), coalesce(sum(i.quantity),0)::bigint \
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
         JOIN game_item_instances i ON i.item_instance_id=c.item \
         WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[])",
    ).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(&keys).fetch_one(&h.pool).await?;
    assert_eq!((distinct, quantity), (6, 6));
    eprintln!("ROTTEN_SOURCE_ITEMS POSTGRES_CASE_OBSERVED distinct={distinct} quantity={quantity}");
    Ok(())
}
