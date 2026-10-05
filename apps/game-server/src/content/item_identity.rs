//! Item identity equals the Tibia id (decision `A12-ITEM-IDENTITY-TIBIA-ID-V1`, D146-D149).
//!
//! The canonical key of a CipSoft item is `oteryn:item.tibia.i<id>` (§4.1). Every earlier Item
//! key (the CW2-B1 and epoch-2 registry keys and the named keys) is retired, and
//! `content/items/aliases.json` holds exactly one current entry for each: an alias to the Tibia
//! key of its own recorded id, or `RETIRED_WITHOUT_SUCCESSOR` (D149) with a tombstone.
//!
//! The protected import packets stay byte-exact history and still name retired keys. The
//! materializer admits them in that historical key space and then applies
//! [`apply_tibia_id_key_rule`] once, so authored content only ever names canonical keys.

use super::{
    ItemStackDocument, ProjectReferenceRecord, ProjectV2Draft, ProjectionDocument,
    world_project_sha256,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display, Formatter},
};

pub const TIBIA_ITEM_KEY_PREFIX: &str = "oteryn:item.tibia.i";
pub const OTERYN_ITEM_KEY_PREFIX: &str = "oteryn:item.oteryn.";
pub const TIBIA_ITEM_KEY_RULE: &str = "OTERYN_TIBIA_ID_KEY_RULE_V1";
pub const ITEM_KEY_ALIAS_TABLE_BYTES: usize = 15_667_171;
pub const ITEM_KEY_ALIAS_TABLE_SHA256: &str =
    "128bc354816199702c26f83120e6fb7846fb2608fb5ee1c16085f63268e592f2";
const ALIAS_TABLE_SCHEMA: &str = "OTERYN_ITEM_KEY_ALIAS_TABLE/v1";
const ITEM_KEY_PREFIX: &str = "oteryn:item.";
/// ITEM-ADD-1 owner 2a: the accepted current-appearance-only cohort, absent from donor XML, plus
/// the admitted Snowball with Ice Shards appearance 53855 (D283a) and the r25 spell-profile
/// appearance 40450 (ITEM-KEY-R25-1).
pub const APPEARANCE_ONLY_ITEM_IDS: [u64; 62] = [
    21887, 35384, 35388, 35600, 35846, 36929, 39949, 40450, 40522, 43666, 43762, 43771, 43778,
    43779, 43780, 43781, 43782, 43946, 43947, 43959, 44048, 44432, 44433, 44447, 44664, 44665,
    44666, 44667, 44668, 44669, 44670, 44671, 44684, 44685, 44686, 44687, 44709, 44713, 44717,
    48108, 48112, 48271, 48349, 48353, 48366, 48382, 48403, 48404, 48405, 48406, 48414, 48416,
    49124, 51276, 51302, 51560, 53197, 53199, 53201, 53203, 53205, 53855,
];

