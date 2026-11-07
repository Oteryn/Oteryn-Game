//! Source corpse/loot custody, independent of any tutorial progress or reward.
use super::*;

#[test]
fn questpc3_tutorial_native_corpse_cockroach() -> TestResult {
    physical("oteryn:creature.cockroach")
}
#[test]
fn questpc3_tutorial_native_corpse_rabbit() -> TestResult {
    physical("oteryn:creature.rabbit")
}
#[test]
fn questpc3_tutorial_native_corpse_deer() -> TestResult {
    physical("oteryn:creature.deer")
}

fn physical(creature: &'static str) -> TestResult {
    run_counter(
        creature,
        "oteryn:quest-progress/native-kills/beginning/cockroaches",
        "oteryn:quest-transition/native-kills/beginning",
        "unused",
        CounterMode::Physical,
    )
}

pub(super) async fn assert_custody(h: &Harness, creature: &str, deaths: u64) -> TestResult {
    let corpse = match creature {
        "oteryn:creature.cockroach" => "oteryn:item.tibia.i7763",
        "oteryn:creature.rabbit" => "oteryn:item.tibia.i6017",
        "oteryn:creature.deer" => "oteryn:item.tibia.i5970",
        _ => return Err("unknown physical tutorial creature".into()),
    };
    let bodies: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(corpse).fetch_one(&h.pool).await?;
    assert_eq!(
        bodies,
        i64::try_from(deaths)?,
        "each native death must mint its source corpse once"
    );
    let loot: Vec<(String,i64,i64)> = sqlx::query_as("SELECT i.definition_production_key,sum(i.quantity)::bigint,count(*) FROM game_item_corpse_container_entries e JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id GROUP BY i.definition_production_key ORDER BY 1").fetch_all(&h.pool).await?;
    if creature == "oteryn:creature.cockroach" {
        assert_eq!(
            loot,
            vec![(
                "oteryn:item.tibia.i7882".into(),
                i64::try_from(deaths)?,
                i64::try_from(deaths)?
            )],
            "guaranteed nonstackable legs live in actual corpses"
        );
    } else {
        for (key, quantity, _) in loot {
            let maximum = match key.as_str() {
                "oteryn:item.tibia.i3577" if creature.ends_with("deer") => 4,
                "oteryn:item.tibia.i3577" => 2,
                "oteryn:item.tibia.i3582" if creature.ends_with("deer") => 2,
                "oteryn:item.tibia.i10297" if creature.ends_with("deer") => 1,
                _ => return Err("loot outside the complete source animal table".into()),
            };
            assert!((1..=i64::try_from(deaths)? * maximum).contains(&quantity));
        }
    }
    let xp: i64 =
        sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        xp, 0,
        "source animals grant0XP; tutorial reward is not a kill reward"
    );
    let claims: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        claims, 0,
        "ordinary loot grants no tutorial stage or reward"
    );
    Ok(())
}
