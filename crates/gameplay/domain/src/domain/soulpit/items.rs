use super::{SoulpitError, validate_percent_roll};

mod catalog;
pub use catalog::{SOUL_CORE_SOURCE_IDS, is_soul_core_output, soul_core_name};

pub const SOUL_PRISM_SOURCE_ID: u32 = 49164;
pub const EXALTED_CORE_SOURCE_ID: u32 = 37110;
pub const OMINOUS_SOUL_CORE_SOURCE_ID: u32 = 49163;

/// Soulpit action registration excludes the Prism, although it remains a fusion output.
/// data-global/scripts/quests/soulpit/soulpit_fight.lua:160-164.
pub fn is_soul_core(source_id: u32) -> bool {
    source_id != SOUL_PRISM_SOURCE_ID && is_soul_core_output(source_id)
}

/// The item-use conversion additionally requires the literal donor name parser. Entry uses
/// a different gsub lookup, so malformed-name core49038 is deliberately not removed globally.
pub fn is_fusible_soul_core(source_id: u32) -> bool {
    is_soul_core(source_id) && soul_core_target_monster(source_id).is_some()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionKind {
    Fuse,
    Prism,
    Exalted,
}

/// Exact difficulty lookup, including numeric-key fallback to Lua nil -> C++ uint8 zero.
/// soul_prism.lua:58-67; exalted_core.lua:49-58; lua_functions_loader.hpp:73-88.
pub fn conversion_difficulty(kind: ConversionKind, current_stars: u8) -> Option<u8> {
    let adjacent = match kind {
        ConversionKind::Fuse => return None,
        ConversionKind::Prism => current_stars.checked_add(1),
        ConversionKind::Exalted => current_stars.checked_sub(1),
    };
    Some(
        adjacent
            .filter(|stars| (1..=6).contains(stars))
            .unwrap_or(0),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConversionFacts {
    pub kind: ConversionKind,
    pub source_is_soul_core: bool,
    pub target_is_soul_core: bool,
    pub target_monster_exists: bool,
    pub source_count: u32,
    pub target_count: u32,
    pub same_item_type: bool,
    pub same_instance: bool,
    /// The server draws from the complete donor candidate set, then resolves this Item.
    /// A missing core for the drawn creature refuses; it must not reroll another creature.
    pub selected_item_id: Option<u32>,
    pub selected_item_weight: u64,
    pub free_capacity: u64,
    pub ominous_roll: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConversionPlan {
    pub output_item_id: u32,
    pub consume_source: u32,
    pub consume_target: u32,
}

/// soulpit.lua:155-174, soul_prism.lua:41-102, exalted_core.lua:32-88.
/// Mint and consumption are one writer-owned transaction. Output placement follows addItem's
/// carried-then-ground behavior; lack of a carried slot is not a rule refusal here.
pub fn plan_conversion(facts: ConversionFacts) -> Result<ConversionPlan, SoulpitError> {
    if facts.source_count == 0 || facts.target_count == 0 {
        return Err(SoulpitError::NotEnoughItems);
    }
    if !facts.target_is_soul_core {
        return Err(SoulpitError::InvalidCore);
    }
    if facts.kind == ConversionKind::Fuse {
        if !facts.source_is_soul_core || (facts.same_instance && !facts.same_item_type) {
            return Err(SoulpitError::InvalidCore);
        }
        if facts.same_item_type && facts.source_count <= 1 {
            return Err(SoulpitError::NotEnoughItems);
        }
    } else {
        if facts.same_instance {
            return Err(SoulpitError::InvalidCore);
        }
        if !facts.target_monster_exists {
            return Err(SoulpitError::InvalidMonster);
        }
    }
    let mut output_item_id = facts.selected_item_id.ok_or(SoulpitError::NoCandidate)?;
    if !is_soul_core_output(output_item_id) {
        return Err(SoulpitError::InvalidCore);
    }
    if facts.kind != ConversionKind::Fuse && facts.free_capacity < facts.selected_item_weight {
        return Err(SoulpitError::InsufficientCapacity);
    }
    let mut consume_target = 1;
    if facts.kind == ConversionKind::Prism {
        validate_percent_roll(facts.ominous_roll)?;
        if facts.ominous_roll <= 2 {
            output_item_id = OMINOUS_SOUL_CORE_SOURCE_ID;
            consume_target = 0;
        }
    }
    Ok(ConversionPlan {
        output_item_id,
        consume_source: 1,
        consume_target,
    })
}

/// Lua `name:match("^(.-) soul core")`, used by Prism/Exalted without variation reversal.
pub fn soul_core_monster_name(name: &str) -> Option<&str> {
    name.split_once(" soul core").map(|(monster, _)| monster)
}

pub fn soul_core_target_monster(source_id: u32) -> Option<&'static str> {
    soul_core_name(source_id).and_then(soul_core_monster_name)
}

/// Prism/Exalted output lookup, soul_prism.lua:23-38 and soulpit.lua:7-18. This is a
/// source-name lookup followed by native Item admission, not a creature-profile core binding.
/// The two blue butterfly aliases deliberately produce the same item in this direction.
pub fn soul_core_for_monster(monster_name: &str) -> Option<u32> {
    let variation = match monster_name {
        "Horse" => Some("horse soul core (taupe)"),
        "Brown Horse" => Some("horse soul core (brown)"),
        "Grey Horse" => Some("horse soul core (gray)"),
        "Nomad" => Some("nomad soul core (basic)"),
        "Nomad Blue" => Some("nomad soul core (blue)"),
        "Nomad Female" => Some("nomad soul core (female)"),
        "Purple Butterfly" => Some("butterfly soul core (purple)"),
        "Butterfly" | "Blue Butterfly" => Some("butterfly soul core (blue)"),
        "Red Butterfly" => Some("butterfly soul core (red)"),
        _ => None,
    };
    SOUL_CORE_SOURCE_IDS.iter().copied().find(|id| {
        soul_core_name(*id).is_some_and(|name| match variation {
            Some(variation) => name.eq_ignore_ascii_case(variation),
            None => name
                .strip_suffix(" soul core")
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(monster_name)),
        })
    })
}

/// Fight entry reverses named variations, then removes every literal ` soul core` occurrence
/// (soulpit_fight.lua:86-88). PR2768 ANSWER6542983549 resolves the donor's ambiguous Lua pairs
/// reverse alias to canonical Blue Butterfly; every other mapping follows the source directly.
pub fn soulpit_entry_monster_name(source_id: u32) -> Option<String> {
    if !is_soul_core(source_id) {
        return None;
    }
    let name = soul_core_name(source_id)?;
    let variation = match name {
        "horse soul core (taupe)" => "Horse",
        "horse soul core (brown)" => "Brown Horse",
        "horse soul core (gray)" => "Grey Horse",
        "nomad soul core (basic)" => "Nomad",
        "nomad soul core (blue)" => "Nomad Blue",
        "nomad soul core (female)" => "Nomad Female",
        "butterfly soul core (purple)" => "Purple Butterfly",
        "butterfly soul core (red)" => "Red Butterfly",
        "butterfly soul core (blue)" => "Blue Butterfly",
        _ => return Some(name.replace(" soul core", "")),
    };
    Some(variation.to_owned())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DropFacts {
    pub fiendish: bool,
    pub recipient_can_receive_loot: bool,
    pub same_species_roll: u8,
    pub core_roll: u8,
    pub prism_roll: u8,
    pub same_species_core: Option<u32>,
    pub category_core: Option<u32>,
    pub prism_available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DropPlan {
    pub soul_core: Option<u32>,
    pub soul_prism: bool,
}

impl DropPlan {
    pub const NONE: Self = Self {
        soul_core: None,
        soul_prism: false,
    };
}

/// ondroploot_soul_core.lua:8-53: strict `<5` and `<4` are 4%/3% for rolls in1..=100.
/// Missing the selected core returns before the prism draw; same-species failure never rerolls.
pub fn plan_drop(facts: DropFacts) -> Result<DropPlan, SoulpitError> {
    if !facts.fiendish || !facts.recipient_can_receive_loot {
        return Ok(DropPlan::NONE);
    }
    validate_percent_roll(facts.same_species_roll)?;
    validate_percent_roll(facts.core_roll)?;
    let soul_core = if facts.core_roll < 5 {
        let selected = if facts.same_species_roll <= 15 {
            facts.same_species_core
        } else {
            facts.category_core
        };
        let Some(selected) = selected else {
            return Ok(DropPlan::NONE);
        };
        if !is_soul_core(selected) {
            return Err(SoulpitError::InvalidCore);
        }
        Some(selected)
    } else {
        None
    };
    validate_percent_roll(facts.prism_roll)?;
    Ok(DropPlan {
        soul_core,
        soul_prism: facts.prism_roll < 4 && facts.prism_available,
    })
}
