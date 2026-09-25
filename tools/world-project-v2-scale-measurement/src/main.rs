use oteryn_game_server::content::*;
use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::json;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FULL_PLACEMENTS: usize = 24_502_036;
const SMOKE_PLACEMENTS: usize = 2_000;
const WORLD_KEY: &str = "oteryn:world.scale-probe";
const DEFINITION_KEY: &str = "oteryn:content.world-object.scale-probe";
const MAP_REVISION: &str = "map-scale-probe-r1";
const COORDINATE_FRAME: &str = "global-target-2026-09-24";
const SHARD_SIZE: usize = 500_000;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Smoke,
    Full,
    Sharded,
}

impl Mode {
    fn parse_args() -> Result<Self, String> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        match args.as_slice() {
            [flag, mode] if flag == "--mode" && mode == "smoke" => Ok(Self::Smoke),
            [flag, mode] if flag == "--mode" && mode == "full" => Ok(Self::Full),
            [flag, mode] if flag == "--mode" && mode == "sharded" => Ok(Self::Sharded),
            [] => Err("pass --mode smoke, --mode full, or --mode sharded explicitly".into()),
            _ => Err("only --mode smoke, --mode full, or --mode sharded is accepted; input paths are forbidden".into()),
        }
    }

    fn placements(self) -> usize {
        match self {
            Self::Smoke => SMOKE_PLACEMENTS,
            Self::Full | Self::Sharded => FULL_PLACEMENTS,
        }
    }
}

#[derive(Debug)]
struct Timings {
    build: Duration,
    serialize: Duration,
    syntax_parse: Duration,
    schema_load: Duration,
    staged_write: Duration,
}

fn limits(count: usize) -> ProjectEvidenceLimits {
    let count_limit = count.saturating_add(32);
    ProjectEvidenceLimits {
        max_documents: 16,
        max_document_bytes: 12 * 1024 * 1024 * 1024,
        max_total_bytes: 14 * 1024 * 1024 * 1024,
        max_json_depth: 32,
        max_decoded_fields: count.saturating_mul(32).saturating_add(4_096),
        max_string_bytes: count.saturating_mul(512).saturating_add(4_096),
        max_locator_bytes: 256,
        max_locator_segments: 12,
        max_reference_records: count_limit,
        max_import_records: 8,
        max_reimport_states: 8,
    }
}

fn core() -> ProjectDraft {
    ProjectDraft {
        project_revision: "project-scale-probe-r1".into(),
        package_key: "oteryn:content.world-project-scale-probe".into(),
        semantic_schema_version: "reference-schema-v1".into(),
        licensing_metadata: "license:scale-study-authorized".into(),
        world_id: "0123456789ab70cd8ef0123456789abc".into(),
        coordinate_frame: COORDINATE_FRAME.into(),
        records: vec![ProjectReferenceRecord::Generic {
            identity: DefinitionIdentityDocument {
                family: "Presentation".into(),
                key: "oteryn:reference.presentation.scale-probe".into(),
                revision: "definition-r1".into(),
            },
            client_projection: ProjectionDocument::ClientSafe,
        }],
        imports: vec![],
        metadata: vec![],
    }
}

fn placement(index: usize) -> ProjectV2Placement {
    ProjectV2Placement {
        key: format!("oteryn:placement.scale-probe.{index:08}"),
        world: WORLD_KEY.into(),
        map_revision: MAP_REVISION.into(),
        definition: ProjectV2DefinitionRef {
            family: ProjectV2Family::WorldObject,
            key: DEFINITION_KEY.into(),
            revision: "definition-r1".into(),
        },
        area: None,
        document: None,
        parent_placement: None,
        coordinate_frame: COORDINATE_FRAME.into(),
        x: (index % 1_000_000) as i32,
        y: (index / 1_000_000) as i32,
        floor: 7,
        presentation_order: ProjectV2PresentationOrder {
            plane: 0,
            order: index as u32,
        },
        disposition: ProjectV2Disposition::CandidateOnly,
    }
}

