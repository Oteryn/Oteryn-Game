//! Migration 2770 converts legacy four-cell domain vessels into twelve Gem Vessel slots exactly as
//! the server's own legacy conversion does, and refuses rather than drop a gem.
use super::TestResult;
use super::fixture::{Harness, run};
use oteryn_gameplay_progression::atelier::GemQuality;
use oteryn_gameplay_progression::atelier::vessels::{
    GemVessels, NO_GEM_VESSELS, from_legacy_cells,
};

fn gem(tag: u8) -> [u8; 16] {
    let mut id = [0_u8; 16];
    id[0] = 0x01;
    id[15] = tag;
    id
}

fn state(gems: &[([u8; 16], GemQuality)]) -> serde_json::Value {
    let gems: serde_json::Map<String, serde_json::Value> = gems
        .iter()
        .map(|(id, quality)| {
            let quality = match quality {
                GemQuality::Lesser => "Lesser",
                GemQuality::Regular => "Regular",
                GemQuality::Greater => "Greater",
            };
            (
                u128::from_be_bytes(*id).to_string(),
                serde_json::json!({ "quality": quality, "domain": "Green" }),
            )
        })
        .collect();
    serde_json::json!({ "gems": gems })
}

async fn convert(
    h: &Harness,
    cells: [Option<[u8; 16]>; 4],
    atelier: &serde_json::Value,
) -> Result<GemVessels, sqlx::Error> {
    let converted: serde_json::Value =
        sqlx::query_scalar("SELECT game_wheel_gem_vessels_from_legacy($1, $2)")
            .bind(serde_json::to_value(cells).map_err(|e| sqlx::Error::Decode(Box::new(e)))?)
            .bind(atelier)
            .fetch_one(&h.pool)
            .await?;
    serde_json::from_value(converted).map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

fn refused(result: Result<GemVessels, sqlx::Error>) -> bool {
    result.err().is_some_and(|error| {
        error
            .as_database_error()
            .and_then(|db| db.code())
            .is_some_and(|code| code == "23514")
    })
}

#[test]
fn gem_vessel_migration_converts_legacy_domain_cells_like_the_server() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "gem_vessel_migration", false).await?;
        let gems = [
            (gem(1), GemQuality::Regular),
            (gem(2), GemQuality::Regular),
            (gem(3), GemQuality::Lesser),
            (gem(4), GemQuality::Greater),
        ];
        let atelier = state(&gems);
        let quality_of = |id: [u8; 16]| {
            gems.iter()
                .find(|(gem, _)| *gem == id)
                .map(|(_, quality)| *quality)
        };
        for cells in [
            [Some(gem(1)), None, Some(gem(2)), Some(gem(3))],
            [Some(gem(4)), Some(gem(3)), Some(gem(2)), Some(gem(1))],
            [None, None, None, Some(gem(4))],
        ] {
            let server = from_legacy_cells(&NO_GEM_VESSELS, &cells, quality_of)
                .ok_or("server conversion refused a stored legacy vector")?;
            assert_eq!(convert(&h, cells, &atelier).await?, server);
            let kept = server.iter().flatten().count();
            assert_eq!(kept, cells.iter().flatten().count(), "no gem is dropped");
        }
        assert_eq!(
            convert(&h, [None; 4], &serde_json::Value::Null).await?,
            NO_GEM_VESSELS,
            "a Character without Atelier state keeps empty vessels"
        );
        // A gem the Atelier state does not know, or one listed twice, stops the migration.
        assert!(refused(
            convert(&h, [Some(gem(9)), None, None, None], &atelier).await
        ));
        assert!(refused(
            convert(&h, [Some(gem(1)), Some(gem(1)), None, None], &atelier).await
        ));
        let (default, slots): (String, String) = sqlx::query_as(
            "SELECT (SELECT column_default FROM information_schema.columns
                      WHERE table_name='game_character_wheel_receipts'
                        AND column_name='gem_vessels_after'),
                    (SELECT pg_get_constraintdef(oid) FROM pg_constraint
                      WHERE conname='game_wheel_vessels_slots')",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(default.matches("null").count(), 12, "new receipts default to empty slots");
        assert!(slots.contains("12"));
        h.cleanup().await
    })
}
