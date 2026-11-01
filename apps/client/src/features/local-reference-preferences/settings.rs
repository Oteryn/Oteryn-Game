//! Local-only preferences whose keys come from the audited client reference surface.
//!
//! These values neither select a protocol capability nor manufacture server-owned panel data.

use crate::map::World;
use crate::play::PlayView;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

const PANEL_IDS: &[&str] = &[
    "battle",
    "chat",
    "guild",
    "hireling",
    "imbuement",
    "inventory",
    "market",
    "minimap",
    "npc-shop",
    "party",
    "prey",
    "quest-log",
    "stash",
    "task-board",
    "trade",
    "vip",
    "wheel",
];

fn default_scale() -> u16 {
    100
}

fn default_panels() -> Vec<String> {
    ["battle", "chat", "inventory", "minimap"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// Preferences owned entirely by the native client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferencePreferences {
    #[serde(default)]
    pub english: bool,
    /// Interface scale as an integral percentage.
    #[serde(default = "default_scale")]
    pub ui_scale: u16,
    #[serde(default = "default_panels")]
    pub visible_panels: Vec<String>,
}

impl Default for ReferencePreferences {
    fn default() -> Self {
        Self {
            english: false,
            ui_scale: default_scale(),
            visible_panels: default_panels(),
        }
    }
}

impl ReferencePreferences {
    fn validate(&self) -> Result<(), PreferenceError> {
        if !(80..=180).contains(&self.ui_scale)
            || self.visible_panels.len() > PANEL_IDS.len()
            || self.visible_panels.iter().enumerate().any(|(index, id)| {
                PANEL_IDS.binary_search(&id.as_str()).is_err()
                    || self.visible_panels[..index].contains(id)
            })
        {
            return Err(PreferenceError);
        }
        Ok(())
    }

    #[must_use]
    pub fn panel_visible(&self, id: &str) -> bool {
        self.visible_panels.iter().any(|visible| visible == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreferenceError;

impl Display for PreferenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid local client preferences")
    }
}

impl std::error::Error for PreferenceError {}

#[derive(Debug, Clone)]
pub(crate) struct LocalPlayState {
    world: Arc<World>,
    preferences: ReferencePreferences,
}

impl LocalPlayState {
    pub(crate) fn new(world: World) -> Self {
        Self {
            world: Arc::new(world),
            preferences: ReferencePreferences::default(),
        }
    }

    pub(crate) fn world(&self) -> Arc<World> {
        Arc::clone(&self.world)
    }

    pub(crate) fn set_world(&mut self, world: Arc<World>) {
        self.world = world;
    }
}

impl PlayView {
    #[must_use]
    pub const fn preferences(&self) -> &ReferencePreferences {
        &self.local.preferences
    }

    pub fn set_preferences(
        &mut self,
        preferences: ReferencePreferences,
    ) -> Result<(), PreferenceError> {
        preferences.validate()?;
        self.local.preferences = preferences;
        Ok(())
    }
}
