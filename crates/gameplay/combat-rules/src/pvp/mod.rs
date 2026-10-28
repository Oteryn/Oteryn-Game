//! Pure PvP legality, combat deadlines and bounded aggression relations.

pub mod death;
pub mod legality;
pub mod lock;
pub mod skull;

pub use legality::*;
pub use lock::*;

#[cfg(test)]
mod death_tests;
#[cfg(test)]
mod skull_tests;
#[cfg(test)]
mod tests;
