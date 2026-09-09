use std::fmt::{self, Display, Formatter};
use std::time::Duration;

pub const FRAME_COUNT: u32 = 36;
pub const ATLAS_COLUMNS: u32 = 6;
pub const ATLAS_ROWS: u32 = 6;
pub const LOGICAL_SPRITE_PX: f32 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Basic,
    Normal,
    Stress,
}

impl Scenario {
    #[must_use]
    pub const fn static_quads(self) -> usize {
        match self {
            Self::Basic => 10_000,
            Self::Normal => 25_000,
            Self::Stress => 50_000,
        }
    }

    #[must_use]
    pub const fn animated_quads(self) -> usize {
        match self {
            Self::Basic => 50,
            Self::Normal => 200,
            Self::Stress => 500,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Normal => "normal",
            Self::Stress => "stress",
        }
    }
}

impl Display for Scenario {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub scenario: Scenario,
    pub sprite_px: u32,
    pub warmup_frames: u64,
    pub sample_frames: u64,
    pub width: u32,
    pub height: u32,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            scenario: Scenario::Normal,
            sprite_px: 32,
            warmup_frames: 180,
            sample_frames: 900,
            width: 1920,
            height: 1080,
        }
    }
}

impl BenchConfig {
    pub fn from_env_args() -> Result<Self, String> {
        let mut config = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--scenario" => {
                    config.scenario = match value.as_str() {
                        "basic" => Scenario::Basic,
                        "normal" => Scenario::Normal,
                        "stress" => Scenario::Stress,
                        _ => return Err(format!("unsupported scenario: {value}")),
                    };
                }
                "--sprite-px" => {
                    config.sprite_px = parse_u32(&value, "sprite-px")?;
                    if !matches!(config.sprite_px, 32 | 64 | 128) {
                        return Err("sprite-px must be 32, 64 or 128".to_owned());
                    }
                }
                "--warmup" => config.warmup_frames = parse_u64(&value, "warmup")?,
                "--frames" => config.sample_frames = parse_u64(&value, "frames")?,
                "--width" => config.width = parse_u32(&value, "width")?,
                "--height" => config.height = parse_u32(&value, "height")?,
                _ => return Err(format!("unsupported argument: {flag}")),
            }
        }
        if config.sample_frames == 0 || config.width == 0 || config.height == 0 {
            return Err("frames, width and height must be non-zero".to_owned());
        }
        Ok(config)
    }

    #[must_use]
    pub const fn total_frames(&self) -> u64 {
        self.warmup_frames + self.sample_frames
    }
}

fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {label} value {value}: {error}"))
}

fn parse_u64(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label} value {value}: {error}"))
}

#[derive(Debug, Clone, Copy)]
pub struct RenderQuad {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub logical_size: f32,
    pub frame: u32,
    pub phase: u32,
}

#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub static_quads: Vec<RenderQuad>,
    pub animated_quads: Vec<RenderQuad>,
}

#[must_use]
pub fn build_snapshot(config: &BenchConfig) -> RenderSnapshot {
    RenderSnapshot {
        static_quads: build_quads(config, config.scenario.static_quads(), 0x9E37_79B9, false),
        animated_quads: build_quads(
            config,
            config.scenario.animated_quads(),
            0x85EB_CA6B,
            true,
        ),
    }
}

fn build_quads(config: &BenchConfig, count: usize, seed: u32, animated: bool) -> Vec<RenderQuad> {
    let width = config.width as f32;
    let height = config.height as f32;
    let mut state = seed;
    let mut quads = Vec::with_capacity(count);
    for index in 0..count {
        state = xorshift32(state.wrapping_add(index as u32).wrapping_add(1));
        let x = (state % config.width) as f32 - (width * 0.5);
        state = xorshift32(state ^ 0xA511_E9B3);
        let y = (state % config.height) as f32 - (height * 0.5);
        let frame = state % FRAME_COUNT;
        let phase = if animated {
            xorshift32(state ^ 0x63D8_3595) % FRAME_COUNT
        } else {
            0
        };
        quads.push(RenderQuad {
            x,
            y,
            z: (index % 1024) as f32 / 1024.0,
            logical_size: LOGICAL_SPRITE_PX,
            frame,
            phase,
        });
    }
    quads
}

