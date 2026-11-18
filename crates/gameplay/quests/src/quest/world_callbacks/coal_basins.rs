//! Ancient Tombs item arrivals from Crystal 201189f8,
//! movements_all_teleports_tombs_coal_basin.lua:1-25,30-55,61-63.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{Document, NativeSelector, Position};

pub const COIN: &str = "oteryn:item.tibia.i3042";
pub const QUEST: &str = "oteryn:quest.the_ancient_tombs_quest";
pub const START: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/default_start";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Treasure {
    Dipthrah,
    Mahrdis,
}

impl Treasure {
    pub const fn track(self) -> &'static str {
        match self {
            Self::Dipthrah => {
                "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/diphtrahs_treasure"
            }
            Self::Mahrdis => "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/mahrdis_treasure",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CoalBasin {
    /// Legacy source identity. Dispatch selects the exact authored native placement.
    pub source_uid: u16,
    pub basin: NativeSelector,
    pub flame: Position,
    pub destination: Position,
    #[serde(default)]
    pub treasure: Option<Treasure>,
}

pub(super) fn validate(bindings: &[CoalBasin]) -> Result<(), String> {
    let mut positions = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for binding in bindings {
        let basin = binding.basin.position;
        let (flame, destination) = (binding.flame, binding.destination);
        let expected_treasure = match binding.source_uid {
            9035 => Some(Treasure::Dipthrah),
            9036 => Some(Treasure::Mahrdis),
            _ => None,
        };
        if !(9033..=9040).contains(&binding.source_uid)
            || !ids.insert(binding.source_uid)
            || !positions.insert(basin)
            || binding.basin.appearance != 2114
            || binding.basin.ordinal == 0
            || !(-15..=0).contains(&basin.floor)
            || basin.floor != flame.floor
            || basin.floor != destination.floor
            || basin == flame
            || basin == destination
            || flame == destination
            || basin.x.abs_diff(flame.x) > 1
            || basin.y.abs_diff(flame.y) > 1
            || binding.treasure != expected_treasure
        {
            return Err(format!(
                "invalid Ancient Tombs coal basin {}",
                binding.source_uid
            ));
        }
    }
    Ok(())
}

pub fn load(document: &str) -> Result<Vec<CoalBasin>, String> {
    let document: Document = serde_json::from_str(document).map_err(|error| error.to_string())?;
    if document.schema != 1 {
        return Err("coal basin document schema".into());
    }
    validate(&document.coal_basins)?;
    Ok(document.coal_basins)
}
