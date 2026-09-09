use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const RESOURCE_SPRITES_PER_PAGE: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Basic,
    Normal,
    Stress,
}

impl Scenario {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "basic" => Ok(Self::Basic),
            "normal" => Ok(Self::Normal),
            "stress" => Ok(Self::Stress),
            other => Err(format!("unsupported scenario: {other}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Normal => "normal",
            Self::Stress => "stress",
        }
    }

    pub const fn cache_slots(self) -> usize {
        match self {
            Self::Basic => 32,
            Self::Normal => 48,
            Self::Stress => 88,
        }
    }

    pub const fn creature_count(self) -> usize {
        match self {
            Self::Basic => 12,
            Self::Normal => 60,
            Self::Stress => 220,
        }
    }

    pub const fn projectile_count(self) -> usize {
        match self {
            Self::Basic => 12,
            Self::Normal => 80,
            Self::Stress => 320,
        }
    }

    pub const fn weather_particles(self) -> usize {
        match self {
            Self::Basic => 180,
            Self::Normal => 1_000,
            Self::Stress => 4_500,
        }
    }

    pub const fn ambient_particles(self) -> usize {
        match self {
            Self::Basic => 80,
            Self::Normal => 500,
            Self::Stress => 2_500,
        }
    }

    pub const fn telegraph_count(self) -> usize {
        match self {
            Self::Basic => 2,
            Self::Normal => 10,
            Self::Stress => 36,
        }
    }

