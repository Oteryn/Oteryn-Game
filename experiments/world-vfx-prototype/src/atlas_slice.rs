use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

const SCHEMA: &str = "oteryn-world-vfx-atlas-slice-v1";
const PUBLICATION_ROOT: &str =
    "sha256:9d0d2f3bb16a5a90f9b51a21366e4ed42963f5cb12366c404a20d9502ec4857f";
const SEMANTIC_ROOT: &str =
    "sha256:27d7a83a7d9f498ea614b440ab4216cae5e6d11ea0527482410e40948cade5a9";
const PIXEL_ROOT: &str = "sha256:8b8228fcc4574903e547cb7d65b96f3d45e5a9e67045091c1bceb6e54d3690ad";
const RUNTIME_ROOT: &str =
    "sha256:fa30ae5fc47f0ca8a6d482ed87b5db2cd74f32f7f523df16187ca719b8e04f08";
const CREATURE_DIGEST: &str =
    "sha256:7dc951874c95424279737eaaf51cf2d50940162ef4799daea39a187a581ef0e8";

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasSlice {
    pub schema: String,
    pub source: AtlasSource,
    pub bounds: AtlasBounds,
    pub tiles: Vec<AtlasTile>,
    pub creatures: Vec<AtlasCreature>,
    #[serde(skip)]
    pub slice_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlasSource {
    pub publication_root: String,
    pub semantic_root: String,
    pub pixel_root: String,
    pub runtime_index_root: String,
    pub source_fingerprint: String,
    pub game_sha: String,
    pub creature_semantic_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AtlasBounds {
    pub x_min: i32,
    pub x_max_exclusive: i32,
    pub y_min: i32,
    pub y_max_exclusive: i32,
    pub floors: Vec<i16>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasTile {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub presentations: Vec<AtlasPresentation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasPresentation {
    pub order: u16,
    pub role: String,
    pub appearance_source_id: u32,
    pub primitives: Vec<AtlasPrimitive>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasPrimitive {
    pub sprite_id: u32,
    pub width_units: u32,
    pub height_units: u32,
    pub dx_units: i32,
    pub dy_units: i32,
    pub layer_index: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtlasCreature {
    pub record_id: String,
    pub kind: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
    pub look_type: u32,
    pub presentation_resolved: bool,
    pub dx_units: i32,
    pub dy_units: i32,
    pub phase_count: u32,
    pub phase_durations_ms: Vec<u32>,
    pub synchronized: bool,
    pub loop_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AtlasSliceEvidence {
    pub schema: &'static str,
    pub source: AtlasSource,
    pub bounds: AtlasBounds,
    pub slice_sha256: String,
    pub tiles: usize,
    pub resolved_primitives: usize,
    pub creatures: usize,
    pub resolved_creatures: usize,
    pub unique_sprite_ids: usize,
    pub pixels_committed: bool,
}

impl AtlasSlice {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("read Atlas slice {}: {error}", path.display()))?;
        let slice_sha256 = hex::encode(Sha256::digest(&bytes));
        let mut slice: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse Atlas slice {}: {error}", path.display()))?;
        slice.slice_sha256 = slice_sha256;
        slice.validate()?;
        Ok(slice)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("unsupported Atlas slice schema: {}", self.schema));
        }
        for (label, actual, expected) in [
            (
                "publication",
                self.source.publication_root.as_str(),
                PUBLICATION_ROOT,
            ),
            (
                "semantic",
                self.source.semantic_root.as_str(),
                SEMANTIC_ROOT,
            ),
            ("pixel", self.source.pixel_root.as_str(), PIXEL_ROOT),
            (
                "runtime",
                self.source.runtime_index_root.as_str(),
                RUNTIME_ROOT,
            ),
            (
                "creatures",
                self.source.creature_semantic_digest.as_str(),
                CREATURE_DIGEST,
            ),
        ] {
            if actual != expected {
                return Err(format!("Atlas slice {label} identity mismatch: {actual}"));
            }
        }
        if self.bounds.x_min >= self.bounds.x_max_exclusive
            || self.bounds.y_min >= self.bounds.y_max_exclusive
            || self.bounds.floors.is_empty()
        {
            return Err("Atlas slice bounds are invalid".to_owned());
        }
        for tile in &self.tiles {
            if !self.contains(tile.x, tile.y, tile.floor) {
                return Err("Atlas tile outside declared slice bounds".to_owned());
            }
            for (index, presentation) in tile.presentations.iter().enumerate() {
                if usize::from(presentation.order) != index {
                    return Err("Atlas presentation order is not contiguous".to_owned());
                }
                if presentation.role != "ground" && presentation.role != "tile_item" {
                    return Err(format!(
                        "unsupported Atlas source role: {}",
                        presentation.role
                    ));
                }
                if presentation.appearance_source_id == 0 {
                    return Err("Atlas appearance source id must be positive".to_owned());
                }
                for primitive in &presentation.primitives {
                    if primitive.sprite_id == 0
                        || !matches!(
                            (primitive.width_units, primitive.height_units),
                            (32, 32) | (32, 64) | (64, 32) | (64, 64)
                        )
                    {
                        return Err("Atlas primitive geometry/source id invalid".to_owned());
                    }
                }
            }
        }
        for creature in &self.creatures {
            if !self.contains(creature.x, creature.y, creature.floor) {
                return Err("Atlas creature outside declared slice bounds".to_owned());
            }
            if creature.record_id.is_empty()
                || creature.name.is_empty()
                || (creature.kind != "monster" && creature.kind != "npc")
            {
                return Err("Atlas creature identity invalid".to_owned());
            }
        }
        Ok(())
    }

    pub fn contains(&self, x: i32, y: i32, floor: i16) -> bool {
        self.bounds.x_min <= x
            && x < self.bounds.x_max_exclusive
            && self.bounds.y_min <= y
            && y < self.bounds.y_max_exclusive
            && self.bounds.floors.contains(&floor)
    }

    pub fn center(&self) -> (f32, f32, i16) {
        let x = (self.bounds.x_min + self.bounds.x_max_exclusive - 1) as f32 * 0.5;
        let y = (self.bounds.y_min + self.bounds.y_max_exclusive - 1) as f32 * 0.5;
        let floor = if self.bounds.floors.contains(&-7) {
            -7
        } else {
            self.bounds.floors[0]
        };
        (x, y, floor)
    }

    pub fn evidence(&self) -> AtlasSliceEvidence {
        let mut sprites = BTreeSet::new();
        let mut resolved_primitives = 0_usize;
        for tile in &self.tiles {
            for presentation in &tile.presentations {
                resolved_primitives += presentation.primitives.len();
                sprites.extend(
                    presentation
                        .primitives
                        .iter()
                        .map(|primitive| primitive.sprite_id),
                );
            }
        }
        AtlasSliceEvidence {
            schema: "oteryn-world-vfx-atlas-slice-evidence-v1",
            source: self.source.clone(),
            bounds: self.bounds.clone(),
            slice_sha256: self.slice_sha256.clone(),
            tiles: self.tiles.len(),
            resolved_primitives,
            creatures: self.creatures.len(),
            resolved_creatures: self
                .creatures
                .iter()
                .filter(|value| value.presentation_resolved)
                .count(),
            unique_sprite_ids: sprites.len(),
            pixels_committed: false,
        }
    }
}
