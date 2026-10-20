// `crate::content::spell_data` slice for test binaries that path-load `src/combat.rs`
// directly: `combat/chain.rs` and
// `combat/formula.rs` re-export these two std-only modules. Nothing else is needed.

#[path = "../../src/content/spell_data/chain.rs"]
pub mod chain;
#[path = "../../src/content/spell_data/formula.rs"]
pub mod formula;
