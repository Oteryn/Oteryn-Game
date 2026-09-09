use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const PREPARED_CACHE_SCHEMA: &str = "oteryn-world-vfx-real-content-cache-v1";
pub const SOURCE_ZIP_SHA256: &str =
    "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f";
pub const SOURCE_CATALOG_SHA256: &str =
    "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85";
pub const SOURCE_APPEARANCE_SHA256: &str =
    "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075";
pub const SEMANTIC_AUTHORITY_PATH: &str = "tools/game-atlas-appearances/export.py";
pub const SEMANTIC_CONTRACT_ID: &str = "oteryn-game-atlas-animated-appearances-v1";
pub const SEMANTIC_REVISION: &str = "1";
pub const SOURCE_PROFILE_ID: &str = "oteryn-atlas-15-32-appearance-spatial-v1";
pub const SPRITES_PER_PAGE: usize = 64;
pub const SLOT_SIZE: usize = 64;
pub const PAGE_BYTES: usize = SPRITES_PER_PAGE * SLOT_SIZE * SLOT_SIZE * 4;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_REQUIRED_SPRITES: usize = 8_192;
const MAX_PAGES: usize = 8_192;
const MAX_RESIDENT_PAGES: usize = 256;

#[derive(Debug, Deserialize)]
struct Manifest {
    schema: String,
    source: ManifestSource,
    semantic_authority: SemanticAuthority,
    atlas_slice_sha256: String,
    sprite_page: SpritePageSummary,
    bindings: Bindings,
    required_sprite_ids: Vec<u32>,
    pages: Vec<ManifestPage>,
    proprietary_pixels_committed: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestSource {
    label: String,
    zip_sha256: String,
    catalog_sha256: String,
    appearance_sha256: String,
}

#[derive(Debug, Deserialize)]
struct SemanticAuthority {
    path: String,
    contract_id: String,
    #[serde(deserialize_with = "deserialize_semantic_revision")]
    semantic_revision: String,
    product_root: String,
}

#[derive(Debug, Deserialize)]
struct SpritePageSummary {
    sprites_per_page: usize,
    slot_size: usize,
    page_count: usize,
    required_sprite_count: usize,
    decoded_sheet_count: usize,
}

#[derive(Debug, Deserialize)]
struct Bindings {
    outfit: serde_json::Value,
    effect: serde_json::Value,
    missile: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ManifestPage {
    page_id: u32,
    path: String,
    sha256: String,
    byte_length: usize,
    members: Vec<ManifestMember>,
}

#[derive(Debug, Deserialize)]
struct ManifestMember {
    sprite_source_id: u32,
    source_geometry: [u32; 2],
    slot: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteLocator {
    pub sprite_source_id: u32,
    pub page_id: u32,
    pub layer: u32,
    pub source_width: u32,
    pub source_height: u32,
}

#[derive(Debug, Clone)]
pub struct PreparedPage {
    pub page_id: u32,
    pub rgba: Arc<[u8]>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedCacheStats {
    pub page_requests: u64,
    pub page_hits: u64,
    pub page_loads: u64,
    pub page_evictions: u64,
    pub bytes_read: u64,
}

#[derive(Debug)]
struct PageMeta {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct ResidentPage {
    page_id: u32,
    last_used: u64,
    rgba: Arc<[u8]>,
}

#[derive(Debug)]
pub struct PreparedCache {
    pages: BTreeMap<u32, PageMeta>,
    sprites: BTreeMap<u32, SpriteLocator>,
    max_resident_pages: usize,
    resident: Vec<ResidentPage>,
    tick: u64,
    stats: PreparedCacheStats,
    contract_id: String,
    semantic_revision: String,
    product_root: String,
    atlas_slice_sha256: String,
}

impl PreparedCache {
    pub fn open(manifest_path: &Path, max_resident_pages: usize) -> Result<Self, String> {
        if max_resident_pages == 0 || max_resident_pages > MAX_RESIDENT_PAGES {
            return Err(format!(
                "prepared page cache capacity must be in 1..={MAX_RESIDENT_PAGES}, got {max_resident_pages}"
            ));
        }
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
        let manifest: Manifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse {}: {error}", manifest_path.display()))?;
        validate_manifest_identity(&manifest)?;

        let manifest_dir = manifest_path
            .parent()
            .ok_or_else(|| "prepared manifest has no parent directory".to_owned())?
            .canonicalize()
            .map_err(|error| format!("canonicalize prepared cache root: {error}"))?;

        let mut pages = BTreeMap::new();
        let mut sprites = BTreeMap::new();
        let required: BTreeSet<u32> = manifest.required_sprite_ids.iter().copied().collect();
        if required.len() != manifest.required_sprite_ids.len() {
            return Err("prepared manifest contains duplicate required sprite IDs".to_owned());
        }
        if required.len() > MAX_REQUIRED_SPRITES {
            return Err(format!(
                "prepared manifest requires {} sprites, cap is {MAX_REQUIRED_SPRITES}",
                required.len()
            ));
        }
        if required.iter().any(|sprite_id| *sprite_id == 0) {
            return Err("prepared manifest contains sprite_source_id 0".to_owned());
        }
        if !manifest
            .required_sprite_ids
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        {
            return Err("prepared required_sprite_ids must be strictly increasing".to_owned());
        }
        if manifest.pages.len() > MAX_PAGES {
            return Err(format!(
                "prepared manifest contains {} pages, cap is {MAX_PAGES}",
                manifest.pages.len()
            ));
        }

        for page in manifest.pages {
            if page.byte_length != PAGE_BYTES {
                return Err(format!(
                    "page {} byte_length must be {PAGE_BYTES}, got {}",
                    page.page_id, page.byte_length
                ));
            }
            validate_lower_sha256(&page.sha256, "page sha256")?;
            let expected_path = format!("pages/page-{:06}.rgba", page.page_id);
            if page.path != expected_path {
                return Err(format!(
                    "page {} path mismatch: expected {expected_path:?}, got {:?}",
                    page.page_id, page.path
                ));
            }
            let canonical = manifest_dir
                .join(Path::new(&page.path))
                .canonicalize()
                .map_err(|error| format!("canonicalize page {}: {error}", page.page_id))?;
            if !canonical.starts_with(&manifest_dir) {
                return Err(format!("page {} escapes prepared cache root", page.page_id));
            }
            let file_len = fs::metadata(&canonical)
                .map_err(|error| format!("stat page {}: {error}", page.page_id))?
                .len();
            if file_len != PAGE_BYTES as u64 {
                return Err(format!(
                    "page {} physical length must be {PAGE_BYTES}, got {file_len}",
                    page.page_id
                ));
            }
            if pages
                .insert(
                    page.page_id,
                    PageMeta {
                        path: canonical,
                        sha256: page.sha256,
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate page_id {}", page.page_id));
            }

            let mut slots = BTreeSet::new();
            for member in page.members {
                if member.slot >= SPRITES_PER_PAGE {
                    return Err(format!(
                        "sprite {} has invalid page slot {}",
                        member.sprite_source_id, member.slot
                    ));
                }
                if !slots.insert(member.slot) {
                    return Err(format!(
                        "page {} contains duplicate slot {}",
                        page.page_id, member.slot
                    ));
                }
                if member.sprite_source_id / SPRITES_PER_PAGE as u32 != page.page_id
                    || member.sprite_source_id % SPRITES_PER_PAGE as u32 != member.slot as u32
                {
                    return Err(format!(
                        "sprite {} does not map to page {} slot {}",
                        member.sprite_source_id, page.page_id, member.slot
                    ));
                }
                let [width, height] = member.source_geometry;
                if !matches!((width, height), (32, 32) | (32, 64) | (64, 32) | (64, 64)) {
                    return Err(format!(
                        "sprite {} has unsupported source geometry {width}x{height}",
                        member.sprite_source_id
                    ));
                }
                let locator = SpriteLocator {
                    sprite_source_id: member.sprite_source_id,
                    page_id: page.page_id,
                    layer: member.slot as u32,
                    source_width: width,
                    source_height: height,
                };
                if sprites.insert(member.sprite_source_id, locator).is_some() {
                    return Err(format!(
                        "duplicate sprite_source_id {} in page members",
                        member.sprite_source_id
                    ));
                }
            }
        }

        let actual: BTreeSet<u32> = sprites.keys().copied().collect();
        if actual != required {
            return Err(
                "prepared page members do not exactly match required_sprite_ids".to_owned(),
            );
        }
        if manifest.sprite_page.page_count != pages.len()
            || manifest.sprite_page.required_sprite_count != sprites.len()
        {
            return Err(
                "prepared manifest count summary does not match page/member content".to_owned(),
            );
        }
        if manifest.sprite_page.decoded_sheet_count == 0 && !sprites.is_empty() {
            return Err(
                "prepared manifest reports zero decoded sheets for non-empty content".to_owned(),
            );
        }

        Ok(Self {
            pages,
            sprites,
            max_resident_pages,
            resident: Vec::with_capacity(max_resident_pages),
            tick: 0,
            stats: PreparedCacheStats::default(),
            contract_id: manifest.semantic_authority.contract_id,
            semantic_revision: manifest.semantic_authority.semantic_revision,
            product_root: manifest.semantic_authority.product_root,
            atlas_slice_sha256: manifest.atlas_slice_sha256,
        })
    }

    pub fn locator(&self, sprite_source_id: u32) -> Option<SpriteLocator> {
        self.sprites.get(&sprite_source_id).copied()
    }

    pub fn page(&mut self, page_id: u32) -> Result<PreparedPage, String> {
        self.stats.page_requests = self.stats.page_requests.saturating_add(1);
        self.tick = self.tick.saturating_add(1);
        if let Some(index) = self
            .resident
            .iter()
            .position(|resident| resident.page_id == page_id)
        {
            self.stats.page_hits = self.stats.page_hits.saturating_add(1);
            self.resident[index].last_used = self.tick;
            return Ok(PreparedPage {
                page_id,
                rgba: Arc::clone(&self.resident[index].rgba),
            });
        }

        let meta = self
            .pages
            .get(&page_id)
            .ok_or_else(|| format!("prepared page {page_id} is not in the manifest"))?;
        let bytes = fs::read(&meta.path)
            .map_err(|error| format!("read prepared page {page_id}: {error}"))?;
        if bytes.len() != PAGE_BYTES {
            return Err(format!(
                "prepared page {page_id} length changed: expected {PAGE_BYTES}, got {}",
                bytes.len()
            ));
        }
        let digest = sha256_bytes(&bytes);
        if digest != meta.sha256 {
            return Err(format!(
                "prepared page {page_id} SHA-256 mismatch: expected {}, got {digest}",
                meta.sha256
            ));
        }
        let rgba: Arc<[u8]> = Arc::from(bytes);
        self.stats.page_loads = self.stats.page_loads.saturating_add(1);
        self.stats.bytes_read = self.stats.bytes_read.saturating_add(PAGE_BYTES as u64);

        let resident = ResidentPage {
            page_id,
            last_used: self.tick,
            rgba: Arc::clone(&rgba),
        };
        if self.resident.len() < self.max_resident_pages {
            self.resident.push(resident);
        } else {
            let index = self
                .resident
                .iter()
                .enumerate()
                .min_by_key(|(_, candidate)| candidate.last_used)
                .map(|(index, _)| index)
                .ok_or_else(|| "non-empty prepared cache has no eviction candidate".to_owned())?;
            self.resident[index] = resident;
            self.stats.page_evictions = self.stats.page_evictions.saturating_add(1);
        }
        Ok(PreparedPage { page_id, rgba })
    }

    pub fn page_for_sprite(
        &mut self,
        sprite_source_id: u32,
    ) -> Result<(SpriteLocator, PreparedPage), String> {
        let locator = self.locator(sprite_source_id).ok_or_else(|| {
            format!("sprite_source_id {sprite_source_id} is absent from prepared cache")
        })?;
        let page = self.page(locator.page_id)?;
        Ok((locator, page))
    }

    pub fn stats(&self) -> &PreparedCacheStats {
        &self.stats
    }

    pub fn resident_pages(&self) -> usize {
        self.resident.len()
    }

    pub fn semantic_identity(&self) -> (&str, &str, &str) {
        (
            &self.contract_id,
            &self.semantic_revision,
            &self.product_root,
        )
    }

    pub fn atlas_slice_sha256(&self) -> &str {
        &self.atlas_slice_sha256
    }
}

fn deserialize_semantic_revision<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(number) if number.is_u64() => Ok(number.to_string()),
        serde_json::Value::String(text) => Ok(text),
        other => Err(serde::de::Error::custom(format!(
            "semantic_revision must be an integer or decimal string, got {other}"
        ))),
    }
}

fn validate_manifest_identity(manifest: &Manifest) -> Result<(), String> {
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
            "prepared cache source identity is not the pinned Tibia 15.32 source".to_owned(),
        );
    }
    if manifest.semantic_authority.path != SEMANTIC_AUTHORITY_PATH {
        return Err(format!(
            "prepared semantic authority must be {SEMANTIC_AUTHORITY_PATH}, got {}",
            manifest.semantic_authority.path
        ));
    }
    if manifest.semantic_authority.contract_id != SEMANTIC_CONTRACT_ID
        || manifest.semantic_authority.semantic_revision != SEMANTIC_REVISION
    {
        return Err(format!(
            "prepared semantic authority mismatch: expected {SEMANTIC_CONTRACT_ID} revision {SEMANTIC_REVISION}, got {} revision {}",
            manifest.semantic_authority.contract_id, manifest.semantic_authority.semantic_revision
        ));
    }
    let product_digest = manifest
        .semantic_authority
        .product_root
        .strip_prefix("sha256:")
        .ok_or_else(|| "prepared semantic authority product_root must use sha256: prefix".to_owned())?;
    validate_lower_sha256(product_digest, "semantic product_root")?;
    validate_lower_sha256(&manifest.atlas_slice_sha256, "atlas_slice_sha256")?;
    if manifest.sprite_page.sprites_per_page != SPRITES_PER_PAGE
        || manifest.sprite_page.slot_size != SLOT_SIZE
    {
        return Err(format!(
            "prepared page geometry must be {SPRITES_PER_PAGE}x{SLOT_SIZE}, got {}x{}",
            manifest.sprite_page.sprites_per_page, manifest.sprite_page.slot_size
        ));
    }
    if manifest.proprietary_pixels_committed {
        return Err("prepared manifest claims proprietary pixels were committed".to_owned());
    }
    for (label, binding) in [
        ("outfit", &manifest.bindings.outfit),
        ("effect", &manifest.bindings.effect),
        ("missile", &manifest.bindings.missile),
    ] {
        if !binding.is_object() {
            return Err(format!(
                "prepared {label} binding must be a normalized object"
            ));
        }
    }
    Ok(())
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

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex::encode(digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn create() -> Result<Self, String> {
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "oteryn-prepared-cache-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(path.join("pages"))
                .map_err(|error| format!("create temp cache: {error}"))?;
            Ok(Self(path))
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            drop(fs::remove_dir_all(&self.0));
        }
    }

