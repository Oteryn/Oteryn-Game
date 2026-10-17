//! BED-1 house bed sleeper data. Raw stored values only; no bed USE, wake,
//! settlement or regeneration runtime is wired here yet.

// The private storage leaf needs `content`, `house_scope` and `house_bed`, so
// it stays outside standalone `durability/mod.rs` path loads in tests/.
#[path = "../durability/bed_sleeper.rs"]
pub mod bed_sleeper;
pub mod sleeper;
