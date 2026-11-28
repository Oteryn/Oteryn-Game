//! Store item keys on disk. Wire indices are meaningful only within one content generation.
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{io, sync::OnceLock};

struct Catalogue {
    active: Vec<&'static str>,
    legacy: Vec<Option<&'static str>>,
}

fn catalogue() -> io::Result<&'static Catalogue> {
    static KEYS: OnceLock<Result<Catalogue, String>> = OnceLock::new();
    KEYS.get_or_init(|| {
        #[derive(Deserialize)]
        struct Document<'a> {
            #[serde(borrow)]
            records: Vec<(&'a str, &'a str)>,
        }
        let doc: Document<'_> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/content-schema/native-gameplay/item-keys.json"
        )))
        .map_err(|e| e.to_string())?;
        let active: Vec<_> = doc.records.into_iter().map(|(key, _)| key).collect();
        // 7a shipped before 7b. Both earlier catalogues wrote indistinguishable numeric files.
        const TRAIN_A: [&str; 7] = [
            "oteryn:item.tibia.i35136",
            "oteryn:item.tibia.i35686",
            "oteryn:item.tibia.i48109",
            "oteryn:item.tibia.i48340",
            "oteryn:item.tibia.i48396",
            "oteryn:item.tibia.i48401",
            "oteryn:item.tibia.i49120",
        ];
        let original: Vec<_> = active
            .iter()
            .copied()
            .filter(|key| {
                !TRAIN_A.contains(key)
                    && !matches!(
                        *key,
                        "oteryn:item.tibia.i36547" | "oteryn:item.tibia.i36548"
                    )
            })
            .collect();
        // Prove the full pre-upgrade key order (e2750951) before interpreting bare numbers.
        // A later catalogue that cannot reconstruct this order discards unversioned bindings.
        let mut digest = Sha256::new();
        for key in &original {
            digest.update(key.as_bytes());
            digest.update([0]);
        }
        let legacy = if digest.finalize().as_slice()
            == [
                0x63, 0x20, 0xb1, 0xaf, 0x52, 0x09, 0x6a, 0x98, 0xe4, 0x2d, 0x88, 0x1f, 0xb9, 0xfe,
                0xd5, 0x1b, 0x97, 0xc0, 0xe6, 0x06, 0x69, 0xc2, 0x86, 0x68, 0x10, 0xc8, 0x57, 0x41,
                0x50, 0x7a, 0x6d, 0xda,
            ] {
            let mut train_a = original.clone();
            train_a.extend(TRAIN_A);
            train_a.sort_unstable();
            // ef4c9e9d also wrote bare indices, with the two 7b keys but without 7a.
            let mut review_base = original.clone();
            review_base.extend(["oteryn:item.tibia.i36547", "oteryn:item.tibia.i36548"]);
            review_base.sort_unstable();
            train_a
                .into_iter()
                .enumerate()
                .map(|(i, key)| {
                    (original.get(i).is_none_or(|old| *old == key)
                        && review_base.get(i).is_none_or(|old| *old == key))
                    .then_some(key)
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(Catalogue { active, legacy })
    })
    .as_ref()
    .map_err(|error| io::Error::other(error.clone()))
}

impl super::PreferenceStore {
    pub(in crate::play) fn item_source_id(definition: std::num::NonZeroU32) -> Option<u32> {
        let key = catalogue()
            .ok()?
            .active
            .get(definition.get() as usize - 1)?;
        key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
    }
}

#[derive(Clone, Copy)]
pub(super) enum Operation {
    Save,
    Load,
}

impl Operation {
    // A missing key invalidates only the shortcut containing it.
    fn resolve(self, value: &mut Value) -> io::Result<bool> {
        let keys = catalogue()?;
        let index = value
            .as_u64()
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| usize::try_from(n).ok());
        match self {
            Self::Save => {
                let key = index
                    .and_then(|i| keys.active.get(i))
                    .ok_or_else(|| io::Error::other("Unknown item shortcut definition"))?;
                *value = Value::from(*key);
            }
            Self::Load => {
                let key = match (value.as_str(), index) {
                    (Some(key), _) => key,
                    (_, Some(i)) => match keys.legacy.get(i).copied().flatten() {
                        Some(key) => key,
                        None => return Ok(false),
                    },
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Invalid item shortcut",
                        ));
                    }
                };
                let Ok(i) = keys.active.binary_search(&key) else {
                    return Ok(false);
                };
                *value = Value::from(i + 1);
            }
        }
        Ok(true)
    }

    fn action(self, value: &mut Value) -> io::Result<bool> {
        if let Some(definition) = value.get_mut("definition") {
            return self.resolve(definition);
        }
        if let Some(actions) = value.get_mut("actions").and_then(Value::as_array_mut) {
            for action in actions {
                if !self.action(action)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn bindings(self, value: &mut Value) -> io::Result<()> {
        if let Some(map) = value.as_object_mut() {
            let mut missing = Vec::new();
            for (slot, action) in map.iter_mut() {
                if !self.action(action)? {
                    missing.push(slot.clone());
                }
            }
            for slot in missing {
                map.remove(&slot);
            }
        }
        Ok(())
    }

    pub(super) fn apply(self, document: &mut Value) -> io::Result<()> {
        for key in ["bar_actions", "custom_hotkeys"] {
            if let Some(map) = document.get_mut(key) {
                self.bindings(map)?;
            }
        }
        for key in ["bar_action_profiles", "custom_hotkey_profiles"] {
            if let Some(profiles) = document.get_mut(key).and_then(Value::as_object_mut) {
                for map in profiles.values_mut() {
                    self.bindings(map)?;
                }
            }
        }
        if let Some(slots) = document
            .pointer_mut("/item_hotkeys/bindings")
            .and_then(Value::as_array_mut)
        {
            for binding in slots {
                if !binding.is_null() && !self.resolve(binding)? {
                    *binding = Value::Null;
                }
            }
        }
        Ok(())
    }
}
