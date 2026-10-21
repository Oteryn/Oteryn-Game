//! Pure PvP legality, combat deadlines and bounded aggression relations.

pub mod legality;
pub mod lock;

pub use legality::*;
pub use lock::*;

#[cfg(test)]
mod tests;
