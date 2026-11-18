//! Bounded map-owned color puzzle bindings for STEP switches and a solve/reset exit.
use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GridAxis {
    X,
    Y,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GridGroup {
    pub cells: Vec<Position>,
    /// A group is uniform in any one of these colors. The first is used in the reset pool.
    pub colors: Vec<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ColorGrid {
    Shift {
        axis: GridAxis,
        offset: u8,
        length: u8,
        colors: Vec<u16>,
    },
    Reset {
        groups: Vec<GridGroup>,
    },
}

fn valid_colors(colors: &[u16]) -> bool {
    (1..=8).contains(&colors.len())
        && !colors.contains(&0)
        && colors.iter().copied().collect::<BTreeSet<_>>().len() == colors.len()
}

impl ColorGrid {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        if callback.trigger != Trigger::StepIn
            || callback.actor_kind != ActorKind::Player
            || callback.has_personal_state()
            || callback.lever.is_some()
            || !callback.passages.is_empty()
            || !callback.offerings.is_empty()
            || callback.item_on_target.is_some()
            || callback.remote_teleport.is_some()
            || !callback.world_requirements.is_empty()
            || callback.effects
                != CallbackEffectSet::from_shared(CallbackEffects {
                    color_grid: Some(self.clone()),
                    transform: callback.effects.transform,
                    teleport_effect: callback.effects.teleport_effect,
                    ..CallbackEffects::default()
                })
        {
            return false;
        }
        match self {
            Self::Shift {
                offset,
                length,
                colors,
                ..
            } => {
                (callback.selector.action.is_some() || callback.selector.native.is_some())
                    && callback.teleport.is_none()
                    && !callback.return_on_failure
                    && *offset > 0
                    && (2..=8).contains(length)
                    && valid_colors(colors)
            }
            Self::Reset { groups } => {
                callback.teleport.is_some()
                    && callback.return_on_failure
                    && callback.effects.transform.is_none()
                    && (1..=8).contains(&groups.len())
                    && groups.iter().all(|group| {
                        !group.cells.is_empty()
                            && valid_colors(&group.colors)
                            && group.cells.iter().all(|at| (-15..=0).contains(&at.floor))
                    })
                    && groups.iter().map(|group| group.cells.len()).sum::<usize>() <= 64
                    && groups
                        .iter()
                        .flat_map(|group| &group.cells)
                        .collect::<BTreeSet<_>>()
                        .len()
                        == groups.iter().map(|group| group.cells.len()).sum::<usize>()
            }
        }
    }
}
