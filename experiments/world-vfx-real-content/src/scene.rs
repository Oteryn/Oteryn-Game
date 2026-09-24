use crate::prepared_cache::{PreparedCache, SpriteLocator};
use crate::presentation::PreparedPrograms;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const SCENE_SCHEMA: &str = "oteryn-world-vfx-real-content-thais-scene-v1";
const THAIS_FIXTURE_ARTIFACT: &str =
    "sha256:4b340053f72b3522a9fe644c9afdf08d9c7b9b686aec0b57cef764a7cb7dc468";
const MAX_SCENE_BYTES: u64 = 64 * 1024 * 1024;
const EXPECTED_TILE_RECORDS: usize = 1_200;
const EXPECTED_PRIMITIVES: usize = 2_215;
const DENSE_FLOOR: i32 = -7;
const DENSE_X_MIN: i32 = 32_400;
const DENSE_X_MAX: i32 = 32_439;
const DENSE_Y_MIN: i32 = 32_239;
const DENSE_Y_MAX: i32 = 32_268;
const DENSE_WIDTH: i32 = 40;
const DENSE_HEIGHT: i32 = 30;

#[derive(Debug, Deserialize)]
struct SceneDocument {
    schema: String,
    fixture_artifact: String,
    viewport: SceneViewport,
    counts: SceneCounts,
    sprite_source_ids: Vec<u32>,
    primitives: Vec<ScenePrimitiveRecord>,
    proprietary_pixels_embedded: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
struct SceneViewport {
    floor: i32,
    x_min: i32,
    x_max_inclusive: i32,
    y_min: i32,
    y_max_inclusive: i32,
    width_tiles: i32,
    height_tiles: i32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
struct SceneCounts {
    tile_records: usize,
    primitives: usize,
    unique_sprite_source_ids: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneTile {
    pub world_x: i32,
    pub world_y: i32,
    pub scene_x: i32,
    pub scene_y: i32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresentationOrder {
    pub order: i64,
    pub plane: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct SceneDisplacement {
    pub dx_units: i32,
    pub dy_units: i32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct CoverageOffset {
    pub dx_tiles: i32,
    pub dy_tiles: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ScenePrimitiveRecord {
    tile: SceneTile,
    appearance_source_id: u32,
    source_role: String,
    presentation_order: PresentationOrder,
    primitive_index: usize,
    sprite_source_id: u32,
    source_geometry: [u32; 2],
    displacement: SceneDisplacement,
    visual_coverage_offsets: Vec<CoverageOffset>,
    layer_index: usize,
    phase: usize,
    frame_group_id: u64,
    frame_group_type: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneDrawPrimitive {
    pub tile: SceneTile,
    pub appearance_source_id: u32,
    pub source_role: String,
    pub presentation_order: PresentationOrder,
    pub primitive_index: usize,
    pub sprite_source_id: u32,
    pub locator: SpriteLocator,
    pub displacement: SceneDisplacement,
    pub visual_coverage_offsets: Vec<CoverageOffset>,
    pub layer_index: usize,
    pub phase: usize,
    pub frame_group_id: u64,
    pub frame_group_type: i64,
}

#[derive(Debug, Clone)]
pub struct DenseScene {
    primitives: Vec<SceneDrawPrimitive>,
    unique_sprite_source_ids: Vec<u32>,
    source_sha256: String,
}

impl DenseScene {
    pub fn open(scene_path: &Path, cache: &PreparedCache) -> Result<Self, String> {
        let metadata = fs::metadata(scene_path)
            .map_err(|error| format!("stat {}: {error}", scene_path.display()))?;
        if metadata.len() > MAX_SCENE_BYTES {
            return Err(format!(
                "dense scene exceeds {MAX_SCENE_BYTES} bytes: {}",
                metadata.len()
            ));
        }
        let bytes = fs::read(scene_path)
            .map_err(|error| format!("read {}: {error}", scene_path.display()))?;
        let source_sha256 = sha256_bytes(&bytes);
        if cache.atlas_slice_sha256() != source_sha256 {
            return Err(format!(
                "dense scene/cache SHA-256 mismatch: cache expects {}, got {source_sha256}",
                cache.atlas_slice_sha256()
            ));
        }
        let document: SceneDocument = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse {}: {error}", scene_path.display()))?;
        validate_document(&document)?;

        let mut primitives = Vec::with_capacity(document.primitives.len());
        for record in document.primitives {
            let locator = cache.locator(record.sprite_source_id).ok_or_else(|| {
                format!(
                    "dense scene sprite {} is absent from prepared cache",
                    record.sprite_source_id
                )
            })?;
            if [locator.source_width, locator.source_height] != record.source_geometry {
                return Err(format!(
                    "dense scene sprite {} geometry mismatch: scene {:?}, cache {}x{}",
                    record.sprite_source_id,
                    record.source_geometry,
                    locator.source_width,
                    locator.source_height
                ));
            }
            primitives.push(SceneDrawPrimitive {
                tile: record.tile,
                appearance_source_id: record.appearance_source_id,
                source_role: record.source_role,
                presentation_order: record.presentation_order,
                primitive_index: record.primitive_index,
                sprite_source_id: record.sprite_source_id,
                locator,
                displacement: record.displacement,
                visual_coverage_offsets: record.visual_coverage_offsets,
                layer_index: record.layer_index,
                phase: record.phase,
                frame_group_id: record.frame_group_id,
                frame_group_type: record.frame_group_type,
            });
        }

        Ok(Self {
            primitives,
            unique_sprite_source_ids: document.sprite_source_ids,
            source_sha256,
        })
    }

    pub fn primitives(&self) -> &[SceneDrawPrimitive] {
        &self.primitives
    }

    pub fn unique_sprite_source_ids(&self) -> &[u32] {
        &self.unique_sprite_source_ids
    }

    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
}

#[derive(Debug)]
pub struct QualificationBundle {
    cache: PreparedCache,
    programs: PreparedPrograms,
    scene: DenseScene,
}

impl QualificationBundle {
    pub fn open(
        manifest_path: &Path,
        scene_path: &Path,
        max_resident_pages: usize,
    ) -> Result<Self, String> {
        let cache = PreparedCache::open(manifest_path, max_resident_pages)?;
        let programs = PreparedPrograms::open(manifest_path)?;
        if cache.semantic_identity().2 != programs.product_root() {
            return Err(format!(
                "prepared cache/program product-root mismatch: cache {}, programs {}",
                cache.semantic_identity().2,
                programs.product_root()
            ));
        }
        let scene = DenseScene::open(scene_path, &cache)?;
        Ok(Self {
            cache,
            programs,
            scene,
        })
    }

    pub fn cache(&self) -> &PreparedCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut PreparedCache {
        &mut self.cache
    }

    pub fn programs(&self) -> &PreparedPrograms {
        &self.programs
    }

    pub fn scene(&self) -> &DenseScene {
        &self.scene
    }
}

fn validate_document(document: &SceneDocument) -> Result<(), String> {
    if document.schema != SCENE_SCHEMA {
        return Err(format!(
            "dense scene schema mismatch: expected {SCENE_SCHEMA}, got {}",
            document.schema
        ));
    }
    if document.fixture_artifact != THAIS_FIXTURE_ARTIFACT {
        return Err(format!(
            "dense scene fixture artifact mismatch: expected {THAIS_FIXTURE_ARTIFACT}, got {}",
            document.fixture_artifact
        ));
    }
    let expected_viewport = SceneViewport {
        floor: DENSE_FLOOR,
        x_min: DENSE_X_MIN,
        x_max_inclusive: DENSE_X_MAX,
        y_min: DENSE_Y_MIN,
        y_max_inclusive: DENSE_Y_MAX,
        width_tiles: DENSE_WIDTH,
        height_tiles: DENSE_HEIGHT,
    };
    if document.viewport != expected_viewport {
        return Err(format!(
            "dense scene viewport mismatch: expected {expected_viewport:?}, got {:?}",
            document.viewport
        ));
    }
    if document.counts.tile_records != EXPECTED_TILE_RECORDS
        || document.counts.primitives != EXPECTED_PRIMITIVES
        || document.primitives.len() != EXPECTED_PRIMITIVES
    {
        return Err(format!(
            "dense scene count mismatch: expected {EXPECTED_TILE_RECORDS} tiles/{EXPECTED_PRIMITIVES} primitives, got {}/{}",
            document.counts.tile_records,
            document.primitives.len()
        ));
    }
    if document.proprietary_pixels_embedded {
        return Err("dense scene claims embedded proprietary pixels".to_owned());
    }
    if document.sprite_source_ids.is_empty()
        || !document
            .sprite_source_ids
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        || document.sprite_source_ids.contains(&0)
    {
        return Err(
            "dense scene sprite_source_ids must be non-zero and strictly increasing".to_owned(),
        );
    }
    if document.counts.unique_sprite_source_ids != document.sprite_source_ids.len() {
        return Err(format!(
            "dense scene unique-sprite count mismatch: summary {}, list {}",
            document.counts.unique_sprite_source_ids,
            document.sprite_source_ids.len()
        ));
    }

    let declared_sprites: BTreeSet<u32> = document.sprite_source_ids.iter().copied().collect();
    let mut actual_sprites = BTreeSet::new();
    let mut tiles = BTreeSet::new();
    let mut previous_key: Option<(i32, i32, i64, i64, usize, usize)> = None;

    for primitive in &document.primitives {
        validate_primitive(primitive)?;
        actual_sprites.insert(primitive.sprite_source_id);
        tiles.insert((primitive.tile.scene_x, primitive.tile.scene_y));
        let key = primitive_sort_key(primitive);
        if previous_key.is_some_and(|previous| previous > key) {
            return Err(
                "dense scene primitives are not in deterministic presentation order".to_owned(),
            );
        }
        previous_key = Some(key);
    }

    if actual_sprites != declared_sprites {
        return Err("dense scene primitive sprite set does not match sprite_source_ids".to_owned());
    }
    if tiles.len() != EXPECTED_TILE_RECORDS {
        return Err(format!(
            "dense scene covers {} tile coordinates, expected {EXPECTED_TILE_RECORDS}",
            tiles.len()
        ));
    }
    Ok(())
}

fn validate_primitive(primitive: &ScenePrimitiveRecord) -> Result<(), String> {
    let tile = primitive.tile;
    if tile.scene_x < 0
        || tile.scene_x >= DENSE_WIDTH
        || tile.scene_y < 0
        || tile.scene_y >= DENSE_HEIGHT
        || tile.world_x != DENSE_X_MIN + tile.scene_x
        || tile.world_y != DENSE_Y_MIN + tile.scene_y
    {
        return Err(format!("dense scene tile coordinate mismatch: {tile:?}"));
    }
    if primitive.appearance_source_id == 0 || primitive.sprite_source_id == 0 {
        return Err(format!(
            "dense scene tile {},{} has zero semantic/source sprite id",
            tile.scene_x, tile.scene_y
        ));
    }
    if !matches!(primitive.source_role.as_str(), "ground" | "tile_item") {
        return Err(format!(
            "dense scene tile {},{} has unsupported source role {}",
            tile.scene_x, tile.scene_y, primitive.source_role
        ));
    }
    if !matches!(
        primitive.source_geometry,
        [32, 32] | [32, 64] | [64, 32] | [64, 64]
    ) {
        return Err(format!(
            "dense scene sprite {} has unsupported source geometry {:?}",
            primitive.sprite_source_id, primitive.source_geometry
        ));
    }
    if primitive.visual_coverage_offsets.is_empty() {
        return Err(format!(
            "dense scene sprite {} has empty visual coverage",
            primitive.sprite_source_id
        ));
    }
    Ok(())
}

fn primitive_sort_key(primitive: &ScenePrimitiveRecord) -> (i32, i32, i64, i64, usize, usize) {
    (
        primitive.tile.scene_y,
        primitive.tile.scene_x,
        primitive.presentation_order.plane,
        primitive.presentation_order.order,
        primitive.layer_index,
        primitive.primitive_index,
    )
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex::encode(digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primitive(scene_x: i32, scene_y: i32, plane: i64, order: i64) -> ScenePrimitiveRecord {
        ScenePrimitiveRecord {
            tile: SceneTile {
                world_x: DENSE_X_MIN + scene_x,
                world_y: DENSE_Y_MIN + scene_y,
                scene_x,
                scene_y,
            },
            appearance_source_id: 10,
            source_role: "ground".to_owned(),
            presentation_order: PresentationOrder { order, plane },
            primitive_index: 0,
            sprite_source_id: 20,
            source_geometry: [32, 32],
            displacement: SceneDisplacement {
                dx_units: 0,
                dy_units: 0,
            },
            visual_coverage_offsets: vec![CoverageOffset {
                dx_tiles: 0,
                dy_tiles: 0,
            }],
            layer_index: 0,
            phase: 0,
            frame_group_id: 0,
            frame_group_type: 2,
        }
    }

    #[test]
    fn presentation_sort_key_matches_projection_contract() {
        let first = primitive(4, 3, 1, 8);
        let second = primitive(5, 3, 0, 0);
        let third = primitive(5, 3, 0, 1);
        assert!(primitive_sort_key(&first) < primitive_sort_key(&second));
        assert!(primitive_sort_key(&second) < primitive_sort_key(&third));
    }

    #[test]
    fn primitive_validation_rejects_scene_world_mismatch() {
        let mut value = primitive(0, 0, 0, 0);
        value.tile.world_x += 1;
        assert!(validate_primitive(&value).is_err());
    }
}
