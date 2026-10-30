//! Immutable loot table shapes and corpse entry limits.

/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): a corpse container
/// holds at most this many entries; a loot entry's `placement_ordinal` is
/// `1..=` this.
pub const CORPSE_CONTAINER_ENTRIES_MAX: u32 = 30;

/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): a corpse container
/// holds at most 30 entries (SUMMER owner decision 20). The independent
/// `COMBAT01-LOOT-PLAN-ITEMS` ceiling still bounds generated loot plans. Checked against the whole plan,
/// before the corpse or any entry is frozen.
pub const GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX: usize = CORPSE_CONTAINER_ENTRIES_MAX as usize;

/// This module's own minimal, `Clone`/`PartialEq` typed-identity shape:
/// field-compatible with both `content::reference_playable::TypedDefinitionRef`
/// (`family`/`key`/`revision`) and `durability::item_mint::TypedDefinitionRef`
/// (`family`/`production_key`/`revision_ref`, all `String`) without importing
/// either (see module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootDefinitionRef {
    pub family: String,
    pub production_key: String,
    pub revision_ref: String,
}

impl LootDefinitionRef {
    pub fn new(
        family: impl Into<String>,
        production_key: impl Into<String>,
        revision_ref: impl Into<String>,
    ) -> Self {
        Self {
            family: family.into(),
            production_key: production_key.into(),
            revision_ref: revision_ref.into(),
        }
    }
}

/// Mirrors `content::reference_playable::ReferenceLootSelectionAlgorithm`
/// (same variant names); only `IndependentBernoulliPpm` is implemented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootSelectionAlgorithm {
    IndependentBernoulliPpm,
    /// Daily boost: up to three additional independent rolls of the original entries.
    /// Appended Forge drops are outside `boosted_entries` and roll once.
    DailyBoosted {
        extra_rolls: u8,
        boosted_entries: usize,
    },
    WeightedSingleSelection,
    GuaranteedEntries,
    NestedGroups,
}

/// Mirrors `content::reference_playable::ReferenceLootEntry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootTableEntry {
    pub item: LootDefinitionRef,
    pub min_count: u32,
    pub max_count: u32,
    pub probability_ppm: Option<u32>,
    pub details: LootEntryDetails,
}

/// Per-entry container and item state carried by the admitted loot table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LootEntryDetails {
    pub children: Vec<LootTableEntry>,
    pub container_capacity: Option<u32>,
    /// Full native charges; a fresh non-stackable item starts at its definition default.
    pub charges: Option<u32>,
    /// Stackable charged items use the count as their stack quantity.
    pub stackable: bool,
    pub subtype: Option<u16>,
    pub unique: bool,
}

/// Mirrors `content::reference_playable::ReferenceLootDefinition`: the
/// already-loaded, already-validated loot table this module plans from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootTableDefinition {
    pub algorithm: LootSelectionAlgorithm,
    pub entries: Vec<LootTableEntry>,
}
