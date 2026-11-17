//! The two donor fish trades and their refusal through live NPC talk.
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const POSITION: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const TROUT: &str = "oteryn:item.tibia.i7158";
const PERCH: &str = "oteryn:item.tibia.i7159";
const SHARD: &str = "oteryn:item.tibia.i7290";

#[test]
fn sinatuki_exchange_postgres_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("sinatuki-exchange", || {
        super::quest_world_callbacks::scenario("sinatuki-exchange", POSITION, false)
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED sinatuki_exchange_postgres_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    for (track, value) in [
        (
            "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
            3_i64,
        ),
        (
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
            5_i64,
        ),
    ] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)")
            .bind(id(41).as_slice()).bind(track)
            .bind("oteryn:quest.the_ice_islands_quest").bind(value).execute(&mut *fixture).await?;
    }
    fixture.commit().await?;
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.sinatuki"]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    for (ordinal, item, count) in [(760, TROUT, 60), (761, TROUT, 40), (762, PERCH, 99)] {
        super::quest_world_callbacks::input_item(
            owner,
            fence,
            &Offering {
                position: Position {
                    x: POSITION.x - 1,
                    ..POSITION
                },
                item: item.into(),
                count,
            },
            ordinal,
            true,
        )
        .await?;
    }
    super::classic_quests::talk(owner, admitted, npc, 39700, "hi").await?;
    for (command, track, minimum) in [
        (
            39710,
            "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
            3_i64,
        ),
        (
            39711,
            "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
            5_i64,
        ),
    ] {
        set_progress(h, track, minimum - 1).await?;
        assert!(
            super::classic_quests::talk(owner, admitted, npc, command, "nupi")
                .await?
                .is_empty()
        );
        assert_inventory(h, [100, 99, 0]).await?;
        set_progress(h, track, minimum).await?;
    }
    for (command, reply, expected) in [
        (
            39701,
            "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>",
            [0, 99, 5],
        ),
        (
            39702,
            "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>",
            [0, 0, 10],
        ),
        (39703, "Kisavuta! <giggles>", [0, 98, 10]),
    ] {
        if command == 39703 {
            super::quest_world_callbacks::input_item(
                owner,
                fence,
                &Offering {
                    position: Position {
                        x: POSITION.x - 1,
                        ..POSITION
                    },
                    item: PERCH.into(),
                    count: 98,
                },
                763,
                true,
            )
            .await?;
        }
        let lines = super::classic_quests::talk(owner, admitted, npc, command, "nupi").await?;
        assert_eq!(lines, [reply]);
        assert_inventory(h, expected).await?;
    }
    Ok(())
}

async fn set_progress(h: &Harness, track: &str, value: i64) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=$3 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(track).bind(value).execute(&mut *fixture).await?;
    fixture.commit().await?;
    Ok(())
}

async fn assert_inventory(h: &Harness, expected: [i64; 3]) -> TestResult {
    for (item, quantity) in [TROUT, PERCH, SHARD].into_iter().zip(expected) {
        let actual: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i
             JOIN game_item_container_entries e USING(item_instance_id)
             WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2",
        ).bind(id(41).as_slice()).bind(item).fetch_one(&h.pool).await?;
        assert_eq!(actual, quantity, "{item}");
    }
    Ok(())
}