    fn write_fixture(root: &Path, page_ids: &[u32]) -> Result<PathBuf, String> {
        let mut pages = Vec::new();
        let mut required = Vec::new();
        for page_id in page_ids {
            let sprite_id = page_id * SPRITES_PER_PAGE as u32 + 1;
            let mut bytes = vec![0_u8; PAGE_BYTES];
            bytes[0] = (*page_id & 0xff) as u8;
            let digest = sha256_bytes(&bytes);
            let relative = format!("pages/page-{page_id:06}.rgba");
            fs::write(root.join(&relative), &bytes)
                .map_err(|error| format!("write temp page: {error}"))?;
            pages.push(json!({
                "page_id": page_id,
                "path": relative,
                "sha256": digest,
                "byte_length": PAGE_BYTES,
                "members": [{
                    "sprite_source_id": sprite_id,
                    "source_geometry": [32, 32],
                    "slot": 1
                }]
            }));
            required.push(sprite_id);
        }
        required.sort_unstable();
        let manifest = json!({
            "schema": PREPARED_CACHE_SCHEMA,
            "source": {
                "label": "15.32",
                "zip_sha256": SOURCE_ZIP_SHA256,
                "catalog_sha256": SOURCE_CATALOG_SHA256,
                "appearance_sha256": SOURCE_APPEARANCE_SHA256
            },
            "semantic_authority": {
                "path": SEMANTIC_AUTHORITY_PATH,
                "contract_id": SEMANTIC_CONTRACT_ID,
                "semantic_revision": 1,
                "product_root": format!("sha256:{}", "0".repeat(64))
            },
            "atlas_slice_sha256": "0".repeat(64),
            "sprite_page": {
                "sprites_per_page": SPRITES_PER_PAGE,
                "slot_size": SLOT_SIZE,
                "page_count": page_ids.len(),
                "required_sprite_count": required.len(),
                "decoded_sheet_count": page_ids.len()
            },
            "bindings": {"outfit": {}, "effect": {}, "missile": {}},
            "required_sprite_ids": required,
            "pages": pages,
            "proprietary_pixels_committed": false
        });
        let path = root.join("real-content-manifest.json");
        let encoded = serde_json::to_vec(&manifest)
            .map_err(|error| format!("encode temp manifest: {error}"))?;
        fs::write(&path, encoded).map_err(|error| format!("write temp manifest: {error}"))?;
        Ok(path)
    }

