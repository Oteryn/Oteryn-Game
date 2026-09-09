use crate::model::{AnimationEvaluationEvidence, FrameSemanticStats};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Percentiles {
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameSample {
    pub frame: Duration,
    pub prep: Duration,
    pub batches: usize,
    pub primitives: usize,
    pub upload_cpu: Duration,
    pub active_resource_pages: usize,
    pub camera_moved: bool,
    pub zoom_changed: bool,
    pub floor_changed: bool,
    pub gameplay_signature: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Samples {
    frame_ms: Vec<f64>,
    prep_ms: Vec<f64>,
    batches: Vec<f64>,
    primitives: Vec<f64>,
    upload_cpu_ms: Vec<f64>,
    active_resource_pages: Vec<f64>,
    camera_scroll_ms: Vec<f64>,
    zoom_change_ms: Vec<f64>,
    floor_transition_ms: Vec<f64>,
    gameplay_signature_xor: u64,
    pub max_semantics: FrameSemanticStats,
}

impl Samples {
    pub fn record(&mut self, sample: FrameSample, semantic: &FrameSemanticStats) {
        self.frame_ms.push(sample.frame.as_secs_f64() * 1000.0);
        self.prep_ms.push(sample.prep.as_secs_f64() * 1000.0);
        self.batches.push(sample.batches as f64);
        self.primitives.push(sample.primitives as f64);
        self.upload_cpu_ms
            .push(sample.upload_cpu.as_secs_f64() * 1000.0);
        self.active_resource_pages
            .push(sample.active_resource_pages as f64);
        if sample.camera_moved {
            self.camera_scroll_ms
                .push(sample.frame.as_secs_f64() * 1000.0);
        }
        if sample.zoom_changed {
            self.zoom_change_ms
                .push(sample.frame.as_secs_f64() * 1000.0);
        }
        if sample.floor_changed {
            self.floor_transition_ms
                .push(sample.frame.as_secs_f64() * 1000.0);
        }
        self.gameplay_signature_xor ^= sample.gameplay_signature;
        merge_semantics(&mut self.max_semantics, semantic);
    }

    pub fn len(&self) -> usize {
        self.frame_ms.len()
    }

    pub fn frame(&self) -> Percentiles {
        summarize(&self.frame_ms)
    }

    pub fn prep(&self) -> Percentiles {
        summarize(&self.prep_ms)
    }

    pub fn batches(&self) -> Percentiles {
        summarize(&self.batches)
    }

    pub fn primitives(&self) -> Percentiles {
        summarize(&self.primitives)
    }

    pub fn upload_cpu(&self) -> Percentiles {
        summarize(&self.upload_cpu_ms)
    }

    pub fn active_resource_pages(&self) -> Percentiles {
        summarize(&self.active_resource_pages)
    }

    pub fn camera_scroll(&self) -> Percentiles {
        summarize(&self.camera_scroll_ms)
    }

    pub fn zoom_change(&self) -> Percentiles {
        summarize(&self.zoom_change_ms)
    }

    pub fn floor_transition(&self) -> Percentiles {
        summarize(&self.floor_transition_ms)
    }

    pub const fn gameplay_signature_xor(&self) -> u64 {
        self.gameplay_signature_xor
    }
}

fn merge_semantics(target: &mut FrameSemanticStats, source: &FrameSemanticStats) {
    target.total_primitives = target.total_primitives.max(source.total_primitives);
    target.world_primitives = target.world_primitives.max(source.world_primitives);
    target.creatures = target.creatures.max(source.creatures);
    target.effects = target.effects.max(source.effects);
    target.projectiles = target.projectiles.max(source.projectiles);
    target.particles = target.particles.max(source.particles);
    target.lights = target.lights.max(source.lights);
    target.overlays = target.overlays.max(source.overlays);
    target.critical_vfx = target.critical_vfx.max(source.critical_vfx);
    target.fallback_variants = target.fallback_variants.max(source.fallback_variants);
    target.weather_suppressed_under_roof = target
        .weather_suppressed_under_roof
        .max(source.weather_suppressed_under_roof);
    target.lower_floor_hole_primitives = target
        .lower_floor_hole_primitives
        .max(source.lower_floor_hole_primitives);
}

fn summarize(values: &[f64]) -> Percentiles {
    if values.is_empty() {
        return Percentiles::default();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Percentiles {
        mean: sorted.iter().sum::<f64>() / sorted.len() as f64,
        p50: percentile_sorted(&sorted, 0.50),
        p95: percentile_sorted(&sorted, 0.95),
        p99: percentile_sorted(&sorted, 0.99),
        max: *sorted.last().unwrap_or(&0.0),
    }
}

fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let last = sorted.len() - 1;
    let index = ((last as f64) * q).round() as usize;
    sorted[index.min(last)]
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheCounters {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub uploads: u64,
    pub upload_bytes: u64,
    pub overflow_fallbacks: u64,
}

impl CacheCounters {
    pub fn delta_from(&self, previous: &Self) -> Self {
        Self {
            hits: self.hits.saturating_sub(previous.hits),
            misses: self.misses.saturating_sub(previous.misses),
            evictions: self.evictions.saturating_sub(previous.evictions),
            uploads: self.uploads.saturating_sub(previous.uploads),
            upload_bytes: self.upload_bytes.saturating_sub(previous.upload_bytes),
            overflow_fallbacks: self
                .overflow_fallbacks
                .saturating_sub(previous.overflow_fallbacks),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterEvidence {
    pub name: String,
    pub driver: String,
    pub driver_info: String,
    pub backend: String,
    pub device_type: String,
    pub max_texture_array_layers: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorpusEvidence {
    pub object_appearances: u32,
    pub outfit_appearances: u32,
    pub effect_appearances: u32,
    pub missile_appearances: u32,
    pub animated_object_appearances: u32,
    pub animated_outfit_appearances: u32,
    pub animated_effect_appearances: u32,
    pub animated_missile_appearances: u32,
    pub unique_world_sprite_ids: u32,
    pub source_label: String,
    pub source_zip_sha256: String,
    pub source_catalog_sha256: String,
    pub source_appearance_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetEvidence {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub expected_sha256: String,
    pub matches_expected: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReliabilityEvidence {
    pub surface_timeout: u64,
    pub surface_occluded: u64,
    pub surface_outdated: u64,
    pub surface_lost: u64,
    pub device_loss_detected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub schema: &'static str,
    pub backend: &'static str,
    pub scenario: String,
    pub resource_mode: String,
    pub presentation_family: String,
    pub density: u32,
    pub width: u32,
    pub height: u32,
    pub warmup_frames: u64,
    pub sample_frames: u64,
    pub seed: u64,
    pub startup_ms: f64,
    pub first_frame_ms: f64,
    pub surface_format: String,
    pub present_mode: String,
    pub available_present_modes: Vec<String>,
    pub sampler: String,
    pub surface_is_srgb: bool,
    pub adapter: AdapterEvidence,
    pub frame_ms: Percentiles,
    pub gpu_frame_ms: Option<Percentiles>,
    pub prep_ms: Percentiles,
    pub order_preserving_batches: Percentiles,
    pub visible_primitives: Percentiles,
    pub upload_submit_cpu_ms: Percentiles,
    pub active_resource_pages: Percentiles,
    pub camera_scroll_stall_ms: Percentiles,
    pub zoom_change_stall_ms: Percentiles,
    pub floor_transition_stall_ms: Percentiles,
    pub gameplay_signature_xor: String,
    pub mean_fps: f64,
    pub cache: CacheCounters,
    pub cache_resident_pages: usize,
    pub cache_capacity_pages: usize,
    pub cache_estimated_gpu_bytes: u64,
    pub max_semantics: FrameSemanticStats,
    pub corpus: CorpusEvidence,
    pub asset: Option<AssetEvidence>,
    pub gpu_timestamp_reliable: bool,
    pub vram_measurement_reliable: bool,
    pub reliability: ReliabilityEvidence,
    pub pipeline_prewarm_ms: f64,
    pub animation_evaluation: AnimationEvaluationEvidence,
}