fn build_draft_range(start: usize, count: usize) -> Result<ProjectV2Draft, String> {
    let mut placements = Vec::new();
    placements
        .try_reserve_exact(count)
        .map_err(|error| format!("placement allocation failed: {error}"))?;
    for offset in 0..count {
        placements.push(placement(start + offset));
    }

    Ok(ProjectV2Draft {
        core: core(),
        state: ProjectV2State {
            declarations: vec![ProjectV2Declaration::WorldObject {
                identity: ProjectV2Identity {
                    key: DEFINITION_KEY.into(),
                    revision: "definition-r1".into(),
                },
                presentation: None,
                fields: vec![],
            }],
            item_authoring: vec![],
            authoring_profiles: vec![],
            worlds: vec![ProjectV2World {
                key: WORLD_KEY.into(),
                world_id: "0123456789ab70cd8ef0123456789abc".into(),
                coordinate_frame: COORDINATE_FRAME.into(),
                bounds: ProjectV2Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x_exclusive: 1_000_000,
                    max_y_exclusive: 32,
                },
                floors: vec![7],
            }],
            placements,
            appearance_bindings: vec![],
            assets: vec![],
            sources: vec![],
            source_identity_bindings: vec![],
            editor: vec![],
        },
    })
}

fn build_draft(count: usize) -> Result<ProjectV2Draft, String> {
    build_draft_range(0, count)
}

fn total_bytes(documents: &BTreeMap<String, Vec<u8>>) -> usize {
    documents.values().map(Vec::len).sum()
}

fn role_bytes(documents: &BTreeMap<String, Vec<u8>>) -> BTreeMap<&'static str, usize> {
    let roles = [
        ("reference-records", "definitions/reference.json"),
        ("declarative-definitions", "definitions/declarations.json"),
        ("world-records", "worlds/world.json"),
        ("presentation-bindings", "presentations/bindings.json"),
        ("asset-records", "assets/catalog.json"),
        ("import-candidates", "provenance/imports.json"),
        ("provenance-records", "provenance/sources.json"),
        ("editor-records", "editor/author.json"),
    ];
    roles
        .into_iter()
        .map(|(role, path)| (role, documents.get(path).map_or(0, Vec::len)))
        .collect()
}

fn syntax_parse(documents: &BTreeMap<String, Vec<u8>>) -> Result<(), Box<dyn Error>> {
    for bytes in documents.values() {
        let mut parser = serde_json::Deserializer::from_slice(bytes);
        IgnoredAny::deserialize(&mut parser)?;
        parser.end()?;
    }
    Ok(())
}

fn stage_documents(documents: &BTreeMap<String, Vec<u8>>) -> Result<Duration, Box<dyn Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!(
        "oteryn-world-project-v2-scale-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root)?;
    let started = Instant::now();
    let result = (|| -> Result<(), Box<dyn Error>> {
        for (locator, bytes) in documents {
            let path = root.join(locator);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            writer.write_all(bytes)?;
            writer.flush()?;
        }
        Ok(())
    })();
    let elapsed = started.elapsed();
    let cleanup = fs::remove_dir_all(&root);
    result?;
    cleanup?;
    Ok(elapsed)
}

fn changed_byte_count(left: &[u8], right: &[u8]) -> usize {
    let common = left.len().min(right.len());
    let differences = left[..common]
        .iter()
        .zip(&right[..common])
        .filter(|(a, b)| a != b)
        .count();
    differences + left.len().abs_diff(right.len())
}

fn edit_amplification(total: usize) -> Result<serde_json::Value, Box<dyn Error>> {
    let before = placement(0);
    let mut after = before.clone();
    after.x = 1;
    let before_bytes = serde_json::to_vec(&before)?;
    let after_bytes = serde_json::to_vec(&after)?;
    let changed = changed_byte_count(&before_bytes, &after_bytes).max(1);
    Ok(json!({
        "measurement_status": "ESTIMATE_NOT_GATE_COMPLETE",
        "definition": "estimated full-project rewrite bytes / isolated serialized placement-record byte difference; does not diff two canonical snapshots",
        "changed_paths": null,
        "changed_file_count": null,
        "changed_project_bytes": null,
        "isolated_placement_record_changed_bytes": changed,
        "estimated_full_project_rewrite_bytes": total,
        "estimated_amplification_ratio": total as f64 / changed as f64
    }))
}

#[cfg(target_os = "linux")]
fn peak_rss_bytes() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let kb = status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?.split_whitespace().next()?.parse::<u64>().ok()
    })?;
    Some(kb.saturating_mul(1024))
}

#[cfg(not(target_os = "linux"))]
fn peak_rss_bytes() -> Option<u64> {
    None
}

