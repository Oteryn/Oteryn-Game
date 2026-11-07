//! WVAR: bounded content recipes selected only by the existing World/reset owner.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::OnceLock};

pub const CAPACITY: usize = 16;
pub type Position = (u16, u16, i8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selected(pub [Option<u8>; CAPACITY], pub [u8; 32]);

pub const RECIPES: &str =
    include_str!("../../../../content/encounters/world-changes/variants.json");
pub const LAYERS: &str = include_str!("../../../../content/encounters/world-changes/layers.json");

pub fn recipe_digest() -> [u8; 32] {
    static DIGEST: OnceLock<[u8; 32]> = OnceLock::new();
    *DIGEST.get_or_init(|| {
        Sha256::new()
            .chain_update(b"oteryn:wvar:recipes:v1\0")
            .chain_update(RECIPES.as_bytes())
            .chain_update(LAYERS.as_bytes())
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
    #[serde(default)]
    pub reset_tally: Option<ResetTally>,
    #[serde(default)]
    pub stations: Vec<UseStation>,
    #[serde(default)]
    pub entry_limit: Option<EntryLimit>,
    #[serde(default)]
    pub entry_cooldown: Option<String>,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryLimit {
    pub track: String,
    pub maximum: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UseStation {
    pub position: Position,
    pub appearance: u16,
    pub material: String,
    pub resource: crate::owner::ContributionKind,
    pub after: Option<u16>,
    pub yield_item: Option<u16>,
}

impl UseStation {
    pub fn matches(&self, position: Position, appearance: u16, material: &str) -> bool {
        self.position == position && self.appearance == appearance && self.material == material
    }
}

/// The existing bundle placement format, including its exact 32-byte bundle prefix.
pub fn station_position(placement: &[u8]) -> Option<Position> {
    let placement: &[u8; 40] = placement.try_into().ok()?;
    let key = u64::from_be_bytes(placement[32..].try_into().ok()?);
    let depth = (key >> 8) & 255;
    if key >> 48 != 0 || depth > 15 || key & 255 > 63 {
        return None;
    }
    Some((
        u16::try_from((key >> 32) & 65535).ok()?,
        u16::try_from((key >> 16) & 65535).ok()?,
        i8::try_from(depth).ok()?,
    ))
}

pub fn station_at(
    position: Position,
    appearance: u16,
    material: &str,
) -> Option<(&'static Definition, &'static UseStation)> {
    definitions().ok()?.iter().find_map(|definition| {
        definition
            .stations
            .iter()
            .find(|station| station.matches(position, appearance, material))
            .map(|station| (definition, station))
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetTally {
    pub resource: crate::owner::ContributionKind,
    pub threshold: u64,
    pub at_or_above: u8,
    pub below: u8,
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
    #[serde(default)]
    pub barriers: Vec<Position>,
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
    pub multiplicity: SourceMultiplicity,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceMultiplicity {
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
    /// Exit remains available after closure; source-order fallback is a safe surface return.
    #[serde(default)]
    pub exit: bool,
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
    let mut stations = BTreeSet::new();
    for d in definitions {
        if d.slot >= CAPACITY
            || !slots.insert(d.slot)
            || !keys.insert(&d.key)
            || d.key.is_empty()
            || d.key.len() > 64
            || d.source.is_empty()
            || d.inactive_tiles.len() > 65_536
            || d.inactive_tiles
                .iter()
                .any(|t| !(0..=15).contains(&t.position.2) || t.entries.len() > 64)
            || d.chance_permille > 1000
            || d.entry_limit.as_ref().is_some_and(|limit| {
                limit.maximum == 0
                    || limit.track.len() > 512
                    || !limit.track.starts_with("oteryn:quest-progress/")
            })
            || d.entry_cooldown
                .as_ref()
                .is_some_and(|key| key != &d.key || key != "warpath" || key.len() > 64)
            || d.stations.len() > 1024
            || d.variants.is_empty()
            || d.variants.len() > 16
        {
            return Err("invalid world variant definition");
        }
        for station in &d.stations {
            if !(0..=15).contains(&station.position.2)
                || station.appearance == 0
                || !station.material.starts_with("oteryn:item.")
                || station.material.len() > 512
                || !matches!(
                    station.resource,
                    crate::owner::ContributionKind::AwashCoal
                        | crate::owner::ContributionKind::ThornfireExtinguished
                )
                || station.after == Some(0)
                || station.yield_item == Some(0)
                || !stations.insert((
                    station.position,
                    station.appearance,
                    station.material.as_str(),
                ))
            {
                return Err("invalid world change station");
            }
        }
        if d.reset_tally.as_ref().is_some_and(|rule| {
            rule.threshold == 0
                || usize::from(rule.at_or_above) >= d.variants.len()
                || usize::from(rule.below) >= d.variants.len()
                || rule.at_or_above == rule.below
        }) {
            return Err("invalid reset tally rule");
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
                || v.barriers.len() > 256
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
                .chain(v.barriers.iter().copied())
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

    /// Current selected routes win. Closed changes retain only their explicit exits.
    /// Shared exits use the first authored safe surface destination after closure.
    pub fn passage(self, position: Position) -> Option<&'static Portal> {
        if !self.compatible() {
            return None;
        }
        self.active()
            .flat_map(|v| &v.portals)
            .find(|p| p.position == position)
            .or_else(|| {
                definitions()
                    .ok()?
                    .iter()
                    .flat_map(|d| &d.variants)
                    .flat_map(|v| &v.portals)
                    .find(|p| p.exit && p.position == position)
            })
    }

    pub fn portals(self) -> impl Iterator<Item = &'static Portal> {
        let mut seen = BTreeSet::new();
        let mut routes = Vec::new();
        if self.compatible() {
            for portal in self.active().flat_map(|v| &v.portals).chain(
                definitions()
                    .into_iter()
                    .flatten()
                    .flat_map(|d| &d.variants)
                    .flat_map(|v| &v.portals)
                    .filter(|p| p.exit),
            ) {
                if seen.insert(portal.position) {
                    routes.push(portal);
                }
            }
        }
        routes.into_iter()
    }

    pub fn select(mut draw: impl FnMut(&str) -> u64) -> Self {
        let mut selected = Self([None; CAPACITY], recipe_digest());
        for d in definitions().into_iter().flatten() {
            if let Some(rule) = &d.reset_tally {
                selected.0[d.slot] = Some(rule.below);
                continue;
            }
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

#[cfg(test)]
mod station_tests {
    use super::*;

    #[test]
    fn station_position_requires_exact_bundle_placement_and_source_depth() {
        let mut placement = vec![3; 32];
        let key = (32558_u64 << 32) | (32007_u64 << 16) | (9 << 8) | 1;
        placement.extend_from_slice(&key.to_be_bytes());
        assert_eq!(station_position(&placement), Some((32558, 32007, 9)));
        assert_eq!(station_position(&placement[..39]), None);
        for invalid in [
            key | (1 << 48),
            (key & !0xff00) | (16 << 8),
            (key & !255) | 64,
        ] {
            placement[32..].copy_from_slice(&invalid.to_be_bytes());
            assert_eq!(station_position(&placement), None);
        }
    }

    #[test]
    fn source_station_requires_position_appearance_and_material_together() {
        let station = UseStation {
            position: (32558, 32007, 9),
            appearance: 12666,
            material: "oteryn:item.tibia.i12600".into(),
            resource: crate::owner::ContributionKind::AwashCoal,
            after: Some(12666),
            yield_item: None,
        };
        assert!(station.matches((32558, 32007, 9), 12666, "oteryn:item.tibia.i12600"));
        assert!(!station.matches((32559, 32007, 9), 12666, "oteryn:item.tibia.i12600"));
        assert!(!station.matches((32558, 32007, 9), 12667, "oteryn:item.tibia.i12600"));
        assert!(!station.matches((32558, 32007, 9), 12666, "oteryn:item.tibia.i3492"));
    }

    #[test]
    fn every_value_source_requires_explicit_repeatability() {
        let base = serde_json::json!({"position":[32558,32007,9],"creature":"oteryn:creature.rat","respawn_ms":90000});
        assert!(serde_json::from_value::<Spawn>(base.clone()).is_err());
        let mut admitted = base;
        admitted["multiplicity"] = serde_json::json!("CHANNEL_LOCAL_REPEATABLE");
        assert!(serde_json::from_value::<Spawn>(admitted).is_ok());
    }
}