/// The retired named Item keys as code constants (D147): each is the Tibia key of the item the
/// name stood for. The retired strings resolve only through the alias table.
pub mod semantic {
    pub const AMMUNITION_ARROW: &str = "oteryn:item.tibia.i3447";
    pub const ARMOR_PLATE_ARMOR: &str = "oteryn:item.tibia.i3357";
    pub const CHARGED_MIGHT_RING: &str = "oteryn:item.tibia.i3048";
    pub const CHARGED_SACRED_TREE_AMULET: &str = "oteryn:item.tibia.i9302";
    pub const CHARGED_STONE_SKIN_AMULET: &str = "oteryn:item.tibia.i3081";
    pub const CONSUMABLE_POTION_GREAT_HEALTH: &str = "oteryn:item.tibia.i239";
    pub const CONSUMABLE_POTION_HEALTH: &str = "oteryn:item.tibia.i266";
    pub const CONSUMABLE_POTION_MANA: &str = "oteryn:item.tibia.i268";
    pub const CONSUMABLE_POTION_STRONG_MANA: &str = "oteryn:item.tibia.i237";
    pub const CONSUMABLE_SUDDEN_DEATH_RUNE: &str = "oteryn:item.tibia.i3155";
    pub const CONTAINER_BOOK_BACKPACK: &str = "oteryn:item.tibia.i28571";
    pub const CONTAINER_DEEPLING_BACKPACK: &str = "oteryn:item.tibia.i14248";
    pub const CONTAINER_FUR_BAG: &str = "oteryn:item.tibia.i7343";
    pub const CONTAINER_GOLDEN_BACKPACK: &str = "oteryn:item.tibia.i2871";
    pub const CONTAINER_MOON_BACKPACK: &str = "oteryn:item.tibia.i9604";
    pub const CONTAINER_PILLOW_BACKPACK: &str = "oteryn:item.tibia.i24393";
    pub const CONTAINER_PIRATE_BACKPACK: &str = "oteryn:item.tibia.i5926";
    pub const CONTAINER_PIRATE_BAG: &str = "oteryn:item.tibia.i5927";
    pub const CURRENCY_CRYSTAL_COIN: &str = "oteryn:item.tibia.i3043";
    pub const CURRENCY_GOLD_COIN: &str = "oteryn:item.tibia.i3031";
    pub const CURRENCY_PLATINUM_COIN: &str = "oteryn:item.tibia.i3035";
    pub const DECOR_CANDLESTICK: &str = "oteryn:item.tibia.i2917";
    pub const DECOR_FLASK_BROWN: &str = "oteryn:item.tibia.i2885";
    pub const DECOR_OIL_LAMP_SMALL: &str = "oteryn:item.tibia.i2933";
    pub const DECOR_PANPIPES: &str = "oteryn:item.tibia.i2953";
    pub const DECOR_PIGGY_BANK: &str = "oteryn:item.tibia.i2995";
    pub const DECOR_PILLOW_SMALL_BLUE: &str = "oteryn:item.tibia.i2389";
    pub const DECOR_TEDDY_BEAR: &str = "oteryn:item.tibia.i2993";
    pub const DECOR_TOME_BLUE: &str = "oteryn:item.tibia.i2850";
    pub const DECOR_TOME_PURPLE: &str = "oteryn:item.tibia.i2848";
    pub const DECOR_TOME_RED: &str = "oteryn:item.tibia.i2852";
    pub const DECOR_VASE: &str = "oteryn:item.tibia.i2876";
    pub const EQUIPMENT_CROWN_HELMET: &str = "oteryn:item.tibia.i3385";
    pub const EQUIPMENT_CRUSADER_HELMET: &str = "oteryn:item.tibia.i3391";
    pub const EQUIPMENT_GLACIER_ROBE: &str = "oteryn:item.tibia.i824";
    pub const EQUIPMENT_KNIGHT_ARMOR: &str = "oteryn:item.tibia.i3370";
    pub const EQUIPMENT_MAGMA_MONOCLE: &str = "oteryn:item.tibia.i827";
    pub const EQUIPMENT_PLATE_LEGS: &str = "oteryn:item.tibia.i3557";
    pub const EQUIPMENT_PLATINUM_AMULET: &str = "oteryn:item.tibia.i3055";
    pub const EQUIPMENT_STEEL_HELMET: &str = "oteryn:item.tibia.i3351";
    pub const EQUIPMENT_STEEL_SHIELD: &str = "oteryn:item.tibia.i3409";
    pub const EQUIPMENT_TERRA_LEGS: &str = "oteryn:item.tibia.i812";
    pub const EQUIPMENT_WOODEN_SHIELD: &str = "oteryn:item.tibia.i3412";
    pub const MATERIAL_GEM_SMALL_ENCHANTED_AMETHYST: &str = "oteryn:item.tibia.i678";
    pub const MATERIAL_GEM_SMALL_ENCHANTED_EMERALD: &str = "oteryn:item.tibia.i677";
    pub const MATERIAL_GEM_SMALL_ENCHANTED_RUBY: &str = "oteryn:item.tibia.i676";
    pub const MATERIAL_GEM_SMALL_ENCHANTED_SAPPHIRE: &str = "oteryn:item.tibia.i675";
    pub const MATERIAL_WORM: &str = "oteryn:item.tibia.i3492";
    pub const PHYSICAL_CRYSTAL_ICE_FLAWLESS: &str = "oteryn:item.tibia.i942";
    pub const PHYSICAL_MARLIN: &str = "oteryn:item.tibia.i901";
    pub const PHYSICAL_NAIL: &str = "oteryn:item.tibia.i953";
    pub const PHYSICAL_SOIL_GLIMMERING: &str = "oteryn:item.tibia.i941";
    pub const PHYSICAL_SOIL_NATURAL: &str = "oteryn:item.tibia.i940";
    pub const WEAPON_AXE_FIRE: &str = "oteryn:item.tibia.i3320";
    pub const WEAPON_AXE_HAND: &str = "oteryn:item.tibia.i3268";
    pub const WEAPON_AXE_KNIGHT: &str = "oteryn:item.tibia.i3318";
    pub const WEAPON_BLADE_DAGGER: &str = "oteryn:item.tibia.i3267";
    pub const WEAPON_CLUB_BATTLE_HAMMER: &str = "oteryn:item.tibia.i3305";
    pub const WEAPON_CLUB_MACE: &str = "oteryn:item.tibia.i3286";
    pub const WEAPON_CLUB_SKULL_STAFF: &str = "oteryn:item.tibia.i3324";
    pub const WEAPON_RANGED_BOW: &str = "oteryn:item.tibia.i3350";
    pub const WEAPON_RANGED_CROSSBOW: &str = "oteryn:item.tibia.i3349";
    pub const WEAPON_SWORD_BRIGHT: &str = "oteryn:item.tibia.i3295";
    pub const WEAPON_SWORD_FIRE: &str = "oteryn:item.tibia.i3280";
    pub const WEAPON_WAND_SNAKEBITE_ROD: &str = "oteryn:item.tibia.i3066";
}