fn elapsed_ms(value: Duration) -> u128 {
    value.as_millis()
}

fn run(mode: Mode) -> Result<serde_json::Value, (String, String)> {
    if matches!(mode, Mode::Sharded) {
        return run_sharded();
    }
    let count = mode.placements();
    let limits = limits(count);

    let build_started = Instant::now();
    let draft = build_draft(count).map_err(|error| ("build".into(), error))?;
    let build_time = build_started.elapsed();

    let serialize_started = Instant::now();
    let canonical = CanonicalProjectDocuments::from_v2_draft(draft, limits)
        .map_err(|error| ("serialize_validate".into(), error.to_string()))?;
    let serialize_time = serialize_started.elapsed();
    let snapshot = canonical
        .into_snapshot(limits)
        .map_err(|error| ("snapshot".into(), error.to_string()))?;
    let documents = snapshot.documents();

    let parse_started = Instant::now();
    syntax_parse(documents).map_err(|error| ("syntax_parse".into(), error.to_string()))?;
    let syntax_parse_time = parse_started.elapsed();

    let load_started = Instant::now();
    let project = snapshot
        .parse(limits)
        .map_err(|error| ("schema_load".into(), error.to_string()))?;
    let loaded_placements = project.v2().map_or(0, |state| state.placements.len());
    if loaded_placements != count {
        return Err(("schema_load".into(), format!("loaded {loaded_placements} placements; expected {count}")));
    }
    let schema_load_time = load_started.elapsed();

    let staged_write_time = stage_documents(documents)
        .map_err(|error| ("staged_write".into(), error.to_string()))?;
    let total = total_bytes(documents);
    let amplification = edit_amplification(total)
        .map_err(|error| ("edit_amplification".into(), error.to_string()))?;
    let timings = Timings {
        build: build_time,
        serialize: serialize_time,
        syntax_parse: syntax_parse_time,
        schema_load: schema_load_time,
        staged_write: staged_write_time,
    };
    Ok(json!({
        "schema": "OTERYN_WORLD_PROJECT_SCALE_MEASUREMENT/v1",
        "status": "completed",
        "input_class": "SYNTHETIC_SCALE_STRESS_ONLY",
        "mode": match mode { Mode::Smoke => "smoke", Mode::Full => "full", Mode::Sharded => "sharded" },
        "placements_requested": count,
        "placements_loaded": loaded_placements,
        "synthetic_definitions": 1,
        "worlds": 1,
        "map_revisions": 1,
        "coordinate_frames": 1,
        "placement_disposition": "CandidateOnly",
        "donor_inputs_accepted": false,
        "source_inputs_accepted": false,
        "role_bytes": role_bytes(documents),
        "total_bytes": total,
        "timings_ms": {
            "build": elapsed_ms(timings.build),
            "serialize_and_canonical_validate": elapsed_ms(timings.serialize),
            "syntax_parse": elapsed_ms(timings.syntax_parse),
            "schema_load": elapsed_ms(timings.schema_load),
            "staged_write": elapsed_ms(timings.staged_write)
        },
        "peak_rss_bytes": peak_rss_bytes(),
        "peak_rss_source": "probe self-report; supervised full run replaces this with /usr/bin/time -v",
        "one_placement_edit_amplification": amplification,
        "schema_and_chunk_decision": "out_of_scope"
    }))
}


fn shard_ranges(total: usize, shard_size: usize) -> Result<Vec<(usize, usize)>, String> {
    if total == 0 || shard_size == 0 {
        return Err("total placements and shard size must be positive".into());
    }
    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < total {
        let end = start.saturating_add(shard_size).min(total);
        ranges.push((start, end));
        start = end;
    }
    Ok(ranges)
}

fn validate_identity_range(placements: &[ProjectV2Placement], start: usize) -> Result<(), String> {
    for (offset, record) in placements.iter().enumerate() {
        let expected = format!("oteryn:placement.scale-probe.{:08}", start + offset);
        if record.key != expected {
            return Err(format!(
                "identity coverage mismatch at offset {offset}: expected {expected}, found {}",
                record.key
            ));
        }
    }
    Ok(())
}

