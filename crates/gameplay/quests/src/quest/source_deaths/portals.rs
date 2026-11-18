//! A permanent escape portal from a qualifying physical death, independent of credit.
use super::super::world_callbacks::Position;
use serde::Deserialize;
use std::{collections::BTreeSet, sync::OnceLock};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathPortal {
    pub creature: String,
    pub death_minimum: Position,
    pub death_maximum: Position,
    pub position: Position,
    pub destination: Position,
    pub appearance: u16,
    pub effect: u16,
}

impl DeathPortal {
    fn valid(&self) -> bool {
        self.creature.starts_with("oteryn:creature.")
            && super::super::valid_quest_key(&self.creature)
            && self.appearance != 0
            && self.effect != 0
            && self.death_minimum.floor == self.death_maximum.floor
            && self.death_minimum.x <= self.death_maximum.x
            && self.death_minimum.y <= self.death_maximum.y
            && self.admits(&self.creature, self.position)
            && (-15..=0).contains(&self.position.floor)
            && (-15..=0).contains(&self.destination.floor)
            && self.position != self.destination
    }

    pub fn admits(&self, creature: &str, position: Position) -> bool {
        self.creature == creature
            && self.death_minimum.floor == position.floor
            && (self.death_minimum.x..=self.death_maximum.x).contains(&position.x)
            && (self.death_minimum.y..=self.death_maximum.y).contains(&position.y)
    }
}

pub fn death_portals() -> Result<&'static [DeathPortal], &'static str> {
    static RULES: OnceLock<Result<Vec<DeathPortal>, String>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let mut at = BTreeSet::new();
            let mut result = Vec::new();
            for portal in super::documents()?
                .into_iter()
                .flat_map(|doc| doc.escape_portals)
            {
                if result.len() >= 32 || !portal.valid() || !at.insert(portal.position) {
                    return Err("invalid/duplicate source death escape portal".into());
                }
                result.push(portal);
            }
            Ok(result)
        })
        .as_deref()
        .map_err(String::as_str)
}
