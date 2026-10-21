/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): a corpse container
/// holds at most this many entries; a loot entry's `placement_ordinal` is
/// `1..=` this.
pub const CORPSE_CONTAINER_ENTRIES_MAX: u32 = 16;

/// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2): a corpse container
/// holds at most 16 entries, equal by construction to
/// `COMBAT01-LOOT-PLAN-ITEMS`. Checked once, against the whole accepted plan,
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
}

/// Mirrors `content::reference_playable::ReferenceLootDefinition`: the
/// already-loaded, already-validated loot table this module plans from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootTableDefinition {
    pub algorithm: LootSelectionAlgorithm,
    pub entries: Vec<LootTableEntry>,
}