fn run_sharded() -> Result<serde_json::Value, (String, String)> {
    let ranges = shard_ranges(FULL_PLACEMENTS, SHARD_SIZE)
        .map_err(|error| ("shard_plan".into(), error))?;
    let mut completed = Vec::with_capacity(ranges.len());
    let mut role_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut aggregate_bytes = 0u64;
    let mut aggregate_timings = [0u128; 5];
    let mut placements_loaded = 0usize;

    for (shard_index, (start, end)) in ranges.iter().copied().enumerate() {
        let count = end - start;
        let shard_limits = limits(count);
        let build_started = Instant::now();
        let draft = build_draft_range(start, count)
            .map_err(|error| ("build".into(), format!("shard {shard_index}: {error}")))?;
        let build_time = build_started.elapsed();

        let serialize_started = Instant::now();
        let canonical = CanonicalProjectDocuments::from_v2_draft(draft, shard_limits)
            .map_err(|error| ("serialize_validate".into(), format!("shard {shard_index}: {error}")))?;
        let serialize_time = serialize_started.elapsed();
        let snapshot = canonical
            .into_snapshot(shard_limits)
            .map_err(|error| ("snapshot".into(), format!("shard {shard_index}: {error}")))?;
        let documents = snapshot.documents();

        let parse_started = Instant::now();
        syntax_parse(documents)
            .map_err(|error| ("syntax_parse".into(), format!("shard {shard_index}: {error}")))?;
        let syntax_parse_time = parse_started.elapsed();

        let load_started = Instant::now();
        let project = snapshot
            .parse(shard_limits)
            .map_err(|error| ("schema_load".into(), format!("shard {shard_index}: {error}")))?;
        let loaded = project.v2().map_or(0, |state| state.placements.len());
        if loaded != count {
            return Err((
                "schema_load".into(),
                format!("shard {shard_index} loaded {loaded}; expected {count}"),
            ));
        }
        validate_identity_range(&project.v2().expect("v2 snapshot").placements, start)
            .map_err(|error| ("coverage".into(), format!("shard {shard_index}: {error}")))?;
        let schema_load_time = load_started.elapsed();

        let staged_write_time = stage_documents(documents)
            .map_err(|error| ("staged_write".into(), format!("shard {shard_index}: {error}")))?;
        let bytes = total_bytes(documents) as u64;
        let roles = role_bytes(documents);
        for (role, value) in &roles {
            *role_totals.entry((*role).to_string()).or_default() += *value as u64;
        }

        let phases = [
            elapsed_ms(build_time),
            elapsed_ms(serialize_time),
            elapsed_ms(syntax_parse_time),
            elapsed_ms(schema_load_time),
            elapsed_ms(staged_write_time),
        ];
        for (total, phase) in aggregate_timings.iter_mut().zip(phases) {
            *total += phase;
        }
        aggregate_bytes += bytes;
        placements_loaded += loaded;
        completed.push(json!({
            "shard_index": shard_index,
            "placement_start_inclusive": start,
            "placement_end_exclusive": end,
            "placements_requested": count,
            "placements_loaded": loaded,
            "identity_coverage_verified": true,
            "role_bytes": roles,
            "total_bytes": bytes,
            "timings_ms": {
                "build": phases[0],
                "serialize_and_canonical_validate": phases[1],
                "syntax_parse": phases[2],
                "schema_load": phases[3],
                "staged_write": phases[4]
            }
        }));
    }

    if placements_loaded != FULL_PLACEMENTS {
        return Err((
            "coverage".into(),
            format!("loaded {placements_loaded}; expected {FULL_PLACEMENTS}"),
        ));
    }
    Ok(json!({
        "schema": "OTERYN_WORLD_PROJECT_SCALE_MEASUREMENT/v1",
        "status": "completed",
        "input_class": "SYNTHETIC_SCALE_STRESS_ONLY",
        "mode": "sharded",
        "placements_requested": FULL_PLACEMENTS,
        "placements_loaded": placements_loaded,
        "coverage": {
            "method": "contiguous half-open global identity ranges; every loaded identity checked against its expected global ordinal",
            "shard_size_target": SHARD_SIZE,
            "shards_expected": ranges.len(),
            "shards_completed": completed.len(),
            "no_duplicate_or_missing_identities": true,
            "first_placement_index": 0,
            "last_placement_index_inclusive": FULL_PLACEMENTS - 1
        },
        "synthetic_definitions": 1,
        "worlds": 1,
        "map_revisions": 1,
        "coordinate_frames": 1,
        "placement_disposition": "CandidateOnly",
        "donor_inputs_accepted": false,
        "source_inputs_accepted": false,
        "role_bytes": role_totals,
        "total_bytes": aggregate_bytes,
        "timings_ms": {
            "build": aggregate_timings[0],
            "serialize_and_canonical_validate": aggregate_timings[1],
            "syntax_parse": aggregate_timings[2],
            "schema_load": aggregate_timings[3],
            "staged_write": aggregate_timings[4]
        },
        "max_supervised_shard_peak_rss_bytes": null,
        "peak_rss_source": "pending workflow /usr/bin/time -v measurement of the sequential shard runner",
        "monolithic_full_world_peak_rss": "NOT_MEASURED_BY_SHARDED_RUN",
        "full_world_edit_amplification": "NOT_MEASURED; no canonical full-world snapshot diff is performed",
        "production_layout_selected": false,
        "limitation": "Synthetic bounded sequential shards exercise the real v2 canonicalize/validate/load/staged-write path for all 24,502,036 identities. This is not a monolithic full-world peak-RSS measurement, not full-world edit amplification, and does not select a production shard layout.",
        "shards": completed,
        "schema_and_chunk_decision": "out_of_scope"
    }))
}

