//! Bestiary and Charms client feature, split by concern.

pub use oteryn_client_runtime::cyclopedia_protocol::{
    BestiaryRaceProgress, CharmAssignDisposition, CharmAssignIntent, CharmKind, CharmState,
    CharmUnlockDisposition, CharmUnlockStageIntent, CharmView,
};

mod protocol;
mod settings;
mod state;
mod view;

pub use protocol::*;
pub use settings::*;
pub use state::*;
pub use view::*;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
