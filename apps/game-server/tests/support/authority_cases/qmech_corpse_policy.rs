//! The full migration chain preserves Corruptor and ordinary corpse timings.
use crate::bestiary_postgres_harness as f;

#[test]
fn qmech_corpse_policy_preserves_corruptor_and_source_durations() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let h = f::Harness::create(admin, "qmech_corpse_policy", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "QMECH corpse policy worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    for (root, chain, durations) in [
        (
            52113,
            vec![52113, 52114, 52115, 52116],
            vec![600_000_i64, 600_000, 10_000, 600_000],
        ),
        (
            54678,
            vec![54678, 54679, 54680, 54681],
            vec![10_000, 300_000, 300_000, 60_000],
        ),
        (
            33859,
            vec![33859, 33858, 33857, 33856],
            vec![10_000, 900_000, 600_000, 600_000],
        ),
    ] {
        let key = format!("oteryn:item.tibia.i{root}");
        let actual: (Vec<i32>, Vec<i64>) = sqlx::query_as(
            "SELECT game_summer_corpse_chain($1), game_source_corpse_durations_v1($1)",
        )
        .bind(&key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(actual, (chain, durations), "source corpse {root}");
    }
    let packet: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../imports/crystalserver/raid-items-201189f.json"
    )))?;
    let native: serde_json::Value = serde_json::from_slice(&std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/items/definitions/spell-native-profiles.json"),
    )?)?;
    let records = packet["records"].as_array().ok_or("raid source records")?;
    let native_records = native["records"].as_array().ok_or("native Item profiles")?;
    for (id, stages) in [(9607, 4), (10337, 4), (31599, 4), (50029, 3)] {
        let root = format!("oteryn:item.tibia.i{id}");
        let (chain, durations): (Vec<i32>, Vec<i64>) = sqlx::query_as(
            "SELECT game_summer_corpse_chain($1), game_source_corpse_durations_v1($1)",
        )
        .bind(&root)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(chain.len(), stages, "{root}");
        assert_eq!(chain.len(), durations.len(), "{root}");
        let mut key = Some(root.as_str());
        for (id, duration) in chain.into_iter().zip(durations) {
            let expected = format!("oteryn:item.tibia.i{id}");
            assert_eq!(key, Some(expected.as_str()), "{root}");
            let row = records.iter().chain(native_records).find(|row| {
                row["production_definition"]["production_key"] == expected
            }).ok_or("raid Item")?;
            let decay = &row["semantics"]["temporal"]["value"];
            assert_eq!(decay["duration"]["value"].as_i64(), Some(duration));
            key = decay["decay_target"]["value"]["key"].as_str();
        }
        assert_eq!(key, None, "{root}");
    }
    Ok(())
}
