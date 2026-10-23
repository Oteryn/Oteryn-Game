//! Authored policy rows; loading this JSON into runtime remains a follow-up.
use crate::shared_experience::SHARED_EXPERIENCE_ACTIVITY_MICROS;

#[test]
fn party_shared_experience_ruleset_records_the_accepted_policy() -> Result<(), serde_json::Error> {
    let row: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../rulesets/party/shared-experience/shared-experience.json"
    ))?;
    assert_eq!(
        row["activity_window_ms"],
        SHARED_EXPERIENCE_ACTIVITY_MICROS / 1_000
    );
    assert_eq!(row["scope"], "CHANNEL_LOCAL");
    assert_eq!(row["distance_chebyshev_tiles"], 30);
    assert_eq!(row["lowest_level_multiplier"], 3);
    assert_eq!(row["highest_level_multiplier"], 2);
    assert_eq!(row["bonus_minimum_base_xp"], 20);
    assert_eq!(
        row["bonus_percent_by_distinct_vocations"],
        serde_json::json!([20, 30, 60, 100])
    );
    assert_eq!(row["split_rounding"], "CEILING");
    assert!(row["runtime_consumer"].is_null());
    assert_eq!(
        row["provenance"]["activity_window_and_pz_interpretation"],
        "OTERYN_DECISION:PARITY_PENDING"
    );
    Ok(())
}