/// `oteryn:item.tibia.i<id>`: decimal, no padding, no leading zero (§4.1).
pub fn tibia_item_key(tibia_id: u64) -> Option<String> {
    (tibia_id != 0).then(|| format!("{TIBIA_ITEM_KEY_PREFIX}{tibia_id}"))
}

/// Whether `key` has a canonical form: a Tibia key or a D148 Oteryn-only key.
pub fn is_canonical_item_key(key: &str) -> bool {
    if let Some(id) = key.strip_prefix(TIBIA_ITEM_KEY_PREFIX) {
        return !id.is_empty()
            && !id.starts_with('0')
            && id.bytes().all(|byte| byte.is_ascii_digit())
            && id.parse::<u64>().is_ok();
    }
    key.strip_prefix(OTERYN_ITEM_KEY_PREFIX)
        .is_some_and(|slug| {
            slug.split('_').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            }) && slug.starts_with(|c: char| c.is_ascii_lowercase())
        })
}

/// The current entry of one retired key (§4.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetiredItemKey {
    Alias { target: String },
    WithoutSuccessor { tombstone_sha256: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemIdentityError {
    AliasTableBytes,
    AliasTable(&'static str),
    RuleMismatch(String),
    RetiredReference(String),
    DanglingReference(String),
    UntypedItemKey(String),
    Serde(String),
}

impl Display for ItemIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AliasTableBytes => write!(formatter, "Item key alias table bytes drifted"),
            Self::AliasTable(reason) => write!(formatter, "Item key alias table: {reason}"),
            Self::RuleMismatch(key) => {
                write!(
                    formatter,
                    "Item key {key} does not follow the Tibia id rule"
                )
            }
            Self::RetiredReference(key) => {
                write!(
                    formatter,
                    "reference names a key retired without successor: {key}"
                )
            }
            Self::DanglingReference(key) => {
                write!(formatter, "Item reference has no canonical record: {key}")
            }
            Self::UntypedItemKey(key) => {
                write!(formatter, "Item key outside an Item identity: {key}")
            }
            Self::Serde(error) => write!(formatter, "Item key switch round trip: {error}"),
        }
    }
}

impl std::error::Error for ItemIdentityError {}

/// The verified alias table: the current entry of every retired key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemKeyAliasTable {
    current: BTreeMap<String, RetiredItemKey>,
}