fn main() {
    let mode = match Mode::parse_args() {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(64);
        }
    };
    match run(mode) {
        Ok(result) => println!("{}", serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{\"status\":\"measurement_error\"}".into())),
        Err((stage, message)) => {
            println!("{}", json!({
                "schema": "OTERYN_WORLD_PROJECT_SCALE_MEASUREMENT/v1",
                "status": "resource_limit",
                "input_class": "SYNTHETIC_SCALE_STRESS_ONLY",
                "mode": match mode { Mode::Smoke => "smoke", Mode::Full => "full", Mode::Sharded => "sharded" },
                "placements_requested": mode.placements(),
                "peak_rss_bytes": peak_rss_bytes(),
                "peak_rss_source": "probe self-report; supervised full run replaces this with /usr/bin/time -v",
                "one_placement_edit_amplification": {
                    "measurement_status": "ESTIMATE_NOT_GATE_COMPLETE",
                    "changed_paths": null,
                    "changed_file_count": null,
                    "changed_project_bytes": null,
                    "estimate_status": "unavailable_after_resource_limited_failure"
                },
                "failed_stage": stage,
                "error": message,
                "schema_and_chunk_decision": "out_of_scope"
            }));
            std::process::exit(75);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_ranges_are_contiguous_complete_and_non_overlapping() {
        let ranges = shard_ranges(29, 8).unwrap();
        assert_eq!(ranges, vec![(0, 8), (8, 16), (16, 24), (24, 29)]);
        let mut visited = vec![false; 29];
        for (start, end) in ranges {
            for index in start..end {
                assert!(!visited[index], "duplicate index {index}");
                visited[index] = true;
            }
        }
        assert!(visited.into_iter().all(|seen| seen));
    }

    #[test]
    fn target_plan_covers_exact_full_cardinality() {
        let ranges = shard_ranges(FULL_PLACEMENTS, SHARD_SIZE).unwrap();
        assert_eq!(ranges.first(), Some(&(0, SHARD_SIZE)));
        assert_eq!(ranges.last().map(|range| range.1), Some(FULL_PLACEMENTS));
        assert_eq!(ranges.len(), FULL_PLACEMENTS.div_ceil(SHARD_SIZE));
        assert!(ranges.windows(2).all(|pair| pair[0].1 == pair[1].0));
        assert!(ranges.iter().all(|(start, end)| end > start && end - start <= SHARD_SIZE));
        assert_eq!(ranges.iter().map(|(start, end)| end - start).sum::<usize>(), FULL_PLACEMENTS);
    }

    #[test]
    fn identity_validator_rejects_duplicate_or_missing_ordinals() {
        let valid = vec![placement(10), placement(11), placement(12)];
        assert!(validate_identity_range(&valid, 10).is_ok());
        let duplicate = vec![placement(10), placement(10), placement(12)];
        assert!(validate_identity_range(&duplicate, 10).is_err());
        let missing = vec![placement(10), placement(12)];
        assert!(validate_identity_range(&missing, 10).is_err());
    }

    #[test]
    fn shard_planner_rejects_empty_ranges() {
        assert!(shard_ranges(0, SHARD_SIZE).is_err());
        assert!(shard_ranges(FULL_PLACEMENTS, 0).is_err());
    }
}
