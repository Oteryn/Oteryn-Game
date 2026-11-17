//! Native cooking ingredients from existing Bat/Chicken definitions and ordinary deaths.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc7_hot_native_bat_chicken_corpse_inputs() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-hot-corpses", || {
        super::quest_world_callbacks::scenario(
            "qengine-rewards-hot-corpses",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HOT_CORPSES POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_HOT_CORPSES POSTGRES_STARTED version={version}");
    {
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        for item in [4330, 4331, 4332, 4363, 4364, 4365, 6042, 6053] {
            let policy = native
                .item_policy_for_source_id(item)
                .ok_or_else(|| format!("missing cooking corpse policy {item}"))?;
            assert_eq!(
                serde_json::to_value(policy.record())?["admission"]["materializable"],
                true
            );
        }
    }
    let fishburger = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_policy_for_source_id(9088))
        .ok_or("missing Northern Fishburger custody policy")?;
    assert_eq!(
        serde_json::to_value(fishburger.record())?["admission"]["materializable"],
        true
    );
    assert_eq!(
        serde_json::to_value(fishburger.record())?["semantics"]["physical"]["value"]["weight"],
        serde_json::json!({"state":"KNOWN", "value":460})
    );
    super::questpc7_native_deaths::prepare_native_hunter(owner, a).await?;
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i9088".into(),
            count: 1,
        },
        12999,
        true,
    )
    .await?;
    let fishburger_quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i9088'")
        .bind(super::fixture::id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        fishburger_quantity, 1,
        "fixture MINT and native carried custody; no cooking reward claim"
    );

    for (index, creature, item) in [
        (0_u16, "oteryn:creature.chicken", 6042),
        (1_u16, "oteryn:creature.bat", 6053),
    ] {
        // Fixture travel terrain proves native death/custody, not a source-map spawn.
        super::questpc7_goblin::stalker_death(
            owner,
            a,
            creature,
            Position {
                x: 33294 + index,
                y: 31678,
                floor: -15,
            },
            12000 + u64::from(index) * 100,
        )
        .await?;
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.quantity=1 AND octet_length(g.corpse_ref)>0")
            .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        assert_eq!(
            count, 1,
            "one source corpse per native death despite duplicate notifications"
        );
    }
    assert_eq!(owner.runtime.lock().await.visible_corpses().len(), 2);
    let progress:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE quest_key='oteryn:quest.hot_cuisine_quest'").fetch_one(&h.pool).await?;
    assert_eq!(
        progress, 0,
        "ingredient support must not seed cooking progress"
    );
    Ok(())
}

pub(super) fn terrain() -> Vec<(Position, u16)> {
    (33292..=33296)
        .flat_map(|x| (31675..=31679).map(move |y| (Position { x, y, floor: -15 }, 100)))
        .collect()
}
