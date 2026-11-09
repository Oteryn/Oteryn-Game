//! Pixel previews from the same pinned asset store as live play; no ownership is granted here.
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
use oteryn_client_ui::appearance::Panel;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

pub(super) struct Image {
    pub(super) rgba: Vec<u8>,
}
pub(super) struct PreviewAssets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
    mounts: BTreeMap<String, u32>,
    cosmetics_digest: Option<[u8; 32]>,
}
impl PreviewAssets {
    pub(super) fn load(root: &Path) -> Result<Self, String> {
        use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|error| error.to_string())?;
        let catalog = Catalog::load(&store).map_err(|error| error.to_string())?;
        let index = AppearanceIndex::load(&store, &catalog).map_err(|error| error.to_string())?;
        let metadata = Self::mount_metadata(root).ok();
        let (cosmetics_digest, mounts) = metadata
            .map_or((None, BTreeMap::new()), |(digest, mounts)| {
                (Some(digest), mounts)
            });
        Ok(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
            mounts,
            cosmetics_digest,
        })
    }
    fn mount_metadata(root: &Path) -> Result<([u8; 32], BTreeMap<String, u32>), String> {
        // Presentation metadata is accepted only when its normalized document matches the server's digest.
        // It never adds choices to the server-owned catalogue or grants an entitlement.
        let mut bytes = Vec::new();
        std::fs::File::open(root.join("content/cosmetics/outfits/authoring.json"))
            .map_err(|e| e.to_string())?
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("Appearance metadata exceeds its bound".into());
        }
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let document = value
            .get_mut("document")
            .ok_or("Appearance metadata unavailable")?;
        document["schema"] = "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1".into();
        let normalized = serde_json::to_vec(document).map_err(|e| e.to_string())?;
        let digest: [u8; 32] = Sha256::digest(&normalized).into();
        let entries = document["mounts"]
            .as_array()
            .filter(|items| items.len() <= 256)
            .ok_or("Mount metadata unavailable")?;
        let mut mounts = BTreeMap::new();
        for entry in entries {
            let key = entry["key"].as_str().ok_or("Mount key unavailable")?;
            let look = entry["look"]["id"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n != 0)
                .ok_or("Mount artwork unavailable")?;
            mounts.insert(key.to_owned(), look);
        }
        Ok((digest, mounts))
    }
    pub(super) fn render_memorial(
        &mut self,
        panel: &oteryn_client_ui::appearance::MemorialPanel,
    ) -> Result<Image, String> {
        let mut image = Image {
            rgba: [32, 32, 32, 255].repeat(128 * 128),
        };
        self.draw_look(panel.look(), panel.addons, None, false, &mut image)?;
        Ok(image)
    }
    pub(super) fn render(&mut self, panel: &Panel) -> Result<Image, String> {
        let catalogue = panel
            .catalogue
            .as_ref()
            .ok_or("Waiting for appearance choices")?;
        let outfit = catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.key == panel.fields[0])
            .ok_or("Choose an outfit")?;
        let addons = panel.fields[1]
            .trim()
            .parse::<u8>()
            .map_err(|_| "Choose valid addons")?;
        if addons & !outfit.permitted_addons != 0 {
            return Err("Choose permitted addons".into());
        }
        let colours = panel
            .preview_colours()
            .ok_or("Choose colours from the palette")?;
        let mount = panel.fields[2].trim();
        let mut image = Image {
            rgba: [32, 32, 32, 255].repeat(128 * 128),
        };
        if !mount.is_empty() {
            if !catalogue.mounts.iter().any(|key| key == mount) {
                return Err("Choose an owned mount".into());
            }
            if self.cosmetics_digest != Some(catalogue.context.cosmetics_digest) {
                return Err("Mount artwork needs updating".into());
            }
            let look = *self.mounts.get(mount).ok_or("Mount artwork unavailable")?;
            self.draw_look(look, 0, None, false, &mut image)?;
        }
        self.draw_look(
            outfit.look_type,
            addons,
            Some(colours),
            !mount.is_empty(),
            &mut image,
        )?;
        Ok(image)
    }
}
#[path = "preview_pixels.rs"]
mod pixels;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_preview_renders_pinned_pixels_and_keeps_server_selection_authoritative()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client_runtime::appearance::{Catalogue, Context, OutfitOption};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut assets = PreviewAssets::load(&root)?;
        let catalogue = Catalogue {
            context: Context {
                content_generation: [1; 32],
                cosmetics_digest: assets.cosmetics_digest.ok_or("cosmetics digest")?,
            },
            root_revision: 1,
            selected: None,
            palette: vec![[255, 0, 0], [0, 0, 255]],
            outfits: vec![OutfitOption {
                key: "oteryn:outfit.citizen".into(),
                look_type: 128,
                permitted_addons: 3,
            }],
            mounts: vec!["oteryn:content.mount.haze".into()],
        };
        let wire = catalogue.encode().map_err(|_| "catalogue encode")?;
        let mut panel = oteryn_client_ui::appearance::Panel {
            selected: true,
            catalogue: Some(Catalogue::decode(&wire).map_err(|_| "catalogue decode")?),
            ..Default::default()
        };
        panel.fields = [
            "oteryn:outfit.citizen".into(),
            "0".into(),
            String::new(),
            "0,0,0,0".into(),
            "preview-r1".into(),
        ];
        let plain = assets.render(&panel)?;
        assert_eq!(plain.rgba.len(), 128 * 128 * 4);
        assert!(
            plain
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| *pixel != [32, 32, 32, 255])
        );
        panel.fields[3] = "1,1,1,1".into();
        assert_ne!(assets.render(&panel)?.rgba, plain.rgba);
        let recoloured = assets.render(&panel)?;
        panel.fields[1] = "3".into();
        assert_ne!(assets.render(&panel)?.rgba, recoloured.rgba);
        assert!(panel.cycle_mount(true));
        let mounted = assets.render(&panel)?;
        assert_ne!(mounted.rgba, plain.rgba);
        let command = crate::play::feature_0025_appearance::protocol::command(
            &panel,
            oteryn_client_ui::appearance::Intent::Apply,
        )?;
        let change = oteryn_client_runtime::appearance::Change::decode(&command.payload)
            .map_err(|_| "change decode")?;
        assert_eq!(
            change.selection.mount_key.as_deref(),
            Some("oteryn:content.mount.haze")
        );
        assert_eq!(change.selection.addons, 3);
        assert_eq!(change.selection.colours, [1, 1, 1, 1]);
        assert!(panel.confirmed.is_none());
        if let Some(catalogue) = &mut panel.catalogue {
            catalogue.context.cosmetics_digest[0] ^= 1;
        }
        assert!(assets.render(&panel).is_err());
        Ok(())
    }
}
#[cfg(test)]
#[path = "memorial_art_tests.rs"]
mod memorial_art_tests;
