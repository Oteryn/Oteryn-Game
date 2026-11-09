//! C28: bounded cue projection and a manifest-pinned official sound catalog.
use oteryn_client_assets::AssetStore;
use oteryn_session::PresentationSource;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

pub const MAX_PENDING_SOUNDS: usize = 128;
pub const MAX_SOUND_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioSettings {
    pub master_volume: u8,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 100,
            muted: false,
        }
    }
}

impl AudioSettings {
    pub fn gain(self) -> f32 {
        if self.muted {
            0.0
        } else {
            f32::from(self.master_volume.min(100)) / 100.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundCue {
    pub id: u32,
    pub source: PresentationSource,
    /// Ambient ids have their own namespace; zero stops the loop.
    pub ambient: bool,
}

#[derive(Deserialize)]
struct SoundManifest {
    schema: String,
    sound_data: String,
    sound_cues: BTreeMap<u32, Vec<String>>,
    #[serde(default)]
    ambient_cues: BTreeMap<u32, Vec<String>>,
}

fn plain(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0', ':'])
}

pub fn decode_mapping(bytes: &[u8]) -> Result<(String, BTreeMap<u32, Vec<String>>), String> {
    let manifest: SoundManifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if manifest.schema != "OTERYN_CLIENT_ASSET_MANIFEST/v1"
        || !plain(&manifest.sound_data)
        || manifest.sound_cues.is_empty()
        || manifest.sound_cues.len() > 65_535
        || manifest
            .sound_cues
            .iter()
            .chain(manifest.ambient_cues.iter())
            .any(|(id, files)| {
                !(1..=65_535).contains(id)
                    || files.is_empty()
                    || files.len() > 128
                    || files.iter().any(|name| {
                        !plain(name) || !(name.ends_with(".ogg") || name.ends_with(".wav"))
                    })
            })
    {
        return Err(tr!("invalid official sound mapping").into());
    }
    Ok((manifest.sound_data, manifest.sound_cues))
}

pub struct SoundCatalog {
    store: AssetStore,
    cues: BTreeMap<u32, Vec<String>>,
    ambient: BTreeMap<u32, Vec<String>>,
}

impl SoundCatalog {
    /// `sha256` is the trusted pin, supplied separately from the asset directory.
    pub fn open(dir: &Path, manifest: &Path, sha256: &str) -> Result<Self, String> {
        let store = AssetStore::open_pinned(dir, manifest, sha256).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        std::fs::File::open(manifest)
            .map_err(|e| e.to_string())?
            .take(oteryn_client_assets::MAX_MANIFEST_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > oteryn_client_assets::MAX_MANIFEST_BYTES {
            return Err(tr!("sound manifest too large").into());
        }
        // Re-prove the bytes parsed here, so a replaced manifest cannot bypass open_pinned.
        if oteryn_client_assets::text_sha256(&bytes) != sha256.to_ascii_lowercase() {
            return Err(tr!("sound manifest changed").into());
        }
        let (metadata, cues) = decode_mapping(&bytes)?;
        store
            .read_verified(&metadata, 8 * 1024 * 1024)
            .map_err(|e| e.to_string())?;
        let ambient = serde_json::from_slice::<SoundManifest>(&bytes)
            .map_err(|e| e.to_string())?
            .ambient_cues;
        Ok(Self {
            store,
            cues,
            ambient,
        })
    }

    /// Unknown cues are silent; no guessed filename fallback. Variants retain official order.
    pub fn read(&self, id: u32, variant: usize) -> Result<Option<Vec<u8>>, String> {
        self.read_channel(id, variant, false)
    }

    pub fn read_channel(
        &self,
        id: u32,
        variant: usize,
        ambient: bool,
    ) -> Result<Option<Vec<u8>>, String> {
        let Some(files) = (if ambient { &self.ambient } else { &self.cues }).get(&id) else {
            return Ok(None);
        };
        self.store
            .read_verified(&files[variant % files.len()], MAX_SOUND_BYTES)
            .map(Some)
            .map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
#[path = "native.rs"]
pub mod native;

#[cfg(windows)]
pub use native::{AudioOutput, NativeAudio};

pub const MAX_VISUAL_CUES: usize = 64;
pub const VISUAL_LIFETIME: std::time::Duration = std::time::Duration::from_millis(1000);

#[derive(Debug, Clone)]
pub struct VisualCue {
    pub kind: oteryn_session::PresentationEventKind,
    pub received: std::time::Instant,
}
