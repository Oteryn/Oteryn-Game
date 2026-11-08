//! Reuse questpc8's chosen smith cache through the existing native owners.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;
const QUEST: &str = "oteryn:quest.giant_smithhammer_quest";
const TRACK: &str = "oteryn:quest-progress/chosen/smith-cache/stage";

#[test]
fn questpc7_smith_native_guardian_exact_cache_completion() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-smith", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-smith-story",
            Position {
                x: 32776,
                y: 32254,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_SMITH POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
pub(super) fn terrain() -> Vec<(Position, u16)> {
    (32773..=32778)
        .flat_map(|x| (32252..=32256).map(move |y| (Position { x, y, floor: -8 }, 100)))
        .collect()
}
pub(super) fn items() -> Vec<(Position, u32)> {
    vec![(
        Position {
            x: 32777,
            y: 32254,
            floor: -8,
        },
        2472,
    )]
}
async fn guardian(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Option<crate::foundation::ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    let actors: Vec<_> = runtime
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .filter(|(_, p, s)| {
            s.is_none()
                && p.position()
                    == crate::foundation::MovementLocalPosition {
                        x: 32775,
                        y: 32254,
                        floor: 8,
                    }
        })
        .map(|(actor, _, _)| actor)
        .collect();
    assert!(
        actors.len() <= 1,
        "native refresh cannot duplicate the guardian"
    );
    Ok(actors.first().copied())
}
async fn quantity(h: &Harness, item: u32) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
        .bind(id(41).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_SMITH POSTGRES_STARTED version={version}");
    owner.premium_coordinator = None;
    assert!(
        oteryn_gameplay_quests::quest::world_callbacks::load()
            .map_err(debug)?
            .iter()
            .any(|c| c.key == "oteryn:quest-callback/authored-smith-cache"),
        "missing reused smith programme"
    );
    super::questpc7_native_deaths::prepare_native_hunter(owner, a).await?;
    assert_eq!(super::questpc7_ferumbras::value(owner, a, TRACK).await?, -1);
    assert!(guardian(owner).await?.is_none());
    super::questpc7_ferumbras::step(
        owner,
        a,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    )
    .await?;
    owner.refresh_quest_session(a.game_session_id, false).await;
    let target = guardian(owner)
        .await?
        .ok_or("native smith guardian was not admitted")?;
    owner.refresh_quest_session(a.game_session_id, false).await;
    assert_eq!(guardian(owner).await?, Some(target));
    assert_eq!(super::questpc7_ferumbras::value(owner, a, TRACK).await?, 1);
    let chest = Position {
        x: 32777,
        y: 32254,
        floor: -8,
    };
    assert_ne!(
        super::questpc7_ferumbras::use_at(owner, a, chest, 2472).await?,
        UseDisposition::Committed
    );
    let before = [
        quantity(h, 3208).await?,
        quantity(h, 3034).await?,
        quantity(h, 3031).await?,
    ];
    super::questpc7_native_deaths::lethal_creature(
        owner,
        a,
        target,
        45000,
        "oteryn:creature.giant_spider",
    )
    .await?;
    assert_eq!(
        super::questpc7_ferumbras::value(
            owner,
            a,
            "oteryn:quest-progress/chosen/smith-cache/kills"
        )
        .await?,
        1
    );
    assert_eq!(
        super::questpc7_ferumbras::use_at(owner, a, chest, 2472).await?,
        UseDisposition::Committed
    );
    let after = [
        quantity(h, 3208).await?,
        quantity(h, 3034).await?,
        quantity(h, 3031).await?,
    ];
    assert_eq!(
        [
            after[0] - before[0],
            after[1] - before[1],
            after[2] - before[2]
        ],
        [1, 1, 100]
    );
    assert_eq!(super::questpc7_ferumbras::value(owner, a, TRACK).await?, 3);
    let completed:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(completed, 1);
    let _ = super::questpc7_ferumbras::use_at(owner, a, chest, 2472).await?;
    assert_eq!(
        [
            quantity(h, 3208).await?,
            quantity(h, 3034).await?,
            quantity(h, 3031).await?
        ],
        after
    );
    Ok(())
}
