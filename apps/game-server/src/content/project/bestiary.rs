//! Bestiary facts projected from the canonical Creature admission source. This decoder grants
//! no generation or activation authority; owning Content qualification binds runtime consumption.

use super::{
    ProjectError, ProjectEvidenceLimits, ProjectReferenceRecord, ProjectV2AuthoringProfileData,
    ProjectV2SourceIdentityBinding, parse_strict,
};
use crate::domain::bestiary::BestiaryRace;
use crate::domain::charm::BestiaryRaceKey;
use serde::Deserialize;

const INDEX: &[u8] = include_bytes!("../../../../../content/creatures/definitions/index.json");
const SHARDS: [(&str, &[u8]); 4] = [
    (
        "content/creatures/definitions/creatures-00000-00499.json",
        include_bytes!("../../../../../content/creatures/definitions/creatures-00000-00499.json"),
    ),
    (
        "content/creatures/definitions/creatures-00500-00999.json",
        include_bytes!("../../../../../content/creatures/definitions/creatures-00500-00999.json"),
    ),
    (
        "content/creatures/definitions/creatures-01000-01499.json",
        include_bytes!("../../../../../content/creatures/definitions/creatures-01000-01499.json"),
    ),
    (
        "content/creatures/definitions/creatures-01500-01762.json",
        include_bytes!("../../../../../content/creatures/definitions/creatures-01500-01762.json"),
    ),
];
const CREATURE_COUNT: usize = 1_763;
const BESTIARY_COUNT: usize = 819;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: String,
    family: String,
    record_count: usize,
    attached_authoring_profiles: usize,
    attached_source_bindings: usize,
    shard_size: usize,
    shards: Vec<String>,
    legacy_source: LegacySource,
    /// Native profile overlay descriptors; they create no canonical identity or Bestiary row.
    #[serde(default, rename = "spell_imports")]
    _spell_imports: Vec<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySource {
    schema: String,
    path: String,
    #[serde(rename = "git_blob_sha")]
    _git_blob_sha: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    schema: String,
    family: String,
    shard: ShardRange,
    source_legacy_role: String,
    records: Vec<Record>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShardRange {
    index: usize,
    start: usize,
    end: usize,
    count: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    definition: ProjectReferenceRecord,
    authoring: ProjectV2AuthoringProfileData,
    source_bindings: Vec<ProjectV2SourceIdentityBinding>,
}

/// Decode all canonical source records within caller-selected evidence limits, returning the
/// existing race type and its Charm points. Source revisions are retained verbatim.
#[allow(
    dead_code,
    reason = "native Charm generation binding is the separately owned consumer"
)]
pub(crate) fn canonical_bestiary_rows(
    limits: ProjectEvidenceLimits,
) -> Result<Vec<(BestiaryRace, u32)>, ProjectError> {
    decode_sources(INDEX, &SHARDS, limits)
}

