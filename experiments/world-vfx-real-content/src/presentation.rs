use crate::prepared_cache::{
    PREPARED_CACHE_SCHEMA, SEMANTIC_AUTHORITY_PATH, SEMANTIC_CONTRACT_ID, SOURCE_APPEARANCE_SHA256,
    SOURCE_CATALOG_SHA256, SOURCE_PROFILE_ID, SOURCE_ZIP_SHA256,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PATTERN_DIMENSION: usize = 64;
const MAX_LAYERS: usize = 16;
const MAX_PHASES: usize = 1_024;
const MAX_SPRITE_REFS: usize = 2_000_000;
const EXPECTED_INDEX_ORDER: [&str; 5] = ["phase", "pattern_z", "pattern_y", "pattern_x", "layer"];
const EXPECTED_TIMING_POLICY: &str =
    "source-range-first-nonzero-fallback+deterministic-midpoint-v1";

#[derive(Debug, Deserialize)]
struct ProgramManifest {
    schema: String,
    source: ProgramSource,
    semantic_authority: ProgramAuthority,
    bindings: ProgramBindings,
}

#[derive(Debug, Deserialize)]
struct ProgramSource {
    label: String,
    zip_sha256: String,
    catalog_sha256: String,
    appearance_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ProgramAuthority {
    path: String,
    contract_id: String,
    semantic_revision: u64,
    product_root: String,
}

#[derive(Debug, Deserialize)]
struct ProgramBindings {
    outfit: NormalizedProgram,
    effect: NormalizedProgram,
    missile: NormalizedProgram,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FrameGroupIdentity {
    pub id: u64,
    pub semantic: String,
    #[serde(rename = "type")]
    pub kind: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct PatternShape {
    pub depth: usize,
    pub height: usize,
    pub width: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NormalizedAnimation {
    pub default_start_phase: usize,
    pub duration_ranges_ms: Vec<[u64; 2]>,
    pub effective_duration_ranges_ms: Vec<[u64; 2]>,
    pub loop_count: u64,
    pub loop_type: String,
    pub presentation_durations_ms: Vec<u64>,
    pub random_start_phase: bool,
    pub synchronized: bool,
    pub timing_policy: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NormalizedProgram {
    pub appearance_source_id: u32,
    pub category: String,
    pub frame_group: FrameGroupIdentity,
    pub index_order: Vec<String>,
    pub layers: usize,
    pub patterns: PatternShape,
    pub phase_count: usize,
    pub source_profile_id: String,
    pub sprite_source_ids: Vec<u32>,
    pub animation: Option<NormalizedAnimation>,
    pub program_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramCursor {
    pub phase: usize,
    pub pattern_z: usize,
    pub pattern_y: usize,
    pub pattern_x: usize,
    pub layer: usize,
}

#[derive(Debug, Clone)]
pub struct PreparedPrograms {
    outfit: NormalizedProgram,
    effect: NormalizedProgram,
    missile: NormalizedProgram,
    product_root: String,
}

impl PreparedPrograms {
    pub fn open(manifest_path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(manifest_path)
            .map_err(|error| format!("stat {}: {error}", manifest_path.display()))?;
        if metadata.len() > MAX_MANIFEST_BYTES {
            return Err(format!(
                "prepared manifest exceeds {MAX_MANIFEST_BYTES} bytes: {}",
                metadata.len()
            ));
        }
        let bytes = fs::read(manifest_path)
            .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
        let manifest: ProgramManifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse {}: {error}", manifest_path.display()))?;
        validate_manifest_identity(&manifest)?;

        manifest.bindings.outfit.validate_for_category("outfit")?;
        manifest.bindings.effect.validate_for_category("effect")?;
        manifest.bindings.missile.validate_for_category("missile")?;

        Ok(Self {
            outfit: manifest.bindings.outfit,
            effect: manifest.bindings.effect,
            missile: manifest.bindings.missile,
            product_root: manifest.semantic_authority.product_root,
        })
    }

    pub fn outfit(&self) -> &NormalizedProgram {
        &self.outfit
    }

    pub fn effect(&self) -> &NormalizedProgram {
        &self.effect
    }

    pub fn missile(&self) -> &NormalizedProgram {
        &self.missile
    }

    pub fn product_root(&self) -> &str {
        &self.product_root
    }
}

impl NormalizedProgram {
    pub fn validate_for_category(&self, expected_category: &str) -> Result<(), String> {
        if self.appearance_source_id == 0 {
            return Err(format!(
                "{expected_category} appearance_source_id must be non-zero"
            ));
        }
        if self.category != expected_category {
            return Err(format!(
                "normalized category mismatch: expected {expected_category}, got {}",
                self.category
            ));
        }
        if self.frame_group.semantic.trim().is_empty() {
            return Err(format!("{expected_category} frame-group semantic is empty"));
        }
        if self.index_order.len() != EXPECTED_INDEX_ORDER.len()
            || !self
                .index_order
                .iter()
                .zip(EXPECTED_INDEX_ORDER)
                .all(|(actual, expected)| actual == expected)
        {
            return Err(format!(
                "{expected_category} index_order must be {:?}, got {:?}",
                EXPECTED_INDEX_ORDER, self.index_order
            ));
        }
        for (label, value) in [
            ("pattern depth", self.patterns.depth),
            ("pattern height", self.patterns.height),
            ("pattern width", self.patterns.width),
        ] {
            if value == 0 || value > MAX_PATTERN_DIMENSION {
                return Err(format!(
                    "{expected_category} {label} must be in 1..={MAX_PATTERN_DIMENSION}, got {value}"
                ));
            }
        }
        if self.layers == 0 || self.layers > MAX_LAYERS {
            return Err(format!(
                "{expected_category} layers must be in 1..={MAX_LAYERS}, got {}",
                self.layers
            ));
        }
        if self.phase_count == 0 || self.phase_count > MAX_PHASES {
            return Err(format!(
                "{expected_category} phase_count must be in 1..={MAX_PHASES}, got {}",
                self.phase_count
            ));
        }
        if self.source_profile_id != SOURCE_PROFILE_ID {
            return Err(format!(
                "{expected_category} source_profile_id mismatch: expected {SOURCE_PROFILE_ID}, got {}",
                self.source_profile_id
            ));
        }

        let expected_refs = self
            .phase_count
            .checked_mul(self.patterns.depth)
            .and_then(|value| value.checked_mul(self.patterns.height))
            .and_then(|value| value.checked_mul(self.patterns.width))
            .and_then(|value| value.checked_mul(self.layers))
            .ok_or_else(|| format!("{expected_category} sprite reference geometry overflow"))?;
        if expected_refs > MAX_SPRITE_REFS {
            return Err(format!(
                "{expected_category} requires {expected_refs} sprite refs, cap is {MAX_SPRITE_REFS}"
            ));
        }
        if self.sprite_source_ids.len() != expected_refs {
            return Err(format!(
                "{expected_category} sprite ref count mismatch: expected {expected_refs}, got {}",
                self.sprite_source_ids.len()
            ));
        }
        if self.sprite_source_ids.contains(&0) {
            return Err(format!("{expected_category} contains sprite_source_id 0"));
        }

        match (&self.animation, self.phase_count) {
            (None, 1) => {}
            (Some(animation), phase_count) if phase_count > 1 => {
                validate_animation(expected_category, animation, phase_count)?;
            }
            (None, phase_count) => {
                return Err(format!(
                    "{expected_category} phase_count {phase_count} requires normalized animation timing"
                ));
            }
            (Some(_), 1) => {
                return Err(format!(
                    "{expected_category} single-phase program must have animation=null"
                ));
            }
            (Some(_), _) => {
                return Err(format!(
                    "{expected_category} invalid normalized animation state"
                ));
            }
        }

        let expected_program_id = canonical_program_id(self)?;
        if self.program_id != expected_program_id {
            return Err(format!(
                "{expected_category} program_id mismatch: expected {expected_program_id}, got {}",
                self.program_id
            ));
        }
        Ok(())
    }

    pub fn sprite_source_id(&self, cursor: ProgramCursor) -> Result<u32, String> {
        self.validate_cursor(cursor)?;
        let index = (((cursor.phase * self.patterns.depth + cursor.pattern_z)
            * self.patterns.height
            + cursor.pattern_y)
            * self.patterns.width
            + cursor.pattern_x)
            * self.layers
            + cursor.layer;
        self.sprite_source_ids.get(index).copied().ok_or_else(|| {
            format!(
                "{} cursor resolved index {index} outside {} sprite refs",
                self.category,
                self.sprite_source_ids.len()
            )
        })
    }

    pub fn phase_in_forward_cycle(
        &self,
        elapsed_ms: u64,
        start_phase: usize,
    ) -> Result<usize, String> {
        if self.phase_count == 1 {
            if start_phase != 0 {
                return Err(format!(
                    "{} single-phase start must be 0, got {start_phase}",
                    self.category
                ));
            }
            return Ok(0);
        }
        if start_phase >= self.phase_count {
            return Err(format!(
                "{} start phase {start_phase} outside 0..{}",
                self.category, self.phase_count
            ));
        }
        let animation = self.animation.as_ref().ok_or_else(|| {
            format!(
                "{} animated program is missing normalized timing",
                self.category
            )
        })?;
        if animation.presentation_durations_ms.len() != self.phase_count {
            return Err(format!(
                "{} presentation duration count changed after validation",
                self.category
            ));
        }
        let cycle_ms = animation
            .presentation_durations_ms
            .iter()
            .try_fold(0_u64, |total, duration| total.checked_add(*duration))
            .ok_or_else(|| format!("{} presentation cycle duration overflow", self.category))?;
        if cycle_ms == 0 {
            return Err(format!(
                "{} presentation cycle duration is zero",
                self.category
            ));
        }

        let mut remaining = elapsed_ms % cycle_ms;
        for offset in 0..self.phase_count {
            let phase = (start_phase + offset) % self.phase_count;
            let duration = animation.presentation_durations_ms[phase];
            if remaining < duration {
                return Ok(phase);
            }
            remaining -= duration;
        }
        Err(format!(
            "{} could not resolve phase in validated forward cycle",
            self.category
        ))
    }

    fn validate_cursor(&self, cursor: ProgramCursor) -> Result<(), String> {
        for (label, value, upper) in [
            ("phase", cursor.phase, self.phase_count),
            ("pattern_z", cursor.pattern_z, self.patterns.depth),
            ("pattern_y", cursor.pattern_y, self.patterns.height),
            ("pattern_x", cursor.pattern_x, self.patterns.width),
            ("layer", cursor.layer, self.layers),
        ] {
            if value >= upper {
                return Err(format!(
                    "{} cursor {label}={value} outside 0..{upper}",
                    self.category
                ));
            }
        }
        Ok(())
    }
}

fn validate_animation(
    category: &str,
    animation: &NormalizedAnimation,
    phase_count: usize,
) -> Result<(), String> {
    if animation.default_start_phase >= phase_count {
        return Err(format!(
            "{category} default_start_phase {} outside 0..{phase_count}",
            animation.default_start_phase
        ));
    }
    for (label, ranges) in [
        ("duration_ranges_ms", &animation.duration_ranges_ms),
        (
            "effective_duration_ranges_ms",
            &animation.effective_duration_ranges_ms,
        ),
    ] {
        if ranges.len() != phase_count {
            return Err(format!(
                "{category} {label} count mismatch: expected {phase_count}, got {}",
                ranges.len()
            ));
        }
        for [low, high] in ranges {
            if low > high {
                return Err(format!(
                    "{category} {label} contains inverted range {low}..={high}"
                ));
            }
        }
    }
    if animation
        .effective_duration_ranges_ms
        .iter()
        .any(|[low, high]| *low == 0 || *high == 0)
    {
        return Err(format!(
            "{category} effective duration ranges must be non-zero"
        ));
    }
    if animation.presentation_durations_ms.len() != phase_count
        || animation.presentation_durations_ms.contains(&0)
    {
        return Err(format!(
            "{category} presentation_durations_ms must contain {phase_count} non-zero values"
        ));
    }
    if !matches!(
        animation.loop_type.as_str(),
        "pingpong" | "infinite" | "counted"
    ) {
        return Err(format!(
            "{category} unsupported normalized loop_type {}",
            animation.loop_type
        ));
    }
    if animation.loop_type == "counted" && animation.loop_count == 0 {
        return Err(format!(
            "{category} counted animation requires positive loop_count"
        ));
    }
    if animation.loop_type != "counted" && animation.loop_count != 0 {
        return Err(format!(
            "{category} loop_count is only valid for counted animation"
        ));
    }
    if animation.timing_policy != EXPECTED_TIMING_POLICY {
        return Err(format!(
            "{category} timing_policy mismatch: expected {EXPECTED_TIMING_POLICY}, got {}",
            animation.timing_policy
        ));
    }
    Ok(())
}

fn canonical_program_id(program: &NormalizedProgram) -> Result<String, String> {
    let mut value = serde_json::to_value(program)
        .map_err(|error| format!("serialize normalized program: {error}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "normalized program must serialize as object".to_owned())?;
    object.remove("program_id");
    let mut bytes = serde_json::to_vec(&value)
        .map_err(|error| format!("encode normalized program: {error}"))?;
    bytes.push(b'\n');
    let mut digest = Sha256::new();
    digest.update(bytes);
    Ok(format!(
        "animation-program:sha256:{}",
        hex::encode(digest.finalize())
    ))
}

fn validate_manifest_identity(manifest: &ProgramManifest) -> Result<(), String> {
    if manifest.schema != PREPARED_CACHE_SCHEMA {
        return Err(format!(
            "prepared cache schema mismatch: expected {PREPARED_CACHE_SCHEMA}, got {}",
            manifest.schema
        ));
    }
    if manifest.source.label != "15.32"
        || manifest.source.zip_sha256 != SOURCE_ZIP_SHA256
        || manifest.source.catalog_sha256 != SOURCE_CATALOG_SHA256
        || manifest.source.appearance_sha256 != SOURCE_APPEARANCE_SHA256
    {
        return Err(
            "prepared presentation source identity is not the pinned Tibia 15.32 source".to_owned(),
        );
    }
    if manifest.semantic_authority.path != SEMANTIC_AUTHORITY_PATH
        || manifest.semantic_authority.contract_id != SEMANTIC_CONTRACT_ID
        || manifest.semantic_authority.semantic_revision != 1
    {
        return Err(format!(
            "prepared presentation semantic authority mismatch: expected {SEMANTIC_CONTRACT_ID} revision 1"
        ));
    }
    let product_digest = manifest
        .semantic_authority
        .product_root
        .strip_prefix("sha256:")
        .ok_or_else(|| "prepared presentation product_root must use sha256: prefix".to_owned())?;
    validate_lower_sha256(product_digest, "prepared presentation product_root")
}

fn validate_lower_sha256(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{label} must be a lowercase SHA-256 hex digest"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(category: &str) -> Result<NormalizedProgram, String> {
        let mut program = NormalizedProgram {
            appearance_source_id: 77,
            category: category.to_owned(),
            frame_group: FrameGroupIdentity {
                id: 0,
                semantic: "test".to_owned(),
                kind: 0,
            },
            index_order: EXPECTED_INDEX_ORDER
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            layers: 2,
            patterns: PatternShape {
                depth: 2,
                height: 1,
                width: 2,
            },
            phase_count: 3,
            source_profile_id: SOURCE_PROFILE_ID.to_owned(),
            sprite_source_ids: (100..124).collect(),
            animation: Some(NormalizedAnimation {
                default_start_phase: 1,
                duration_ranges_ms: vec![[100, 100], [200, 200], [300, 300]],
                effective_duration_ranges_ms: vec![[100, 100], [200, 200], [300, 300]],
                loop_count: 0,
                loop_type: "infinite".to_owned(),
                presentation_durations_ms: vec![100, 200, 300],
                random_start_phase: false,
                synchronized: true,
                timing_policy: EXPECTED_TIMING_POLICY.to_owned(),
            }),
            program_id: String::new(),
        };
        program.program_id = canonical_program_id(&program)?;
        Ok(program)
    }

    #[test]
    fn resolves_game_owned_index_order() -> Result<(), String> {
        let program = program("outfit")?;
        program.validate_for_category("outfit")?;
        let sprite_id = program.sprite_source_id(ProgramCursor {
            phase: 1,
            pattern_z: 1,
            pattern_y: 0,
            pattern_x: 1,
            layer: 1,
        })?;
        assert_eq!(sprite_id, 115);
        Ok(())
    }

    #[test]
    fn resolves_forward_phase_from_presentation_durations() -> Result<(), String> {
        let program = program("effect")?;
        assert_eq!(program.phase_in_forward_cycle(0, 1)?, 1);
        assert_eq!(program.phase_in_forward_cycle(199, 1)?, 1);
        assert_eq!(program.phase_in_forward_cycle(200, 1)?, 2);
        assert_eq!(program.phase_in_forward_cycle(499, 1)?, 2);
        assert_eq!(program.phase_in_forward_cycle(500, 1)?, 0);
        assert_eq!(program.phase_in_forward_cycle(600, 1)?, 1);
        Ok(())
    }

    #[test]
    fn rejects_wrong_index_order() -> Result<(), String> {
        let mut program = program("missile")?;
        program.index_order.swap(0, 4);
        program.program_id = canonical_program_id(&program)?;
        let result = program.validate_for_category("missile");
        assert!(result.is_err());
        Ok(())
    }
}