    #[test]
    fn loads_verified_page_and_locator() -> Result<(), String> {
        let root = TempRoot::create()?;
        let manifest = write_fixture(&root.0, &[3])?;
        let mut cache = PreparedCache::open(&manifest, 1)?;
        let sprite_id = 3 * SPRITES_PER_PAGE as u32 + 1;
        let (locator, page) = cache.page_for_sprite(sprite_id)?;
        assert_eq!(locator.page_id, 3);
        assert_eq!(locator.layer, 1);
        assert_eq!(page.rgba.len(), PAGE_BYTES);
        assert_eq!(cache.stats().page_loads, 1);
        assert_eq!(
            cache.semantic_identity().0,
            SEMANTIC_CONTRACT_ID
        );
        assert_eq!(cache.semantic_identity().1, SEMANTIC_REVISION);
        let second = cache.page(3)?;
        assert_eq!(second.rgba.len(), PAGE_BYTES);
        assert_eq!(cache.stats().page_hits, 1);
        Ok(())
    }

    #[test]
    fn bounded_cache_evicts_least_recent_page() -> Result<(), String> {
        let root = TempRoot::create()?;
        let manifest = write_fixture(&root.0, &[1, 2])?;
        let mut cache = PreparedCache::open(&manifest, 1)?;
        drop(cache.page(1)?);
        drop(cache.page(2)?);
        assert_eq!(cache.resident_pages(), 1);
        assert_eq!(cache.stats().page_evictions, 1);
        Ok(())
    }

    #[test]
    fn digest_mismatch_fails_closed() -> Result<(), String> {
        let root = TempRoot::create()?;
        let manifest = write_fixture(&root.0, &[4])?;
        let mut cache = PreparedCache::open(&manifest, 1)?;
        fs::write(
            root.0.join("pages/page-000004.rgba"),
            vec![7_u8; PAGE_BYTES],
        )
        .map_err(|error| format!("mutate temp page: {error}"))?;
        let result = cache.page(4);
        assert!(result.is_err());
        Ok(())
    }
}
