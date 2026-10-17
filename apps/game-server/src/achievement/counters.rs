//! Pending counter-threshold bindings (ACHIEVEMENT0 §3.4; CP D1331–D1332).
//!
//! The embedded table carries one independently reviewed Reference threshold datum. This loader
//! checks authoring shape and resolves each key through the actual Achievement catalogue. Its
//! result remains pending: data qualification does not enable grants, fenced witnesses or the
//! same-transaction XP/build callers. Other counter keys receive no rows without qualified evidence.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::achievement_catalogue::AchievementCatalogue;
use crate::durability::account_achievement::AchievementCatalogueLookup;

const EMBEDDED: &str = include_str!("../../../../content/achievements/counter-thresholds.json");

/// The only counter families with an allocated native owner in ACHIEVEMENT0 §3.4.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String")]
pub(crate) enum AchievementCounter {
    Level,
    MagicLevel,
    Fist,
    Club,
    Sword,
    Axe,
    Distance,
    Shielding,
    Fishing,
}

impl TryFrom<String> for AchievementCounter {
    type Error = &'static str;

    fn try_from(key: String) -> Result<Self, Self::Error> {
        match key.as_str() {
            "level" => Ok(Self::Level),
            "magic_level" => Ok(Self::MagicLevel),
            "fist" => Ok(Self::Fist),
            "club" => Ok(Self::Club),
            "sword" => Ok(Self::Sword),
            "axe" => Ok(Self::Axe),
            "distance" => Ok(Self::Distance),
            "shielding" => Ok(Self::Shielding),
            "fishing" => Ok(Self::Fishing),
            _ => Err("unowned achievement counter key"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    schema_version: u32,
    status: String,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    counter: AchievementCounter,
    value: u64,
    achievement: String,
}

/// A pending authoring row bound to an existing, non-retired catalogue revision.
/// This binding carries no proof of the threshold's Reference evidence or earning authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingCounterThreshold {
    pub(crate) counter: AchievementCounter,
    pub(crate) value: u64,
    pub(crate) achievement: String,
    pub(crate) catalogue_revision: String,
}

/// A malformed, unsupported or unbound table is rejected as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CounterThresholdError {
    MalformedTable,
    UnsupportedVersion,
    NotPending,
    NonPositiveValue,
    DuplicateThreshold,
    AbsentAchievement(String),
    RetiredAchievement(String),
}

/// Shape-checked authoring data only; there is deliberately no active variant or grant API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingCounterThresholdCatalogue {
    rows: Vec<PendingCounterThreshold>,
}

impl PendingCounterThresholdCatalogue {
    /// Parse the checked-in table without activating any counter earning path.
    pub(crate) fn embedded(
        catalogue: &AchievementCatalogue,
    ) -> Result<Self, CounterThresholdError> {
        Self::from_json(EMBEDDED, catalogue)
    }

    /// Validate all rows against the same real catalogue. No lookup callback can substitute
    /// an invented earnable result, and the table cannot supply its own catalogue revision.
    pub(crate) fn from_json(
        text: &str,
        catalogue: &AchievementCatalogue,
    ) -> Result<Self, CounterThresholdError> {
        let table: Table =
            serde_json::from_str(text).map_err(|_| CounterThresholdError::MalformedTable)?;
        if table.schema_version != 1 {
            return Err(CounterThresholdError::UnsupportedVersion);
        }
        if table.status != "PENDING" {
            return Err(CounterThresholdError::NotPending);
        }
        let mut seen = BTreeSet::new();
        let mut rows = Vec::with_capacity(table.rows.len());
        for row in table.rows {
            if row.value == 0 {
                return Err(CounterThresholdError::NonPositiveValue);
            }
            // One counter event may name different achievements. Only an exact triple is a
            // duplicate; the batch writer owns the separate 64-request earning-TX limit.
            if !seen.insert((row.counter, row.value, row.achievement.clone())) {
                return Err(CounterThresholdError::DuplicateThreshold);
            }
            let catalogue_revision = match catalogue.lookup(&row.achievement) {
                AchievementCatalogueLookup::Earnable { revision } => revision,
                AchievementCatalogueLookup::Absent => {
                    return Err(CounterThresholdError::AbsentAchievement(row.achievement));
                }
                AchievementCatalogueLookup::Retired => {
                    return Err(CounterThresholdError::RetiredAchievement(row.achievement));
                }
            };
            rows.push(PendingCounterThreshold {
                counter: row.counter,
                value: row.value,
                achievement: row.achievement,
                catalogue_revision,
            });
        }
        Ok(Self { rows })
    }

