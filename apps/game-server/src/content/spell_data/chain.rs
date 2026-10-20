//! Authored chain-targeting data.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChainShape {
    /// Each further creature is searched from the last creature hit.
    Sequential,
    /// Every further creature is searched from the first creature and hit at step 1.
    Fork,
}

/// An admitted `Ability.chain` of a player spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainSpec {
    /// Further creatures after the first: one cast hits at most `1 + max_targets`.
    pub(crate) max_targets: u32,
    pub(crate) range_tiles: u32,
    /// Search radius for the first creature of a cast without a target (absent: `range_tiles`).
    pub(crate) initial_range_tiles: u32,
    pub(crate) shape: ChainShape,
    pub(crate) damage_step_percent: i32,
    pub(crate) asset_binding: Option<String>,
}