    pub const fn light_count(self) -> usize {
        match self {
            Self::Basic => 12,
            Self::Normal => 48,
            Self::Stress => 128,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceMode {
    Atlas,
    Array,
    Hybrid,
}

impl ResourceMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "atlas" => Ok(Self::Atlas),
            "array" => Ok(Self::Array),
            "hybrid" => Ok(Self::Hybrid),
            other => Err(format!("unsupported resource mode: {other}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Atlas => "atlas",
            Self::Array => "array",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Classic,
    Enhanced,
    Hd,
}

impl Family {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "classic" => Ok(Self::Classic),
            "enhanced" => Ok(Self::Enhanced),
            "hd" => Ok(Self::Hd),
            other => Err(format!("unsupported presentation family: {other}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Enhanced => "enhanced",
            Self::Hd => "hd",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub scenario: Scenario,
    pub resource_mode: ResourceMode,
    pub family: Family,
    pub density: u32,
    pub width: u32,
    pub height: u32,
    pub warmup_frames: u64,
    pub sample_frames: u64,
    pub seed: u64,
    pub asset_zip: Option<PathBuf>,
    pub census_path: PathBuf,
    pub preview: bool,
    pub fixed_zoom: Option<f32>,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            scenario: Scenario::Normal,
            resource_mode: ResourceMode::Atlas,
            family: Family::Enhanced,
            density: 64,
            width: 1_920,
            height: 1_080,
            warmup_frames: 180,
            sample_frames: 900,
            seed: 0x04f5_4455_2594_e480_u64,
            asset_zip: None,
            census_path: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/contracts/OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json"),
            preview: false,
            fixed_zoom: None,
        }
    }
}

impl BenchConfig {
    pub fn from_args() -> Result<Self, String> {
        let mut config = Self::default();
        let mut args = env::args().skip(1);
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--preview" => config.preview = true,
                "--scenario" => {
                    config.scenario = Scenario::parse(&next_value(&mut args, &flag)?)?;
                }
                "--resource-mode" => {
                    config.resource_mode = ResourceMode::parse(&next_value(&mut args, &flag)?)?;
                }
                "--family" => {
                    config.family = Family::parse(&next_value(&mut args, &flag)?)?;
                }
                "--density" => {
                    config.density = parse_u32(&next_value(&mut args, &flag)?, "density")?;
                    if !matches!(config.density, 32 | 64 | 128) {
                        return Err("density must be 32, 64 or 128".to_owned());
                    }
                }
                "--width" => config.width = parse_u32(&next_value(&mut args, &flag)?, "width")?,
                "--height" => config.height = parse_u32(&next_value(&mut args, &flag)?, "height")?,
                "--warmup" => {
                    config.warmup_frames = parse_u64(&next_value(&mut args, &flag)?, "warmup")?;
                }
                "--frames" => {
                    config.sample_frames = parse_u64(&next_value(&mut args, &flag)?, "frames")?;
                }
                "--seed" => config.seed = parse_u64(&next_value(&mut args, &flag)?, "seed")?,
                "--asset-zip" => {
                    config.asset_zip = Some(PathBuf::from(next_value(&mut args, &flag)?))
                }
                "--census" => config.census_path = PathBuf::from(next_value(&mut args, &flag)?),
                "--zoom" => {
                    let raw = next_value(&mut args, &flag)?;
                    let zoom = raw
                        .parse::<f32>()
                        .map_err(|error| format!("invalid zoom {raw}: {error}"))?;
                    if !(0.35..=2.5).contains(&zoom) {
                        return Err("zoom must be between 0.35 and 2.5".to_owned());
                    }
                    config.fixed_zoom = Some(zoom);
                }
                other => return Err(format!("unsupported argument: {other}")),
            }
        }

        if config.width == 0 || config.height == 0 || config.sample_frames == 0 {
            return Err("width, height and sample frames must be non-zero".to_owned());
        }
        Ok(config)
    }

    pub const fn total_frames(&self) -> u64 {
        self.warmup_frames.saturating_add(self.sample_frames)
    }

    pub const fn cache_slots(&self) -> usize {
        let base = self.scenario.cache_slots();
        match (self.resource_mode, self.scenario) {
            (ResourceMode::Hybrid, Scenario::Basic) => 64,
            (ResourceMode::Hybrid, Scenario::Normal) => 96,
            (ResourceMode::Hybrid, Scenario::Stress) => 176,
            (ResourceMode::Atlas | ResourceMode::Array, _) => base,
        }
    }

    pub const fn family(&self) -> Family {
        self.family
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
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

#[derive(Debug, Clone, Deserialize)]
pub struct CorpusCensus {
    pub census: CensusFamilies,
    pub source: CensusSource,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CensusFamilies {
    pub object: FamilyCensus,
    pub outfit: FamilyCensus,
    pub effect: FamilyCensus,
    pub missile: FamilyCensus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FamilyCensus {
    pub appearances: u32,
    #[serde(default)]
    pub animated_appearances: u32,
    pub unique_sprite_ids: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CensusSource {
    pub label: String,
    pub zip_sha256: String,
    pub catalog_sha256: String,
    pub appearance_sha256: String,
}

impl CorpusCensus {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("read census {}: {error}", path.display()))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse census {}: {error}", path.display()))
    }

    pub const fn total_appearances(&self) -> u32 {
        self.census.object.appearances
            + self.census.outfit.appearances
            + self.census.effect.appearances
            + self.census.missile.appearances
    }
}

#[derive(Debug, Clone)]
pub struct AssetProof {
    pub path: PathBuf,
    pub sha256: String,
    pub expected_sha256: String,
    pub matches_expected: bool,
    pub bytes: u64,
}

pub fn hash_asset_zip(path: &Path, expected: &str) -> Result<AssetProof, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("open asset archive {}: {error}", path.display()))?;
    let bytes = file
        .metadata()
        .map_err(|error| format!("stat asset archive {}: {error}", path.display()))?
        .len();
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read asset archive {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let sha256 = hex::encode(hasher.finalize());
    Ok(AssetProof {
        path: path.to_path_buf(),
        matches_expected: sha256.eq_ignore_ascii_case(expected),
        expected_sha256: expected.to_owned(),
        sha256,
        bytes,
    })
}

pub fn require_asset_match(proof: &AssetProof) -> Result<(), String> {
    if proof.matches_expected {
        Ok(())
    } else {
        Err(format!(
            "asset archive digest mismatch for {}: expected {}, got {}",
            proof.path.display(),
            proof.expected_sha256,
            proof.sha256
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_is_independent_from_density() {
        let mut config = BenchConfig {
            family: Family::Hd,
            density: 32,
            ..BenchConfig::default()
        };
        assert_eq!(config.family(), Family::Hd);
        config.density = 128;
        assert_eq!(config.family(), Family::Hd);
    }
}
