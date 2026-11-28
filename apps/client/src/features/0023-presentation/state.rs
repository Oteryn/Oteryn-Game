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
    pub battle_filters: [[bool; 3]; 3],
    pub channel_volumes: [u8; 4],
    pub ui_sounds: [bool; 4],
    pub master_volume: u8,
    pub ambient_volume: u8,
    pub battle_volumes: [u8; 3],
    pub anthem: bool,
    pub creature_death: bool,
    pub creature_noises: bool,
    pub other_spells: bool,
    pub other_weapons: bool,
    pub own_spells: bool,
    pub own_weapons: bool,
    pub eating: bool,
    pub global: bool,
    pub guild: bool,
    pub move_item: bool,
    pub npc: bool,
    pub party_messages: bool,
    pub private: bool,
    pub private_without_tab: bool,
    pub raid: bool,
    pub system: bool,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 100,
            battle_filters: [[true; 3]; 3],
            channel_volumes: [100; 4],
            ui_sounds: [true; 4],
            ambient_volume: 100,
            battle_volumes: [100; 3],
            anthem: true,
            creature_death: true,
            creature_noises: true,
            other_spells: true,
            other_weapons: true,
            own_spells: true,
            own_weapons: true,
            eating: true,
            global: true,
            guild: true,
            move_item: true,
            npc: true,
            party_messages: true,
            private: true,
            private_without_tab: true,
            raid: true,
            system: true,
            muted: false,
        }
    }
}

impl AudioSettings {
    pub fn ambient_gain(self) -> f32 {
        self.gain() * f32::from(self.ambient_volume.min(100)) / 100.0
    }

    pub fn source_gain(self, source: PresentationSource) -> f32 {
        let volume = match source {
            PresentationSource::Own => self.battle_volumes[0],
            PresentationSource::Others => self.battle_volumes[1],
            PresentationSource::Creatures => self.battle_volumes[2],
            PresentationSource::Global => 100,
        };
        self.gain() * f32::from(volume.min(100)) / 100.0
    }

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
    #[serde(default)]
    battle_classes: BTreeMap<u32, BattleClass>,
    #[serde(default)]
    sound_channels: BTreeMap<u32, SoundChannel>,
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
        || manifest.battle_classes.keys().any(|id| {
            !manifest.sound_cues.contains_key(id)
                || manifest
                    .sound_channels
                    .get(id)
                    .is_some_and(|channel| *channel != SoundChannel::Battle)
        })
        || manifest
            .sound_channels
            .keys()
            .any(|id| !manifest.sound_cues.contains_key(id))
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
    title: Option<String>,
    classes: BTreeMap<u32, BattleClass>,
    channels: BTreeMap<u32, SoundChannel>,
    store: AssetStore,
    cues: BTreeMap<u32, Vec<String>>,
    ambient: BTreeMap<u32, Vec<String>>,
}

impl SoundCatalog {
    pub fn read_title(&self) -> Result<Option<Vec<u8>>, String> {
        self.title
            .as_ref()
            .map(|name| {
                self.store
                    .read_verified(name, MAX_SOUND_BYTES)
                    .map_err(|e| e.to_string())
            })
            .transpose()
    }
    pub fn battle_class(&self, id: u32) -> Option<BattleClass> {
        self.classes.get(&id).copied()
    }
    pub fn channel(&self, id: u32) -> SoundChannel {
        self.channels.get(&id).copied().unwrap_or_default()
    }
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
        let metadata = store
            .read_verified(&metadata, 8 * 1024 * 1024)
            .map_err(|e| e.to_string())?;
        let mut parsed =
            serde_json::from_slice::<SoundManifest>(&bytes).map_err(|e| e.to_string())?;
        let routing =
            oteryn_client_assets::sound::SoundRouting::decode(&metadata).unwrap_or_default();
        for (id, kind) in routing.effects {
            let class = match kind {
                1 => Some(BattleClass::Attack),
                2 => Some(BattleClass::Healing),
                3 => Some(BattleClass::Support),
                4 | 7 => Some(BattleClass::Weapon),
                5 => Some(BattleClass::Noise),
                6 => Some(BattleClass::Death),
                19 => Some(BattleClass::GenericSpell),
                _ => None,
            };
            if let Some(class) = class {
                parsed.battle_classes.entry(id).or_insert(class);
            }
            let channel = match kind {
                9 => Some(SoundChannel::Eating),
                10 => Some(SoundChannel::MoveItem),
                13 => Some(SoundChannel::PrivateWithoutTab),
                15 => Some(SoundChannel::PartyNotification),
                17 => Some(SoundChannel::Raid),
                18 => Some(SoundChannel::SystemMessage),
                _ => None,
            };
            if let Some(channel) = channel {
                parsed.sound_channels.insert(id, channel);
            }
        }
        let ambient = parsed.ambient_cues;
        Ok(Self {
            title: routing.title,
            channels: parsed.sound_channels,
            classes: parsed.battle_classes,
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

#[cfg(any(bevy_shell, feature = "native-audio"))]
#[path = "output.rs"]
mod output;
#[cfg(any(bevy_shell, feature = "native-audio"))]
pub use output::{AudioOutput, audio_device_names};

pub const MAX_VISUAL_CUES: usize = 64;
pub const VISUAL_LIFETIME: std::time::Duration = std::time::Duration::from_millis(30_000);

#[derive(Debug, Clone)]
pub struct VisualCue {
    pub kind: oteryn_session::PresentationEventKind,
    pub received: std::time::Instant,
}

#[path = "ui_sound.rs"]
pub mod ui_sound;
pub use ui_sound::UiSound;

#[path = "device.rs"]
mod device;
pub use device::select_audio_device;

#[path = "mixer.rs"]
mod mixer;
pub use mixer::{BattleClass, SoundChannel};

#[cfg(all(bevy_shell, test))]
pub(crate) use output::check_audio_frame;
