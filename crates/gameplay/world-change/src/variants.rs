//! WVAR: bounded content recipes selected only by the existing World/reset owner.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::OnceLock};

pub const CAPACITY: usize = 16;
pub type Position = (u16, u16, i8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selected(pub [Option<u8>; CAPACITY], pub [u8; 32]);

const RECIPES: &str = include_str!("../../../../content/encounters/world-changes/variants.json");

pub fn recipe_digest() -> [u8; 32] {
    static DIGEST: OnceLock<[u8; 32]> = OnceLock::new();
    *DIGEST.get_or_init(|| {
        Sha256::new()
            .chain_update(b"oteryn:wvar:recipes:v1\0")
            .chain_update(RECIPES.as_bytes())
            .chain_update(include_bytes!(
                "../../../../content/encounters/world-changes/layers.json"
            ))
            .finalize()
            .into()
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    /// Stable append-only slot in immutable stored decisions.
    pub slot: usize,
    pub key: String,
    pub chance_permille: u16,
    pub source: String,
    pub assumption: Option<String>,
    #[serde(default)]
    pub inactive_tiles: Vec<Tile>,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub key: String,
    pub weight: u16,
    pub layers: Vec<String>,
    pub tiles: Vec<Tile>,
    pub tags: Vec<Tag>,
    pub spawns: Vec<Spawn>,
    pub npcs: Vec<Npc>,
    pub portals: Vec<Portal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tile {
    pub position: Position,
    pub flags: Option<u32>,
    pub entries: Vec<(u16, Option<u16>, Option<u16>)>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub position: Position,
    pub appearance: u16,
    pub action: Option<u16>,
    pub unique: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spawn {
    pub position: Position,
    pub creature: String,
    pub respawn_ms: u32,
    #[serde(default)]
    pub after_kills: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub position: Position,
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Portal {
    pub position: Position,
    pub destination: Position,
    pub appearance: u16,
    pub action: Option<u16>,
    pub unique: Option<u16>,
    pub premium: bool,
    pub promoted: bool,
    pub min_level: u32,
}

pub fn validate(definitions: &[Definition]) -> Result<(), &'static str> {
    if definitions.len() > CAPACITY {
        return Err("too many world variants");
    }
    let mut slots = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for d in definitions {
        if d.slot >= CAPACITY
            || !slots.insert(d.slot)
            || !keys.insert(&d.key)
            || d.key.is_empty()
            || d.key.len() > 64
            || d.source.is_empty()
            || d.chance_permille > 1000
            || d.variants.is_empty()
            || d.variants.len() > 16
        {
            return Err("invalid world variant definition");
        }
        let mut variants = BTreeSet::new();
        for v in &d.variants {
            if v.key.is_empty()
                || !variants.insert(&v.key)
                || v.weight == 0
                || v.layers.len() > 16
                || v.tiles.len() > 65_536
                || v.tags.len() > 1024
                || v.spawns.len() > 4096
                || v.npcs.len() > 64
                || v.portals.len() > 64
            {
                return Err("invalid world variant recipe");
            }
            let positions = v
                .tiles
                .iter()
                .map(|t| t.position)
                .chain(v.tags.iter().map(|t| t.position))
                .chain(v.spawns.iter().map(|s| s.position))
                .chain(v.npcs.iter().map(|n| n.position))
                .chain(v.portals.iter().flat_map(|p| [p.position, p.destination]));
            if positions
                .into_iter()
                .any(|(_, _, z)| !(0..=15).contains(&z))
                || v.spawns
                    .iter()
                    .any(|s| s.respawn_ms == 0 || !s.creature.starts_with("oteryn:creature."))
                || v.tiles.iter().any(|t| t.entries.len() > 64)
            {
                return Err("invalid world variant placement");
            }
        }
    }
    Ok(())
}

pub fn definitions() -> Result<&'static [Definition], &'static str> {
    static DEFINITIONS: OnceLock<Result<Vec<Definition>, &'static str>> = OnceLock::new();
    match DEFINITIONS.get_or_init(|| {
        let definitions: Vec<Definition> =
            serde_json::from_str(RECIPES).map_err(|_| "invalid world variant JSON")?;
        validate(&definitions)?;
        Ok(definitions)
    }) {
        Ok(definitions) => Ok(definitions),
        Err(error) => Err(*error),
    }
}

impl Selected {
    pub fn compatible(self) -> bool {
        self.0.iter().all(Option::is_none) || self.1 == recipe_digest()
    }

    pub fn variant(self, key: &str) -> Option<usize> {
        if !self.compatible() {
            return None;
        }
        let d = definitions().ok()?.iter().find(|d| d.key == key)?;
        self.0[d.slot]
            .map(usize::from)
            .filter(|i| *i < d.variants.len())
    }

    pub fn active(self) -> impl Iterator<Item = &'static Variant> {
        definitions().into_iter().flatten().filter_map(move |d| {
            if !self.compatible() {
                return None;
            }
            d.variants.get(usize::from(self.0[d.slot]?))
        })
    }

    pub fn select(mut draw: impl FnMut(&str) -> u64) -> Self {
        let mut selected = Self([None; CAPACITY], recipe_digest());
        for d in definitions().into_iter().flatten() {
            if (u128::from(draw(&format!("wvar:{}:chance", d.key))) * 1000) >> 64
                >= u128::from(d.chance_permille)
            {
                continue;
            }
            let total: u64 = d.variants.iter().map(|v| u64::from(v.weight)).sum();
            let mut pick = ((u128::from(draw(&format!("wvar:{}:variant", d.key)))
                * u128::from(total))
                >> 64) as u64;
            for (i, v) in d.variants.iter().enumerate() {
                if pick < u64::from(v.weight) {
                    selected.0[d.slot] = u8::try_from(i).ok();
                    break;
                }
                pick -= u64::from(v.weight);
            }
        }
        selected
    }
}
