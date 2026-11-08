//! Local HUD sidebars. General game miniwindows have their own independent owner.
use super::PlayView;
use super::feature_local_reference_preferences::settings::PreferenceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum HudPanel {
    Resources,
    Combat,
    Conditions,
    Monk,
}
impl HudPanel {
    pub const ALL: [Self; 4] = [Self::Resources, Self::Combat, Self::Conditions, Self::Monk];
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Resources => "Status",
            Self::Combat => "Combat",
            Self::Conditions => "Conditions and rest",
            Self::Monk => "Monk",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudDock {
    Left,
    Right,
    Top,
    Floating,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HudPane {
    pub dock: HudDock,
    pub bounds: [u16; 4],
    pub visible: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HudLayout {
    pub order: [HudPanel; 4],
    panes: [HudPane; 4],
}
impl Default for HudLayout {
    fn default() -> Self {
        Self {
            order: HudPanel::ALL,
            panes: [
                HudPane {
                    dock: HudDock::Top,
                    bounds: [8, 8, 900, 100],
                    visible: true,
                },
                HudPane {
                    dock: HudDock::Left,
                    bounds: [8, 112, 240, 120],
                    visible: false,
                },
                HudPane {
                    dock: HudDock::Left,
                    bounds: [8, 236, 240, 180],
                    visible: false,
                },
                HudPane {
                    dock: HudDock::Right,
                    bounds: [700, 112, 240, 120],
                    visible: false,
                },
            ],
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum HudAction {
    Visible(HudPanel, bool),
    Dock(HudPanel, HudDock),
    Earlier(HudPanel),
    Later(HudPanel),
    Dragged(HudPanel, [u16; 4]),
    Reset,
}
fn scaled(value: u16, scale: u16) -> u16 {
    u16::try_from(u32::from(value) * u32::from(scale) / 100).unwrap_or(u16::MAX)
}
impl HudLayout {
    #[must_use]
    pub fn pane(&self, panel: HudPanel) -> &HudPane {
        &self.panes[panel as usize]
    }
    #[must_use]
    pub fn valid(&self) -> bool {
        self.order
            .iter()
            .enumerate()
            .all(|(i, panel)| !self.order[..i].contains(panel))
            && self.panes.iter().all(|pane| {
                pane.bounds[0] <= 4096
                    && pane.bounds[1] <= 4096
                    && (160..=4096).contains(&pane.bounds[2])
                    && (64..=4096).contains(&pane.bounds[3])
            })
    }
    #[must_use]
    pub fn rect(&self, panel: HudPanel, root: [u16; 2], scale: u16) -> Option<[u16; 4]> {
        let pane = self.pane(panel);
        if !pane.visible {
            return None;
        }
        let [rw, rh] = root.map(|size| size.max(1));
        let width = scaled(pane.bounds[2], scale).min(rw.saturating_sub(16).max(1));
        let height = scaled(pane.bounds[3], scale).min(rh.saturating_sub(16).max(1));
        let before = |dock| {
            self.order
                .iter()
                .take_while(|id| **id != panel)
                .filter(|id| self.pane(**id).visible && self.pane(**id).dock == dock)
                .fold(0_u16, |sum, id| {
                    sum.saturating_add(scaled(self.pane(*id).bounds[3], scale))
                        .saturating_add(4)
                })
        };
        let top = self
            .order
            .iter()
            .filter(|id| self.pane(**id).visible && self.pane(**id).dock == HudDock::Top)
            .fold(0_u16, |sum, id| {
                sum.saturating_add(scaled(self.pane(*id).bounds[3], scale))
                    .saturating_add(4)
            });
        let (x, y) = match pane.dock {
            HudDock::Top => (8, 8_u16.saturating_add(before(HudDock::Top))),
            HudDock::Left => (
                8,
                8_u16
                    .saturating_add(top)
                    .saturating_add(before(HudDock::Left)),
            ),
            HudDock::Right => (
                rw.saturating_sub(width).saturating_sub(8),
                8_u16
                    .saturating_add(top)
                    .saturating_add(before(HudDock::Right)),
            ),
            HudDock::Floating => (scaled(pane.bounds[0], scale), scaled(pane.bounds[1], scale)),
        };
        Some([
            x.min(rw.saturating_sub(width)),
            y.min(rh.saturating_sub(height)),
            width,
            height,
        ])
    }
    pub(super) fn legacy(&mut self, bounds: [u16; 4]) {
        self.panes[HudPanel::Resources as usize].dock = HudDock::Floating;
        self.panes[HudPanel::Resources as usize].bounds =
            [bounds[0], bounds[1], bounds[2].max(160), bounds[3].max(64)];
    }
    pub fn apply(&mut self, action: HudAction, root: [u16; 2], scale: u16) -> bool {
        let previous = self.clone();
        match action {
            HudAction::Visible(panel, visible) => self.panes[panel as usize].visible = visible,
            HudAction::Dock(panel, dock) => self.panes[panel as usize].dock = dock,
            HudAction::Reset => *self = Self::default(),
            HudAction::Earlier(panel) | HudAction::Later(panel) => {
                let index = self.order.iter().position(|id| *id == panel).unwrap_or(0);
                let candidates: Vec<_> = self
                    .order
                    .iter()
                    .enumerate()
                    .filter(|(_, id)| {
                        self.pane(**id).visible && self.pane(**id).dock == self.pane(panel).dock
                    })
                    .map(|(index, _)| index)
                    .collect();
                let next = if matches!(action, HudAction::Earlier(_)) {
                    candidates.into_iter().rfind(|i| *i < index)
                } else {
                    candidates.into_iter().find(|i| *i > index)
                };
                if let Some(next) = next {
                    self.order.swap(index, next);
                }
            }
            HudAction::Dragged(panel, bounds) => {
                let logical = bounds.map(|value| {
                    u16::try_from(u32::from(value) * 100 / u32::from(scale.max(1)))
                        .unwrap_or(4096)
                        .min(4096)
                });
                let pane = &mut self.panes[panel as usize];
                pane.bounds = [
                    logical[0],
                    logical[1],
                    logical[2].max(160),
                    logical[3].max(64),
                ];
                pane.dock = if bounds[0] <= 24 {
                    HudDock::Left
                } else if bounds[0].saturating_add(bounds[2]) >= root[0].saturating_sub(24) {
                    HudDock::Right
                } else if bounds[1] <= 24 {
                    HudDock::Top
                } else {
                    HudDock::Floating
                };
                let dock = pane.dock;
                if dock != HudDock::Floating {
                    let before = self.order.iter().copied().find(|id| {
                        *id != panel
                            && self.pane(*id).visible
                            && self.pane(*id).dock == dock
                            && self
                                .rect(*id, root, scale)
                                .is_some_and(|rect| bounds[1] < rect[1].saturating_add(rect[3] / 2))
                    });
                    if let Some(before) = before {
                        let mut order: Vec<_> =
                            self.order.into_iter().filter(|id| *id != panel).collect();
                        if let Some(index) = order.iter().position(|id| *id == before) {
                            order.insert(index, panel);
                        }
                        if let Ok(order) = order.try_into() {
                            self.order = order;
                        }
                    }
                }
            }
        }
        *self != previous
    }
}
impl PlayView {
    pub fn hud_action(&mut self, action: HudAction, root: [u16; 2]) -> Result<(), PreferenceError> {
        let mut preferences = self.preferences().clone();
        preferences.hud_layout = preferences.effective_hud_layout();
        preferences
            .hud_layout
            .apply(action, root, preferences.interface_scale);
        if matches!(action, HudAction::Reset) {
            preferences.hud_bounds = super::ReferencePreferences::default().hud_bounds;
        }
        self.set_preferences(preferences)
    }
}
