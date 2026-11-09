//! Local list projections retain the entity identity and generation supplied by the server.
use oteryn_session::{EntityDetail, EntityKind, WorldSpatialEntity};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Orders local settings edits across the shell and battle threads.
pub(crate) fn next_edit() -> u64 {
    static EDIT: AtomicU64 = AtomicU64::new(1);
    EDIT.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattleSort {
    #[default]
    Name,
    HealthAscending,
    HealthDescending,
    Identity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BattleListPreferences {
    pub name: String,
    pub players: bool,
    pub creatures: bool,
    pub search: String,
    pub max_health: u8,
    pub sort: BattleSort,
}

impl Default for BattleListPreferences {
    fn default() -> Self {
        Self {
            name: tr!("Nearby").into(),
            players: true,
            creatures: true,
            search: String::new(),
            max_health: 100,
            sort: BattleSort::Name,
        }
    }
}

pub fn default_lists() -> Vec<BattleListPreferences> {
    vec![BattleListPreferences::default()]
}

pub fn validate_lists(lists: &[BattleListPreferences]) -> bool {
    (1..=4).contains(&lists.len())
        && lists.iter().enumerate().all(|(i, list)| {
            !list.name.trim().is_empty()
                && list.name.len() <= 32
                && !list.name.chars().any(char::is_control)
                && list.search.len() <= 64
                && !list.search.chars().any(char::is_control)
                && list.max_health <= 100
                && !lists[..i]
                    .iter()
                    .any(|other| other.name.eq_ignore_ascii_case(&list.name))
        })
}

pub fn health(row: &WorldSpatialEntity) -> u8 {
    match row.detail {
        EntityDetail::Actor { health_percent, .. } => health_percent,
        _ => 0,
    }
}

pub fn name(row: &WorldSpatialEntity) -> &str {
    row.display_name
        .as_ref()
        .and_then(|n| n.as_str().ok())
        .unwrap_or("")
}

#[cfg(windows)]
pub(crate) fn row_label(row: &WorldSpatialEntity) -> String {
    let title = if name(row).is_empty() {
        format!("{:?}", row.kind)
    } else {
        name(row).to_owned()
    };
    format!(
        "{title} — {}% ({}, {})",
        health(row),
        row.position.x,
        row.position.y
    )
}

impl BattleListPreferences {
    pub fn rows(&self, entities: &[WorldSpatialEntity]) -> Vec<WorldSpatialEntity> {
        let search = self.search.to_lowercase();
        let mut rows: Vec<_> = entities
            .iter()
            .filter(|row| {
                ((row.kind == EntityKind::Player && self.players)
                    || (row.kind == EntityKind::Creature && self.creatures))
                    && health(row) <= self.max_health
                    && (search.is_empty() || name(row).to_lowercase().contains(&search))
            })
            .copied()
            .collect();
        rows.sort_by(|a, b| {
            let order = match self.sort {
                BattleSort::Name => name(a).to_lowercase().cmp(&name(b).to_lowercase()),
                BattleSort::HealthAscending => health(a).cmp(&health(b)),
                BattleSort::HealthDescending => health(b).cmp(&health(a)),
                BattleSort::Identity => a.entity.cmp(&b.entity),
            };
            order.then_with(|| a.entity.cmp(&b.entity))
        });
        rows
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleListsState {
    pub preferences: Vec<BattleListPreferences>,
    pub dirty: bool,
    pub edit: u64,
    pub active: usize,
}

impl Default for BattleListsState {
    fn default() -> Self {
        Self {
            preferences: default_lists(),
            dirty: false,
            edit: 0,
            active: 0,
        }
    }
}