fn decode_sources(
    index_bytes: &[u8],
    shards: &[(&str, &[u8])],
    limits: ProjectEvidenceLimits,
) -> Result<Vec<(BestiaryRace, u32)>, ProjectError> {
    let limits = limits.validate()?;
    limits.check(
        "Bestiary source documents",
        shards.len() + 1,
        limits.max_documents,
    )?;
    let mut total_bytes = 0;
    for bytes in std::iter::once(index_bytes).chain(shards.iter().map(|(_, bytes)| *bytes)) {
        limits.check(
            "Bestiary source document bytes",
            bytes.len(),
            limits.max_document_bytes,
        )?;
        total_bytes = super::checked_limit_sum(
            "Bestiary source bytes",
            total_bytes,
            bytes.len(),
            limits.max_total_bytes,
        )?;
    }
    let index: Index = parse_strict(index_bytes, limits)?;
    if index.schema != "OTERYN_FAMILY_INDEX/v1"
        || index.family != "Creature"
        || index.record_count != CREATURE_COUNT
        || index.attached_authoring_profiles != CREATURE_COUNT
        || index.attached_source_bindings != CREATURE_COUNT
        || index.shard_size != 500
        || index
            .shards
            .iter()
            .map(String::as_str)
            .ne(shards.iter().map(|(path, _)| *path))
        || index.legacy_source.schema != super::WORLD_PROJECT_REFERENCE_SCHEMA
    {
        return Err(ProjectError::InvalidProject(
            "Bestiary Creature source index mismatch",
        ));
    }
    limits.check(
        "Bestiary source records",
        index.record_count,
        limits.max_reference_records,
    )?;
    let mut rows = Vec::new();
    let mut previous = None;
    let mut seen = 0;
    for (ordinal, (_, bytes)) in shards.iter().enumerate() {
        let shard: Shard = parse_strict(bytes, limits)?;
        if shard.schema != "OTERYN_CREATURE_ADMISSION_SHARD/v1"
            || shard.family != "Creature"
            || shard.source_legacy_role != index.legacy_source.path
            || shard.shard.index != ordinal
            || shard.shard.start != seen
            || shard.shard.count != shard.records.len()
            || shard.shard.count != index.shard_size.min(index.record_count - seen)
            || shard.shard.end.checked_add(1) != seen.checked_add(shard.shard.count)
        {
            return Err(ProjectError::InvalidProject(
                "Bestiary Creature shard mismatch",
            ));
        }
        for record in shard.records {
            let identity = record.definition.identity();
            if !matches!(&record.definition, ProjectReferenceRecord::Creature { .. })
                || identity.family != "Creature"
                || previous
                    .as_ref()
                    .is_some_and(|key: &String| key >= &identity.key)
                || record.source_bindings.is_empty()
                || record.source_bindings.iter().any(|binding| {
                    binding.target.family != super::ProjectV2Family::Creature
                        || binding.target.key != identity.key
                        || binding.target.revision != identity.revision
                })
            {
                return Err(ProjectError::InvalidProject(
                    "Bestiary Creature record binding mismatch",
                ));
            }
            identity.lower()?;
            previous = Some(identity.key.clone());
            let ProjectV2AuthoringProfileData::Creature(profile) = record.authoring else {
                return Err(ProjectError::InvalidProject(
                    "Bestiary authoring is not Creature",
                ));
            };
            if let Some(bestiary) = profile.bestiary {
                if bestiary.kill_thresholds.len() != 3 {
                    return Err(ProjectError::InvalidProject(
                        "Charm Bestiary requires three thresholds",
                    ));
                }
                BestiaryRaceKey::new(identity.key.clone())
                    .map_err(|_| ProjectError::InvalidProject("invalid Charm Bestiary race key"))?;
                let race = BestiaryRace::new(
                    identity.key.clone(),
                    identity.revision.clone(),
                    bestiary.kill_thresholds,
                )
                .map_err(|_| {
                    ProjectError::InvalidProject("invalid Bestiary identity or thresholds")
                })?;
                rows.push((race, u32::from(bestiary.charm_points)));
            }
            seen += 1;
        }
    }
    if seen != CREATURE_COUNT || rows.len() != BESTIARY_COUNT {
        return Err(ProjectError::InvalidProject(
            "canonical Bestiary population mismatch",
        ));
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProjectEvidenceLimits {
        ProjectEvidenceLimits {
            max_documents: 5,
            max_document_bytes: 1_400_000,
            max_total_bytes: 4_700_000,
            max_json_depth: 10,
            max_decoded_fields: 65_000,
            max_string_bytes: 1_000_000,
            max_locator_bytes: 128,
            max_locator_segments: 5,
            max_reference_records: 1_763,
            max_import_records: 1,
            max_reimport_states: 1,
        }
    }

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn mutate_first_shard(
        mutator: impl FnOnce(&mut serde_json::Value) -> TestResult,
    ) -> TestResult<Vec<(BestiaryRace, u32)>> {
        let mut shard: serde_json::Value = serde_json::from_slice(SHARDS[0].1)?;
        mutator(&mut shard)?;
        let bytes = serde_json::to_vec(&shard)?;
        let shards = [
            (SHARDS[0].0, bytes.as_slice()),
            SHARDS[1],
            SHARDS[2],
            SHARDS[3],
        ];
        Ok(decode_sources(INDEX, &shards, limits())?)
    }

    #[test]
    fn canonical_projection_retains_all_819_real_bestiary_blocks() -> TestResult {
        let rows = canonical_bestiary_rows(limits())?;
        assert_eq!(rows.len(), 819);
        assert!(
            rows.windows(2)
                .all(|pair| pair[0].0.key() < pair[1].0.key())
        );
        for (key, thresholds, points) in [
            ("oteryn:creature.rat", [10, 100, 250], 5),
            ("oteryn:creature.draptor", [2, 3, 5], 50),
        ] {
            let (race, actual_points) = rows
                .iter()
                .find(|(race, _)| race.key() == key)
                .ok_or("canonical Bestiary race is absent")?;
            assert_eq!(race.definition_revision(), "definition-r1");
            assert_eq!(race.kill_thresholds(), thresholds);
            assert_eq!(*actual_points, points);
        }
        assert!(
            !rows
                .iter()
                .any(|(race, _)| race.key() == "oteryn:creature.a_carved_stone_tile")
        );
        Ok(())
    }

    #[test]
    fn malformed_thresholds_and_source_alignment_fail_closed() {
        for thresholds in [vec![1, 2], vec![0, 2, 3], vec![1, 1, 3], vec![3, 2, 1]] {
            assert!(
                mutate_first_shard(|shard| {
                    let record = shard["records"]
                        .as_array_mut()
                        .ok_or("canonical records absent")?
                        .iter_mut()
                        .find(|record| record["authoring"]["profile"]["bestiary"].is_object())
                        .ok_or("canonical Bestiary record absent")?;
                    record["authoring"]["profile"]["bestiary"]["kill_thresholds"] =
                        serde_json::json!(thresholds);
                    Ok(())
                })
                .is_err()
            );
        }
        for field in ["schema", "family", "source_legacy_role"] {
            assert!(
                mutate_first_shard(|shard| {
                    shard[field] = serde_json::json!("wrong");
                    Ok(())
                })
                .is_err()
            );
        }
        for field in ["key", "revision", "family"] {
            assert!(
                mutate_first_shard(|shard| {
                    shard["records"][0]["source_bindings"][0]["target"][field] =
                        serde_json::json!("wrong");
                    Ok(())
                })
                .is_err()
            );
        }
        assert!(
            mutate_first_shard(|shard| {
                shard["records"]
                    .as_array_mut()
                    .ok_or("canonical records absent")?
                    .swap(0, 1);
                Ok(())
            })
            .is_err()
        );
        assert!(
            mutate_first_shard(|shard| {
                shard["records"][1] = shard["records"][0].clone();
                Ok(())
            })
            .is_err()
        );
        assert!(
            mutate_first_shard(|shard| {
                shard["shard"]["start"] = serde_json::json!(1);
                Ok(())
            })
            .is_err()
        );
    }

    #[test]
    fn source_budgets_and_duplicate_members_are_enforced() {
        let mut changed = limits();
        changed.max_documents = 4;
        assert!(matches!(
            canonical_bestiary_rows(changed),
            Err(ProjectError::LimitExceeded { .. })
        ));
        changed = limits();
        changed.max_document_bytes = SHARDS[1].1.len() - 1;
        assert!(matches!(
            canonical_bestiary_rows(changed),
            Err(ProjectError::LimitExceeded { .. })
        ));
        changed = limits();
        changed.max_total_bytes =
            INDEX.len() + SHARDS.iter().map(|(_, bytes)| bytes.len()).sum::<usize>() - 1;
        assert!(matches!(
            canonical_bestiary_rows(changed),
            Err(ProjectError::LimitExceeded { .. })
        ));
        changed = limits();
        changed.max_reference_records = 1_762;
        assert!(matches!(
            canonical_bestiary_rows(changed),
            Err(ProjectError::LimitExceeded { .. })
        ));
        for (depth, fields, strings) in [
            (9, 65_000, 1_000_000),
            (10, 50, 1_000_000),
            (10, 65_000, 100),
        ] {
            changed = limits();
            changed.max_json_depth = depth;
            changed.max_decoded_fields = fields;
            changed.max_string_bytes = strings;
            assert!(canonical_bestiary_rows(changed).is_err());
        }
        changed = limits();
        changed.max_json_depth = 0;
        assert!(matches!(
            canonical_bestiary_rows(changed),
            Err(ProjectError::InvalidLimit(_))
        ));
        let duplicate = br#"{"schema":"OTERYN_FAMILY_INDEX/v1","schema":"duplicate"}"#;
        assert!(matches!(
            decode_sources(duplicate, &SHARDS, limits()),
            Err(ProjectError::DuplicateJsonMember(_))
        ));
    }
}
