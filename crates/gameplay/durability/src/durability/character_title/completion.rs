//! PR #2782 Q2=B: actual class/category completion over the existing Bestiary/Bosstiary
//! owners. This deliberately corrects Crystal's enum-as-race bug at player_title.cpp:254-267
//! and player.cpp:9761-9766. No completion snapshot or second kill counter is persisted.
use crate::content::{ProjectEvidenceLimits, canonical_title_catalogue};
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use oteryn_protocol_oteryn::character_info::{BestiaryClass, BossCategory};
use std::collections::BTreeMap;
use std::sync::LazyLock;

struct Entry {
    key: String,
    threshold: u64,
    group: usize,
}

pub(super) struct Source {
    bestiary: Vec<Entry>,
    bestiary_keys: Vec<String>,
    bosses: Vec<Entry>,
    boss_keys: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Facts {
    pub bestiary_classes: [bool; BestiaryClass::COUNT],
    pub all_bestiary: bool,
    pub boss_categories: [bool; 3],
    pub all_bosses: bool,
}

pub(super) fn source() -> Option<&'static Source> {
    static SOURCE: LazyLock<Option<Source>> = LazyLock::new(|| {
        let catalogue = canonical_title_catalogue(ProjectEvidenceLimits {
            max_documents: 5,
            max_document_bytes: 2_500_000,
            max_total_bytes: 8_000_000,
            max_json_depth: 10,
            max_decoded_fields: 65_000,
            max_string_bytes: 1_000_000,
            max_locator_bytes: 128,
            max_locator_segments: 5,
            max_reference_records: 4096,
            max_import_records: 1,
            max_reimport_states: 1,
        })
        .ok()?;
        let bestiary = catalogue
            .bestiary
            .into_iter()
            .map(|(race, class)| {
                Some(Entry {
                    key: race.key().to_owned(),
                    threshold: u64::from(race.final_kill_threshold()),
                    group: BestiaryClass::from_name(&class)? as usize,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let bosses = catalogue
            .bosstiary
            .into_iter()
            .map(|boss| {
                let group = match boss.category.as_str() {
                    "bane" => BossCategory::Bane,
                    "archfoe" => BossCategory::Archfoe,
                    "nemesis" => BossCategory::Nemesis,
                    _ => return None,
                };
                Some(Entry {
                    threshold: crate::domain::bosstiary::kill_thresholds(&boss.category)?[2],
                    key: boss.key,
                    group: group as usize,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Source {
            bestiary_keys: bestiary.iter().map(|entry| entry.key.clone()).collect(),
            boss_keys: bosses.iter().map(|entry| entry.key.clone()).collect(),
            bestiary,
            bosses,
        })
    });
    SOURCE.as_ref()
}

impl Source {
    pub(super) async fn read(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
    ) -> Result<Facts, DurabilityError> {
        // Both result sets are bounded by the immutable canonical key lists and the existing
        // (Character, race/boss) primary keys. Unknown/non-canonical counters cannot qualify.
        let bestiary: Vec<(String, i64)> = sqlx::query_as(
            "SELECT race_key,kill_count FROM game_character_bestiary_progress \
             WHERE character_id=encode($1,'hex')::uuid AND race_key=ANY($2::text[])",
        )
        .bind(character.as_bytes().as_slice())
        .bind(&self.bestiary_keys)
        .fetch_all(&mut **tx)
        .await?;
        let bosses: Vec<(String, i64)> = sqlx::query_as(
            "SELECT boss_key,kill_count FROM game_character_bosstiary_progress \
             WHERE character_id=encode($1,'hex')::uuid AND boss_key=ANY($2::text[])",
        )
        .bind(character.as_bytes().as_slice())
        .bind(&self.boss_keys)
        .fetch_all(&mut **tx)
        .await?;
        let counters = |rows: Vec<(String, i64)>| {
            rows.into_iter()
                .map(|(key, count)| {
                    u64::try_from(count)
                        .map(|count| (key, count))
                        .map_err(|_| DurabilityError::InvalidStoredState)
                })
                .collect::<Result<BTreeMap<_, _>, _>>()
        };
        let (bestiary_classes, all_bestiary) = completed(&self.bestiary, &counters(bestiary)?);
        let (boss_categories, all_bosses) = completed(&self.bosses, &counters(bosses)?);
        Ok(Facts {
            bestiary_classes,
            all_bestiary,
            boss_categories,
            all_bosses,
        })
    }
}

fn completed<const N: usize>(
    entries: &[Entry],
    counters: &BTreeMap<String, u64>,
) -> ([bool; N], bool) {
    let mut total = [0_u32; N];
    let mut complete = [0_u32; N];
    for entry in entries {
        total[entry.group] += 1;
        if counters
            .get(&entry.key)
            .is_some_and(|count| *count >= entry.threshold)
        {
            complete[entry.group] += 1;
        }
    }
    let groups = std::array::from_fn(|group| total[group] > 0 && total[group] == complete[group]);
    (groups, !entries.is_empty() && total == complete)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_requires_every_exact_member_at_the_final_threshold() {
        let entry = |key: &str, threshold, group| Entry {
            key: key.into(),
            threshold,
            group,
        };
        let entries = [
            entry("first", 5, 0),
            entry("second", 10, 0),
            entry("other", 3, 1),
        ];
        let mut counts = BTreeMap::from([
            ("first".into(), 5),
            ("second".into(), 9),
            ("other".into(), 3),
            ("unknown".into(), u64::MAX),
        ]);
        assert_eq!(
            completed::<3>(&entries, &counts),
            ([false, true, false], false)
        );
        counts.insert("second".into(), 10);
        assert_eq!(
            completed::<3>(&entries, &counts),
            ([true, true, false], true)
        );
        counts.remove("first");
        assert_eq!(
            completed::<3>(&entries, &counts),
            ([false, true, false], false)
        );
        assert_eq!(
            completed::<3>(&[], &counts),
            ([false; 3], false),
            "empty catalogues never grant all-complete titles"
        );
        let expanded = [
            entry("first", 5, 0),
            entry("second", 10, 0),
            entry("other", 3, 1),
            entry("new", 1, 1),
        ];
        counts.insert("first".into(), 5);
        assert_eq!(
            completed::<3>(&expanded, &counts),
            ([true, false, false], false),
            "new canonical entries revoke conditional completion"
        );
    }
}
