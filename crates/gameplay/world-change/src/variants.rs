//! Retained WVAR inputs from checkpoint 982d52b00. The reset owner commits one
//! selection per World and the Channel projects its exact recipe.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::OnceLock};

const RECIPES: &str = include_str!("variant-recipes.json");
const CAPACITY: usize = 16;
pub type Position = (u16, u16, i8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selected {
    choices: [Option<u8>; CAPACITY],
    digest: [u8; 32],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    slot: usize,
    pub key: String,
    chance_permille: u16,
    source: String,
    assumption: String,
    #[serde(default)]
    pub inactive_tiles: Vec<Tile>,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub key: String,
    weight: u16,
    pub tiles: Vec<Tile>,
    pub spawns: Vec<Spawn>,
    pub npcs: Vec<Npc>,
    pub portals: Vec<Portal>,
    #[serde(default)]
    pub barriers: Vec<Position>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tile {
    pub position: Position,
    pub flags: Option<u32>,
    pub entries: Vec<(u16, Option<u16>, Option<u16>)>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spawn {
    pub position: Position,
    pub creature: String,
    pub respawn_ms: u32,
    pub after_kills: Option<usize>,
    #[serde(rename = "multiplicity")]
    _multiplicity: SourceMultiplicity,
}

#[derive(Debug, Deserialize)]
enum SourceMultiplicity {
    #[serde(rename = "CHANNEL_LOCAL_REPEATABLE")]
    ChannelLocalRepeatable,
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
    pub exit: bool,
    pub premium: bool,
    pub promoted: bool,
    pub min_level: u32,
}

fn digest() -> [u8; 32] {
    static DIGEST: OnceLock<[u8; 32]> = OnceLock::new();
    *DIGEST.get_or_init(|| Sha256::digest(RECIPES).into())
}

pub fn definitions() -> Result<&'static [Definition], &'static str> {
    static DEFINITIONS: OnceLock<Result<Vec<Definition>, &'static str>> = OnceLock::new();
    DEFINITIONS
        .get_or_init(|| {
            let definitions: Vec<Definition> =
                serde_json::from_str(RECIPES).map_err(|_| "invalid daily variant recipes")?;
            let mut slots = BTreeSet::new();
            let mut keys = BTreeSet::new();
            for definition in &definitions {
                if definition.slot >= CAPACITY
                    || !slots.insert(definition.slot)
                    || !keys.insert(&definition.key)
                    || !crate::rules::valid_key(&definition.key)
                    || definition.source.is_empty()
                    || definition.assumption.is_empty()
                    || definition.chance_permille > 1000
                    || !(1..=16).contains(&definition.variants.len())
                    || !definition.inactive_tiles.iter().all(valid_tile)
                {
                    return Err("invalid daily variant definition");
                }
                let mut variant_keys = BTreeSet::new();
                for variant in &definition.variants {
                    let mut homes = BTreeSet::new();
                    if variant.weight == 0
                        || !variant_keys.insert(&variant.key)
                        || !variant.tiles.iter().all(valid_tile)
                        || !variant.barriers.iter().copied().all(valid_position)
                        || !variant.npcs.iter().all(|npc| valid_position(npc.position))
                        || !variant.portals.iter().all(|portal| {
                            valid_position(portal.position) && valid_position(portal.destination)
                        })
                        || !variant.spawns.iter().enumerate().all(|(index, spawn)| {
                            valid_position(spawn.position)
                                && homes.insert(spawn.position)
                                && spawn.creature.starts_with("oteryn:creature.")
                                && spawn.respawn_ms > 0
                                && spawn.after_kills.is_none_or(|count| {
                                    count > 0 && count == index && definition.key == "warpath"
                                })
                        })
                    {
                        return Err("invalid daily variant projection");
                    }
                }
            }
            Ok(definitions)
        })
        .as_deref()
        .map_err(|error| *error)
}

fn valid_position((_, _, floor): Position) -> bool {
    (0..=15).contains(&floor)
}

fn valid_tile(tile: &Tile) -> bool {
    valid_position(tile.position) && tile.entries.len() <= 64
}

impl Selected {
    pub fn roll(world: &[u8; 16], epoch: u64) -> Result<Self, &'static str> {
        Self::select(|purpose| crate::daily::draw_for_epoch(world, epoch, purpose))
    }

    pub fn select(mut draw: impl FnMut(&str) -> u64) -> Result<Self, &'static str> {
        let mut choices = [None; CAPACITY];
        for definition in definitions()? {
            let chance = draw(&format!("wvar:{}:chance", definition.key));
            if (u128::from(chance) * 1000) >> 64 >= u128::from(definition.chance_permille) {
                continue;
            }
            let total: u128 = definition
                .variants
                .iter()
                .map(|v| u128::from(v.weight))
                .sum();
            let draw = draw(&format!("wvar:{}:variant", definition.key));
            let mut pick = (u128::from(draw) * total) >> 64;
            for (index, variant) in definition.variants.iter().enumerate() {
                let weight = u128::from(variant.weight);
                if pick < weight {
                    choices[definition.slot] = u8::try_from(index).ok();
                    break;
                }
                pick -= weight;
            }
        }
        Ok(Self {
            choices,
            digest: digest(),
        })
    }

    pub fn validate(self) -> bool {
        if self == Self::default() {
            return true;
        }
        self.digest == digest()
            && definitions().is_ok_and(|defs| {
                self.choices.iter().enumerate().all(|(slot, choice)| {
                    choice.is_none_or(|index| {
                        defs.iter()
                            .any(|d| d.slot == slot && usize::from(index) < d.variants.len())
                    })
                })
            })
    }

    /// Historical days predating the mechanic retain their original projection.
    pub fn installed(self) -> bool {
        self.digest == digest()
    }

    pub fn variant(self, definition: &Definition) -> Option<&Variant> {
        if !self.installed() {
            return None;
        }
        definition
            .variants
            .get(usize::from(self.choices[definition.slot]?))
    }

    pub fn active(self) -> impl Iterator<Item = (&'static Definition, &'static Variant)> {
        definitions()
            .into_iter()
            .flatten()
            .filter_map(move |d| self.variant(d).map(|v| (d, v)))
    }

    pub fn owns_tile_entry(self, key: &str, position: Position, appearance: u16) -> bool {
        self.active().any(|(definition, variant)| {
            definition.key == key
                && variant.tiles.iter().any(|tile| {
                    tile.position == position
                        && tile.entries.iter().any(|entry| entry.0 == appearance)
                })
        })
    }

    pub fn passage(self, position: Position) -> Option<(&'static Definition, &'static Portal)> {
        if !self.installed() {
            return None;
        }
        self.active()
            .flat_map(|(definition, variant)| variant.portals.iter().map(move |p| (definition, p)))
            .find(|(_, portal)| portal.position == position)
            .or_else(|| {
                definitions().ok()?.iter().find_map(|definition| {
                    definition
                        .variants
                        .iter()
                        .flat_map(|v| &v.portals)
                        .find(|p| p.exit && p.position == position)
                        .map(|p| (definition, p))
                })
            })
    }
}
