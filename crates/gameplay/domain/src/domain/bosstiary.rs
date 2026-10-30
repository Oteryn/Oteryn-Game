//! PKG-P4/P02: category rules from the single Bosstiary content table.
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BosstiaryBoss {
    pub key: String,
    pub category: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BosstiaryProgress {
    /// 0 = below prowess, 1 = prowess, 2 = expertise, 3 = mastery.
    pub stage: u8,
    pub points: u16,
}

#[derive(Deserialize)]
struct Category {
    category: String,
    kills: [u64; 3],
    points: [u16; 3],
}

pub fn progress(category: &str, kills: u64) -> Option<BosstiaryProgress> {
    let rows: Vec<Category> = serde_json::from_str(include_str!(
        "../../../../../rulesets/progression/bosstiary/categories.json"
    ))
    .ok()?;
    let row = rows.into_iter().find(|row| row.category == category)?;
    let mut result = BosstiaryProgress {
        stage: 0,
        points: 0,
    };
    for (threshold, points) in row.kills.into_iter().zip(row.points) {
        if kills >= threshold {
            result.stage += 1;
            result.points += points;
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_threshold_and_cumulative_points() {
        for (category, thresholds, awards) in [
            ("bane", [25, 100, 300], [5, 15, 30]),
            ("archfoe", [5, 20, 60], [10, 30, 60]),
            ("nemesis", [1, 3, 5], [10, 30, 60]),
        ] {
            let mut points = 0;
            for (index, threshold) in thresholds.into_iter().enumerate() {
                assert_eq!(
                    progress(category, threshold - 1),
                    Some(BosstiaryProgress {
                        stage: index as u8,
                        points
                    })
                );
                points += awards[index];
                for count in [threshold, threshold + 1] {
                    assert_eq!(
                        progress(category, count),
                        Some(BosstiaryProgress {
                            stage: index as u8 + 1,
                            points
                        })
                    );
                }
            }
        }
        assert_eq!(progress("unknown", 10), None);
    }
}
