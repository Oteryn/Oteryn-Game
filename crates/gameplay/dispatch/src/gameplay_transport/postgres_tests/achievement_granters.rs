//! P11 assertions shared by the real fishing/gathering and NPC arrival scenarios.
use super::TestResult;
use super::fixture::{Harness, id};

const GATHERING: [(&str, i64); 4] = [
    ("oteryn:achievement/here_fishy_fishy", 250),
    ("oteryn:achievement/exquisite_taste", 250),
    ("oteryn:achievement/skin_deep", 500),
    ("oteryn:achievement/ashes_to_dust", 500),
];

pub(super) async fn seed_progress(h: &Harness, key: &str, progress: i64) -> TestResult {
    sqlx::query("INSERT INTO game_character_achievement_progress(character_id,achievement_key,progress) VALUES(encode($1,'hex')::uuid,$2,$3)")
        .bind(id(41).as_slice()).bind(key).bind(progress).execute(&h.pool).await?;
    Ok(())
}

pub(super) async fn seed_gathering(h: &Harness) -> TestResult {
    for (key, total) in GATHERING {
        seed_progress(h, key, total - 1).await?;
    }
    Ok(())
}

/// Return the first provenance as well, so replay must preserve the event and timestamp.
pub(super) async fn assert_progress(
    h: &Harness,
    key: &str,
    progress: i64,
    granted: bool,
    source: &str,
) -> TestResult<Option<(String, Vec<u8>, i64)>> {
    let actual: i64 = sqlx::query_scalar("SELECT progress FROM game_character_achievement_progress WHERE character_id=encode($1,'hex')::uuid AND achievement_key=$2")
        .bind(id(41).as_slice()).bind(key).fetch_one(&h.pool).await?;
    assert_eq!(actual, progress, "counter {key}");
    let requests: Vec<(String, Vec<u8>, i64)> = sqlx::query_as("SELECT source_kind,source_event_id,earned_at FROM game_account_achievement_grant_requests WHERE character_id=encode($1,'hex')::uuid AND achievement_key=$2")
        .bind(id(41).as_slice()).bind(key).fetch_all(&h.pool).await?;
    assert_eq!(requests.len(), usize::from(granted), "grant requests {key}");
    let facts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE account_id=encode($1,'hex')::uuid AND achievement_key=$2")
        .bind(id(40).as_slice()).bind(key).fetch_one(&h.pool).await?;
    assert_eq!(facts, i64::from(granted), "account facts {key}");
    if let Some(request) = requests.first() {
        assert_eq!(request.0, source);
        assert!(!request.1.is_empty());
    }
    Ok(requests.into_iter().next())
}

pub(super) async fn assert_gathering(
    h: &Harness,
    earned: Option<&str>,
) -> TestResult<Option<(String, Vec<u8>, i64)>> {
    let mut provenance = None;
    for (key, total) in GATHERING {
        let granted = earned == Some(key);
        let fact = assert_progress(
            h,
            key,
            total - i64::from(!granted),
            granted,
            "oteryn:item-craft",
        )
        .await?;
        if fact.is_some() {
            provenance = fact;
        }
    }
    if let Some((_, event, _)) = &provenance {
        let receipt_event: String =
            sqlx::query_scalar("SELECT event_id::text FROM game_item_craft_records")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(event.as_slice(), receipt_event.as_bytes());
    }
    Ok(provenance)
}
