//! Newhaven tutorial rules (TUTORIAL-1); no transport or persistence authority.
mod steps;
pub use steps::*;
mod corruptor_loot;
pub use corruptor_loot::corruptor_items;
#[cfg(test)]
mod corruptor_loot_tests;

#[cfg(test)]
mod steps_tests;