    /// Inspect pending rows without turning them into an admitted grant catalogue.
    pub(crate) fn rows(&self) -> &[PendingCounterThreshold] {
        &self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    type TestResult = Result<(), String>;
    const ACTIVE: &str = "oteryn:achievement/synthetic_counter_active";
    const SECOND: &str = "oteryn:achievement/synthetic_counter_second";
    const RETIRED: &str = "oteryn:achievement/synthetic_counter_retired";

    // Synthetic identities and values test binding only; none is a claimed Tibia threshold.
    fn catalogue() -> Result<AchievementCatalogue, String> {
        let records: Vec<Value> = [
            (ACTIVE, "3", false),
            (SECOND, "9", false),
            (RETIRED, "1", true),
        ]
        .into_iter()
        .map(|(key, revision, retired)| {
            json!({
                "identity": {"family": "Achievement", "key": key, "revision": revision},
                "name": "Synthetic counter binding",
                "description": "No Reference threshold is asserted.",
                "grade": 1,
                "points": if retired { 0 } else { 1 },
                "secret": false,
                "retired": retired
            })
        })
        .collect();
        let shard = json!({"family": "Achievement", "records": records}).to_string();
        AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))
    }

    fn row(counter: &str, value: u64, achievement: &str) -> Value {
        json!({"counter": counter, "value": value, "achievement": achievement})
    }

    fn table(rows: Vec<Value>) -> Value {
        json!({"schema_version": 1, "status": "PENDING", "rows": rows})
    }

    fn load(
        table: Value,
        catalogue: &AchievementCatalogue,
    ) -> Result<PendingCounterThresholdCatalogue, CounterThresholdError> {
        PendingCounterThresholdCatalogue::from_json(&table.to_string(), catalogue)
    }

    #[test]
    fn achievement_counter_thresholds_embedded_table_has_only_qualified_pending_row() -> TestResult
    {
        let catalogue = AchievementCatalogue::embedded().map_err(|error| format!("{error:?}"))?;
        let pending = PendingCounterThresholdCatalogue::embedded(&catalogue)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            pending.rows(),
            &[PendingCounterThreshold {
                counter: AchievementCounter::Level,
                value: 50,
                achievement: "oteryn:achievement/seasoned_adventurer".into(),
                catalogue_revision: "1".into(),
            }]
        );
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_all_nine_keys_use_real_catalogue_revisions() -> TestResult {
        let catalogue = catalogue()?;
        let keys = [
            ("level", AchievementCounter::Level),
            ("magic_level", AchievementCounter::MagicLevel),
            ("fist", AchievementCounter::Fist),
            ("club", AchievementCounter::Club),
            ("sword", AchievementCounter::Sword),
            ("axe", AchievementCounter::Axe),
            ("distance", AchievementCounter::Distance),
            ("shielding", AchievementCounter::Shielding),
            ("fishing", AchievementCounter::Fishing),
        ];
        let rows = keys.iter().map(|(key, _)| row(key, 17, ACTIVE)).collect();
        let pending = load(table(rows), &catalogue).map_err(|error| format!("{error:?}"))?;
        assert_eq!(pending.rows().len(), keys.len());
        for (actual, (_, counter)) in pending.rows().iter().zip(keys) {
            assert_eq!(actual.counter, counter);
            assert_eq!(actual.value, 17);
            assert_eq!(actual.achievement, ACTIVE);
            assert_eq!(actual.catalogue_revision, "3");
        }
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_unowned_counter_names_are_rejected() -> TestResult {
        let catalogue = catalogue()?;
        for counter in ["xp", "skill", "soul", "Level", "magicLevel", ""] {
            assert_eq!(
                load(table(vec![row(counter, 17, ACTIVE)]), &catalogue),
                Err(CounterThresholdError::MalformedTable),
                "{counter}"
            );
        }
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_shape_and_duplicate_json_fields_fail_closed() -> TestResult {
        let catalogue = catalogue()?;
        for text in [
            "{",
            "[]",
            "null",
            r#"{"schema_version":1,"status":"PENDING"}"#,
            r#"{"schema_version":1,"status":"PENDING","rows":{}}"#,
            r#"{"schema_version":1,"status":"PENDING","rows":[],"active":true}"#,
            r#"{"schema_version":1,"schema_version":1,"status":"PENDING","rows":[]}"#,
        ] {
            assert_eq!(
                PendingCounterThresholdCatalogue::from_json(text, &catalogue),
                Err(CounterThresholdError::MalformedTable),
                "{text}"
            );
        }
        let mut unknown_row_field = table(vec![row("level", 17, ACTIVE)]);
        unknown_row_field["rows"][0]["catalogue_revision"] = json!("invented");
        assert_eq!(
            load(unknown_row_field, &catalogue),
            Err(CounterThresholdError::MalformedTable)
        );
        // A unit enum's alternate object representation is not a string counter key.
        for counter in [
            json!({"level": null}),
            json!(null),
            json!(17),
            json!(["level"]),
        ] {
            let mut input = table(vec![row("level", 17, ACTIVE)]);
            input["rows"][0]["counter"] = counter;
            assert_eq!(
                load(input, &catalogue),
                Err(CounterThresholdError::MalformedTable)
            );
        }
        let duplicate_row_field = format!(
            r#"{{"schema_version":1,"status":"PENDING","rows":[{{"counter":"level","value":17,"value":18,"achievement":"{ACTIVE}"}}]}}"#
        );
        assert_eq!(
            PendingCounterThresholdCatalogue::from_json(&duplicate_row_field, &catalogue),
            Err(CounterThresholdError::MalformedTable)
        );
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_version_and_activation_status_are_closed() -> TestResult {
        let catalogue = catalogue()?;
        for version in [0, 2] {
            let mut input = table(vec![]);
            input["schema_version"] = json!(version);
            assert_eq!(
                load(input, &catalogue),
                Err(CounterThresholdError::UnsupportedVersion)
            );
        }
        for status in ["ACTIVE", "READY", "pending", ""] {
            let mut input = table(vec![]);
            input["status"] = json!(status);
            assert_eq!(
                load(input, &catalogue),
                Err(CounterThresholdError::NotPending)
            );
        }
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_values_are_positive_lossless_u64() -> TestResult {
        let catalogue = catalogue()?;
        assert_eq!(
            load(table(vec![row("level", 0, ACTIVE)]), &catalogue),
            Err(CounterThresholdError::NonPositiveValue)
        );
        for value in ["-1", "1.5", "18446744073709551616", "true", "\"17\""] {
            let input = format!(
                r#"{{"schema_version":1,"status":"PENDING","rows":[{{"counter":"level","value":{value},"achievement":"{ACTIVE}"}}]}}"#
            );
            assert_eq!(
                PendingCounterThresholdCatalogue::from_json(&input, &catalogue),
                Err(CounterThresholdError::MalformedTable),
                "{value}"
            );
        }
        let pending = load(table(vec![row("level", u64::MAX, ACTIVE)]), &catalogue)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(pending.rows()[0].value, u64::MAX);
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_absent_and_retired_keys_fail_closed() -> TestResult {
        let catalogue = catalogue()?;
        let absent = "oteryn:achievement/synthetic_counter_absent";
        assert_eq!(
            load(table(vec![row("level", 17, absent)]), &catalogue),
            Err(CounterThresholdError::AbsentAchievement(absent.into()))
        );
        assert_eq!(
            load(table(vec![row("level", 17, RETIRED)]), &catalogue),
            Err(CounterThresholdError::RetiredAchievement(RETIRED.into()))
        );
        Ok(())
    }

    #[test]
    fn achievement_counter_thresholds_exact_duplicate_is_rejected_but_shared_event_can_bind()
    -> TestResult {
        let catalogue = catalogue()?;
        let first = row("level", 17, ACTIVE);
        assert_eq!(
            load(table(vec![first.clone(), first.clone()]), &catalogue),
            Err(CounterThresholdError::DuplicateThreshold)
        );
        let pending = load(table(vec![first, row("level", 17, SECOND)]), &catalogue)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(pending.rows().len(), 2);
        assert_eq!(pending.rows()[0].achievement, ACTIVE);
        assert_eq!(pending.rows()[0].catalogue_revision, "3");
        assert_eq!(pending.rows()[1].achievement, SECOND);
        assert_eq!(pending.rows()[1].catalogue_revision, "9");
        Ok(())
    }
}