const fn xorshift32(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

#[must_use]
pub const fn animation_frame(phase: u32, frame_number: u64) -> u32 {
    (phase + ((frame_number / 4) as u32)) % FRAME_COUNT
}

#[must_use]
pub fn synthetic_atlas_rgba8(sprite_px: u32) -> Vec<u8> {
    let width = sprite_px * ATLAS_COLUMNS;
    let height = sprite_px * ATLAS_ROWS;
    let mut bytes = vec![0_u8; (width as usize) * (height as usize) * 4];
    for frame in 0..FRAME_COUNT {
        let cell_x = (frame % ATLAS_COLUMNS) * sprite_px;
        let cell_y = (frame / ATLAS_COLUMNS) * sprite_px;
        let base_r = ((frame * 47 + 31) % 223 + 32) as u8;
        let base_g = ((frame * 83 + 17) % 223 + 32) as u8;
        let base_b = ((frame * 29 + 71) % 223 + 32) as u8;
        for local_y in 0..sprite_px {
            for local_x in 0..sprite_px {
                let x = cell_x + local_x;
                let y = cell_y + local_y;
                let offset = ((y * width + x) * 4) as usize;
                let border = local_x == 0
                    || local_y == 0
                    || local_x + 1 == sprite_px
                    || local_y + 1 == sprite_px;
                let checker = ((local_x / 4) + (local_y / 4) + frame) % 2 == 0;
                bytes[offset] = if border { 255 } else if checker { base_r } else { base_r / 2 };
                bytes[offset + 1] = if border { 255 } else if checker { base_g } else { base_g / 2 };
                bytes[offset + 2] = if border { 255 } else if checker { base_b } else { base_b / 2 };
                bytes[offset + 3] = 255;
            }
        }
    }
    bytes
}

#[derive(Debug, Default)]
pub struct BenchSamples {
    frame_ms: Vec<f64>,
    prep_ms: Vec<f64>,
}

impl BenchSamples {
    pub fn record(&mut self, frame: Duration, prep: Duration) {
        self.frame_ms.push(frame.as_secs_f64() * 1000.0);
        self.prep_ms.push(prep.as_secs_f64() * 1000.0);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.frame_ms.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frame_ms.is_empty()
    }

    #[must_use]
    pub fn summary(&self) -> SampleSummary {
        SampleSummary {
            frame_mean_ms: mean(&self.frame_ms),
            frame_p50_ms: percentile(&self.frame_ms, 0.50),
            frame_p95_ms: percentile(&self.frame_ms, 0.95),
            frame_p99_ms: percentile(&self.frame_ms, 0.99),
            prep_mean_ms: mean(&self.prep_ms),
            prep_p95_ms: percentile(&self.prep_ms, 0.95),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SampleSummary {
    pub frame_mean_ms: f64,
    pub frame_p50_ms: f64,
    pub frame_p95_ms: f64,
    pub frame_p99_ms: f64,
    pub prep_mean_ms: f64,
    pub prep_p95_ms: f64,
}

impl SampleSummary {
    #[must_use]
    pub fn mean_fps(self) -> f64 {
        if self.frame_mean_ms > 0.0 {
            1000.0 / self.frame_mean_ms
        } else {
            0.0
        }
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let last = ordered.len() - 1;
    let index = ((last as f64) * quantile).round() as usize;
    ordered[index.min(last)]
}

#[must_use]
pub fn result_json(
    backend: &str,
    config: &BenchConfig,
    samples: &BenchSamples,
    startup: Duration,
    adapter: &str,
    draw_calls: Option<u64>,
) -> String {
    let summary = samples.summary();
    let draw_calls = draw_calls.map_or_else(|| "null".to_owned(), |value| value.to_string());
    format!(
        "{{\"backend\":\"{}\",\"scenario\":\"{}\",\"sprite_px\":{},\"static_quads\":{},\"animated_quads\":{},\"warmup_frames\":{},\"sample_frames\":{},\"window\":\"{}x{}\",\"startup_ms\":{:.4},\"frame_mean_ms\":{:.4},\"frame_p50_ms\":{:.4},\"frame_p95_ms\":{:.4},\"frame_p99_ms\":{:.4},\"mean_fps\":{:.2},\"prep_mean_ms\":{:.4},\"prep_p95_ms\":{:.4},\"draw_calls\":{},\"adapter\":\"{}\"}}",
        json_escape(backend),
        config.scenario,
        config.sprite_px,
        config.scenario.static_quads(),
        config.scenario.animated_quads(),
        config.warmup_frames,
        config.sample_frames,
        config.width,
        config.height,
        startup.as_secs_f64() * 1000.0,
        summary.frame_mean_ms,
        summary.frame_p50_ms,
        summary.frame_p95_ms,
        summary.frame_p99_ms,
        summary.mean_fps(),
        summary.prep_mean_ms,
        summary.prep_p95_ms,
        draw_calls,
        json_escape(adapter),
    )
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenarios_have_stable_workload_sizes() {
        assert_eq!(Scenario::Basic.static_quads(), 10_000);
        assert_eq!(Scenario::Normal.animated_quads(), 200);
        assert_eq!(Scenario::Stress.static_quads(), 50_000);
    }

    #[test]
    fn snapshot_generation_is_deterministic() {
        let config = BenchConfig::default();
        let left = build_snapshot(&config);
        let right = build_snapshot(&config);
        assert_eq!(left.static_quads.len(), right.static_quads.len());
        assert_eq!(left.animated_quads.len(), right.animated_quads.len());
        assert_eq!(left.static_quads[17].x, right.static_quads[17].x);
        assert_eq!(left.animated_quads[9].frame, right.animated_quads[9].frame);
    }

    #[test]
    fn atlas_size_scales_with_density() {
        assert_eq!(synthetic_atlas_rgba8(32).len(), 192 * 192 * 4);
        assert_eq!(synthetic_atlas_rgba8(128).len(), 768 * 768 * 4);
    }
}
