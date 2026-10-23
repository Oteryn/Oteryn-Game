//! L3 rules: experience, skills and training, charms, bestiary, achievements, wheel of destiny, proficiency.

pub mod account_achievement;
pub mod achievement_catalogue;
pub mod bestiary;
pub mod charm_effects;
pub mod daily_reward;
pub mod hit_training;
pub mod mana_training;
pub mod prey;
pub mod proficiency;
pub mod proficiency_modification;
pub mod promotion;
pub mod ranking;
pub mod regeneration;
pub mod spell_wheel;
pub mod wheel;
pub mod wheel_gem_data;
pub mod wheel_read;

mod revision;

#[cfg(test)]
mod experience_property_tests;

#[cfg(test)]
mod mana_training_property_tests;
