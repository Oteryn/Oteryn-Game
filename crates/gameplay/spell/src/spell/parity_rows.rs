//! Parity area files (`docs/reference/parity/<area>.json`): the generic row, status and loader
//! split. The schema is described in `docs/reference/parity/README.md`.
//!
//! Test-only and shared: the spell crate mounts it, and the dispatch crate mounts the same file
//! with `#[path]` so both adapters read one loader. A new area is one data file plus one adapter
//! that maps a row's `case` key to the owner call and returns the observed facts.
// Each adapter uses only part of the helpers.
#![allow(dead_code)]

use serde_json::Value;
use std::collections::BTreeMap;

/// Facts an adapter observed from the real owner, keyed by the names the data file uses.
pub type Observed = BTreeMap<&'static str, i64>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Global-measured values; the owner must produce them.
    Evidence,
    /// Global evidence exists but content diverges: `current` is pinned and must differ from
    /// `expected`, so the row goes red when Content fixes the value and has to be flipped.
    KnownGap,
    /// No evidence yet: the owner path runs and is sanity-checked, no parity value is asserted.
    Pending,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub case: String,
    pub inputs: Value,
    /// Global values (Evidence, KnownGap).
    pub expected: BTreeMap<String, i64>,
    /// KnownGap only: the value the owner produces today.
    pub current: BTreeMap<String, i64>,
    pub status: Status,
    /// Allowed deviation of an Evidence value, in percent, where the source samples the extremes.
    pub tolerance_percent: i64,
}

pub fn load(json: &str) -> Vec<Row> {
    let file: Value = serde_json::from_str(json).expect("parity file is JSON");
    let facts = |value: Option<&Value>| -> BTreeMap<String, i64> {
        value
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(key, v)| (key.clone(), v.as_i64().expect("fact is an integer")))
                    .collect()
            })
            .unwrap_or_default()
    };
    let rows = file["rows"].as_array().expect("rows");
    let mut seen = std::collections::BTreeSet::new();
    rows.iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().expect(key).to_owned();
            let status = match row["status"].as_str().expect("status") {
                "Evidence" => Status::Evidence,
                "KnownGap" => Status::KnownGap,
                "Pending" => Status::Pending,
                other => panic!("unknown status {other}"),
            };
            let parsed = Row {
                id: text("id"),
                case: text("case"),
                inputs: row["inputs"].clone(),
                expected: facts(row.get("expected")),
                current: facts(row.get("current")),
                status,
                tolerance_percent: row["tolerance_percent"].as_i64().unwrap_or(0),
            };
            assert!(
                seen.insert(parsed.id.clone()),
                "duplicate row id {}",
                parsed.id
            );
            assert!(
                row["source"].is_string() && row["notes"].is_string(),
                "{} needs source and notes",
                parsed.id
            );
            match status {
                Status::Evidence => assert!(!parsed.expected.is_empty(), "{}", parsed.id),
                Status::KnownGap => assert!(
                    !parsed.expected.is_empty() && !parsed.current.is_empty(),
                    "{} needs expected (Global) and current",
                    parsed.id
                ),
                Status::Pending => assert!(
                    parsed.expected.is_empty() && parsed.current.is_empty(),
                    "{} is Pending and carries no value",
                    parsed.id
                ),
            }
            parsed
        })
        .collect()
}

impl Row {
    pub fn str(&self, key: &str) -> &str {
        self.inputs[key]
            .as_str()
            .unwrap_or_else(|| panic!("{}: input {key}", self.id))
    }

    pub fn int(&self, key: &str) -> i64 {
        self.inputs[key].as_i64().unwrap_or(0)
    }

    pub fn str_or(&self, key: &str) -> &str {
        self.inputs[key].as_str().unwrap_or("")
    }

    pub fn has(&self, key: &str) -> bool {
        !self.inputs[key].is_null()
    }

    fn near(&self, expected: i64, observed: i64) -> bool {
        let slack = expected.abs() * self.tolerance_percent / 100;
        (observed - expected).abs() <= slack
    }

    fn matches_global(&self, observed: &Observed) -> bool {
        self.expected.iter().all(|(key, want)| {
            observed
                .get(key.as_str())
                .is_some_and(|got| self.near(*want, *got))
        })
    }

    /// Apply the status rule to what the owner produced.
    pub fn judge(&self, observed: &Observed) {
        match self.status {
            Status::Evidence => assert!(
                self.matches_global(observed),
                "{}: Global {:?} (tolerance {}%), owner produced {observed:?}",
                self.id,
                self.expected,
                self.tolerance_percent
            ),
            Status::KnownGap => {
                for (key, want) in &self.current {
                    assert_eq!(
                        observed.get(key.as_str()),
                        Some(want),
                        "{}: current value of {key} moved; if Content fixed it, flip the row to Evidence",
                        self.id
                    );
                }
                assert!(
                    !self.matches_global(observed),
                    "{}: the owner now matches Global {:?}; flip the row to Evidence",
                    self.id,
                    self.expected
                );
            }
            Status::Pending => {
                if observed.get("castable") == Some(&0) {
                    return;
                }
                if let (Some(min), Some(max)) = (observed.get("min"), observed.get("max")) {
                    assert!(min <= max && *min >= 0, "{}: bounds {min}..{max}", self.id);
                }
                if let Some(mana) = observed.get("mana") {
                    assert!(
                        *mana > 0 || observed.get("free_cast") == Some(&1),
                        "{}: mana {mana}",
                        self.id
                    );
                }
            }
        }
    }
}