impl ItemKeyAliasTable {
    /// Admit the exact committed table bytes. A later entry version supersedes an earlier one
    /// only when the earlier one is `RETIRED_WITHOUT_SUCCESSOR` (§4.5).
    pub fn parse(bytes: &[u8]) -> Result<Self, ItemIdentityError> {
        if bytes.len() != ITEM_KEY_ALIAS_TABLE_BYTES
            || world_project_sha256(bytes) != ITEM_KEY_ALIAS_TABLE_SHA256
        {
            return Err(ItemIdentityError::AliasTableBytes);
        }
        let table: Value = serde_json::from_slice(bytes)
            .map_err(|_| ItemIdentityError::AliasTable("JSON decoding"))?;
        if table["schema"] != ALIAS_TABLE_SCHEMA || table["key_rule"] != TIBIA_ITEM_KEY_RULE {
            return Err(ItemIdentityError::AliasTable("schema"));
        }
        let entries = table["entries"]
            .as_array()
            .ok_or(ItemIdentityError::AliasTable("entries"))?;
        let mut current = BTreeMap::<String, RetiredItemKey>::new();
        let mut versions = BTreeMap::<String, u64>::new();
        for entry in entries {
            let key = entry["key"]
                .as_str()
                .filter(|key| key.starts_with(ITEM_KEY_PREFIX) && !is_canonical_item_key(key))
                .ok_or(ItemIdentityError::AliasTable("retired key"))?;
            let version = entry["version"]
                .as_u64()
                .ok_or(ItemIdentityError::AliasTable("version"))?;
            let resolution = match entry["state"].as_str() {
                Some("ALIAS") => {
                    let target = entry["target"]
                        .as_str()
                        .ok_or(ItemIdentityError::AliasTable("alias target"))?;
                    let own = entry["evidence"]["source_item_id"]
                        .as_u64()
                        .and_then(tibia_item_key);
                    if own.as_deref() != Some(target) {
                        return Err(ItemIdentityError::AliasTable("alias target is not own id"));
                    }
                    RetiredItemKey::Alias {
                        target: target.to_owned(),
                    }
                }
                Some("RETIRED_WITHOUT_SUCCESSOR") => RetiredItemKey::WithoutSuccessor {
                    tombstone_sha256: entry["tombstone_sha256"]
                        .as_str()
                        .ok_or(ItemIdentityError::AliasTable("tombstone digest"))?
                        .to_owned(),
                },
                _ => return Err(ItemIdentityError::AliasTable("state")),
            };
            let previous = versions.insert(key.to_owned(), version).unwrap_or(0);
            if version != previous + 1
                || (previous != 0
                    && !matches!(
                        current.get(key),
                        Some(RetiredItemKey::WithoutSuccessor { .. })
                    ))
            {
                return Err(ItemIdentityError::AliasTable("version chain"));
            }
            current.insert(key.to_owned(), resolution);
        }
        Ok(Self { current })
    }

    pub fn resolve(&self, key: &str) -> Option<&RetiredItemKey> {
        self.current.get(key)
    }

    pub fn len(&self) -> usize {
        self.current.len()
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }
}

/// What the key switch changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemKeySwitch {
    pub item_records: usize,
    pub removed_without_successor: usize,
    pub rewritten_references: usize,
}

/// Apply the §4.1 rule to a draft admitted in the historical key space.
///
/// `source_ids` maps each historical Item record key to its CW2-B1 source item id. Every Item
/// record is re-keyed to `tibia_item_key(source id)`, which must equal its alias entry; a
/// record whose entry is `RETIRED_WITHOUT_SUCCESSOR` is removed (D149). Every other Item
/// identity in the draft is rewritten through the alias table and must name a kept record;
/// a reference to a D149 key, a dangling reference or an Item key outside an Item identity
/// fails closed.
pub fn apply_tibia_id_key_rule(
    draft: &mut ProjectV2Draft,
    table: &ItemKeyAliasTable,
    source_ids: &BTreeMap<String, u64>,
) -> Result<ItemKeySwitch, ItemIdentityError> {
    apply_key_rule(draft, table, source_ids, Vec::new())
}

/// ITEM-ADD-1 owner 2a: include proof-backed minimal appearance Items in reference closure.
///
/// The original historical-only switch stays strict. This extension verifies the same pinned
/// admitted appearance index used by the materializer and accepts only current CipSoft ids,
/// absent from the source allocation, with the already accepted identity-only Item shape.
pub fn apply_tibia_id_key_rule_with_appearance_items(
    draft: &mut ProjectV2Draft,
    table: &ItemKeyAliasTable,
    source_ids: &BTreeMap<String, u64>,
    items: Vec<ProjectReferenceRecord>,
) -> Result<ItemKeySwitch, ItemIdentityError> {
    let index = include_bytes!("../../../../imports/official/appearance-membership/admitted.json");
    let manifest = include_bytes!(
        "../../../../imports/official/appearance-membership/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.json"
    );
    validate_appearance_items(&items, source_ids, index, manifest)?;
    apply_key_rule(draft, table, source_ids, items)
}

fn validate_appearance_items(
    items: &[ProjectReferenceRecord],
    source_ids: &BTreeMap<String, u64>,
    index_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<(), ItemIdentityError> {
    if world_project_sha256(index_bytes)
        != "19f99b709d9a28c1730632e27672adb0a03ef11bf7d1f4b2647a4090f3df0718"
    {
        return Err(ItemIdentityError::AliasTable(
            "admitted appearance index digest",
        ));
    }
    let index: Value = serde_json::from_slice(index_bytes)
        .map_err(|error| ItemIdentityError::Serde(error.to_string()))?;
    let newest = index["files"]
        .as_array()
        .and_then(|files| files.last())
        .ok_or(ItemIdentityError::AliasTable(
            "admitted appearance manifest",
        ))?;
    if index["newest"] != "client-15.30"
        || newest["label"] != index["newest"]
        || newest["manifest_sha256"].as_str() != Some(&world_project_sha256(manifest_bytes))
    {
        return Err(ItemIdentityError::AliasTable(
            "current appearance manifest digest",
        ));
    }
    let manifest: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| ItemIdentityError::Serde(error.to_string()))?;
    let current = manifest["entries"]
        .as_array()
        .ok_or(ItemIdentityError::AliasTable("current appearance entries"))?
        .iter()
        .map(|entry| {
            entry[0]
                .as_u64()
                .ok_or(ItemIdentityError::AliasTable("current appearance id"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let source_ids = source_ids.values().copied().collect::<BTreeSet<_>>();
    let mut added = BTreeSet::new();
    for record in items {
        let ProjectReferenceRecord::Item {
            identity,
            client_projection,
            materializable,
            stack_class,
            semantics,
        } = record
        else {
            return Err(ItemIdentityError::AliasTable(
                "appearance record is not an Item",
            ));
        };
        let id = identity
            .key
            .strip_prefix(TIBIA_ITEM_KEY_PREFIX)
            .and_then(|id| id.parse::<u64>().ok());
        if identity.family != "Item"
            || identity.revision != "definition-r1"
            || id.and_then(tibia_item_key).as_deref() != Some(identity.key.as_str())
            || !id.is_some_and(|id| {
                APPEARANCE_ONLY_ITEM_IDS.contains(&id)
                    && current.contains(&id)
                    && !source_ids.contains(&id)
            })
            || *client_projection != ProjectionDocument::ClientSafe
            || *materializable
            || *stack_class != ItemStackDocument::Unknown
            || !semantics.is_all_unknown()
            || !added.insert(identity.key.clone())
        {
            return Err(ItemIdentityError::RuleMismatch(identity.key.clone()));
        }
    }
    Ok(())
}

fn apply_key_rule(
    draft: &mut ProjectV2Draft,
    table: &ItemKeyAliasTable,
    source_ids: &BTreeMap<String, u64>,
    appearance_items: Vec<ProjectReferenceRecord>,
) -> Result<ItemKeySwitch, ItemIdentityError> {
    let mut canonical = BTreeSet::new();
    let mut removed = 0_usize;
    let mut kept = Vec::with_capacity(draft.core.records.len());
    for record in std::mem::take(&mut draft.core.records) {
        if let ProjectReferenceRecord::Item { identity, .. } = &record {
            let key = identity.key.as_str();
            match table.resolve(key) {
                Some(RetiredItemKey::Alias { target }) => {
                    let by_rule = source_ids.get(key).copied().and_then(tibia_item_key);
                    if by_rule.as_ref() != Some(target) || !canonical.insert(target.clone()) {
                        return Err(ItemIdentityError::RuleMismatch(key.to_owned()));
                    }
                }
                Some(RetiredItemKey::WithoutSuccessor { .. }) => {
                    removed += 1;
                    continue;
                }
                None => return Err(ItemIdentityError::RuleMismatch(key.to_owned())),
            }
        }
        kept.push(record);
    }
    for record in appearance_items {
        let ProjectReferenceRecord::Item { identity, .. } = &record else {
            return Err(ItemIdentityError::AliasTable(
                "appearance record is not an Item",
            ));
        };
        if !canonical.insert(identity.key.clone()) {
            return Err(ItemIdentityError::RuleMismatch(identity.key.clone()));
        }
        kept.push(record);
    }
    draft.core.records = kept;

    let mut switch = Switch {
        table,
        canonical: &canonical,
        rewritten: 0,
    };
    switch.all(&mut draft.core.records)?;
    switch.all(&mut draft.state.declarations)?;
    switch.all(&mut draft.state.item_authoring)?;
    switch.all(&mut draft.state.authoring_profiles)?;
    switch.all(&mut draft.state.source_identity_bindings)?;
    switch.all(&mut draft.state.editor)?;
    switch.all(&mut draft.state.placements)?;
    switch.all(&mut draft.state.appearance_bindings)?;
    switch.all(&mut draft.core.imports)?;
    switch.all(&mut draft.core.metadata)?;
    Ok(ItemKeySwitch {
        item_records: canonical.len(),
        removed_without_successor: removed,
        rewritten_references: switch.rewritten,
    })
}

struct Switch<'a> {
    table: &'a ItemKeyAliasTable,
    canonical: &'a BTreeSet<String>,
    rewritten: usize,
}

impl Switch<'_> {
    fn all<T: Serialize + DeserializeOwned>(
        &mut self,
        values: &mut [T],
    ) -> Result<(), ItemIdentityError> {
        for value in values {
            let mut json = serde_json::to_value(&*value)
                .map_err(|error| ItemIdentityError::Serde(error.to_string()))?;
            if self.value(&mut json)? {
                *value = serde_json::from_value(json)
                    .map_err(|error| ItemIdentityError::Serde(error.to_string()))?;
            }
        }
        Ok(())
    }

    /// Rewrite every `{"family": "Item", "key": ..}` below `value`; true when any changed.
    fn value(&mut self, value: &mut Value) -> Result<bool, ItemIdentityError> {
        match value {
            Value::Object(map) => {
                let mut changed = false;
                let is_item = map.get("family").and_then(Value::as_str) == Some("Item");
                for (field, child) in map.iter_mut() {
                    if is_item
                        && field == "key"
                        && let Value::String(key) = child
                    {
                        let target = self.target(key)?;
                        if target != *key {
                            *key = target;
                            changed = true;
                        }
                        self.rewritten += 1;
                        continue;
                    }
                    changed |= self.value(child)?;
                }
                Ok(changed)
            }
            Value::Array(values) => {
                let mut changed = false;
                for child in values {
                    changed |= self.value(child)?;
                }
                Ok(changed)
            }
            Value::String(text) if text.starts_with(ITEM_KEY_PREFIX) => {
                Err(ItemIdentityError::UntypedItemKey(text.clone()))
            }
            _ => Ok(false),
        }
    }

    fn target(&self, key: &str) -> Result<String, ItemIdentityError> {
        let target = match self.table.resolve(key) {
            Some(RetiredItemKey::Alias { target }) => target.as_str(),
            Some(RetiredItemKey::WithoutSuccessor { .. }) => {
                return Err(ItemIdentityError::RetiredReference(key.to_owned()));
            }
            None => key,
        };
        if !self.canonical.contains(target) {
            return Err(ItemIdentityError::DanglingReference(key.to_owned()));
        }
        Ok(target.to_owned())
    }
}

#[cfg(test)]
mod appearance_proof_tests {
    use super::{ItemIdentityError, validate_appearance_items};
    use std::collections::BTreeMap;

    const INDEX: &[u8] =
        include_bytes!("../../../../imports/official/appearance-membership/admitted.json");
    const MANIFEST: &[u8] = include_bytes!(
        "../../../../imports/official/appearance-membership/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.json"
    );

    #[test]
    fn admission_proof_cannot_be_replaced_by_self_consistent_unpinned_bytes() {
        let mut unpinned = INDEX.to_vec();
        unpinned.push(b'\n');
        assert_eq!(
            validate_appearance_items(&[], &BTreeMap::new(), &unpinned, MANIFEST),
            Err(ItemIdentityError::AliasTable(
                "admitted appearance index digest"
            ))
        );
    }

    #[test]
    fn current_manifest_bytes_are_bound_to_the_admitted_index() {
        let mut unpinned = MANIFEST.to_vec();
        unpinned.push(b'\n');
        assert_eq!(
            validate_appearance_items(&[], &BTreeMap::new(), INDEX, &unpinned),
            Err(ItemIdentityError::AliasTable(
                "current appearance manifest digest"
            ))
        );
    }
}
