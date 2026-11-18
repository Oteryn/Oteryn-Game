//! Crystal 201189f8 actions_holy_water.lua:105-135 and data/items/items.xml:21313-21336.
use super::{ActorKind, Callback, Trigger};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChargeWindow {
    Charge {
        group: String,
        tool: String,
        phases: Vec<ChargePhase>,
        cue: u8,
        announcement: String,
        radius: u8,
    },
    Spend {
        group: String,
        active: u16,
        announcement: String,
        radius: u8,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChargePhase {
    pub from: u16,
    pub burning: u16,
    pub ready: u16,
    pub burn_ms: u32,
    pub ready_ms: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<[u8; 2]>,
}

impl ChargeWindow {
    pub fn accepts(&self, appearance: u16) -> bool {
        match self {
            Self::Charge { phases, .. } => phases.iter().any(|p| p.from == appearance),
            Self::Spend { active, .. } => *active == appearance,
        }
    }

    pub fn group(&self) -> &str {
        match self {
            Self::Charge { group, .. } | Self::Spend { group, .. } => group,
        }
    }
}

pub(super) fn disjoint(left_callback: &Callback, right_callback: &Callback) -> bool {
    let (Some(left), Some(right)) = (
        &left_callback.effects.charge_window,
        &right_callback.effects.charge_window,
    ) else {
        return false;
    };
    let (phases, active) = match (left, right) {
        (ChargeWindow::Charge { phases, .. }, ChargeWindow::Spend { active, .. })
        | (ChargeWindow::Spend { active, .. }, ChargeWindow::Charge { phases, .. }) => {
            (phases, active)
        }
        (ChargeWindow::Spend { .. }, ChargeWindow::Spend { .. })
            if left == right && same_tool_and_disjoint_storage(left_callback, right_callback) =>
        {
            return true;
        }
        _ => return false,
    };
    left.group() == right.group() && phases.iter().all(|p| p.from != *active)
}

fn same_tool_and_disjoint_storage(left: &Callback, right: &Callback) -> bool {
    left.quest == right.quest
        && left.item_on_target == right.item_on_target
        && left.storage.iter().any(|a| {
            right
                .storage
                .iter()
                .any(|b| a.track == b.track && (a.maximum < b.minimum || b.maximum < a.minimum))
        })
}

/// The retained base Source can be qualified by an exact position and a unique
/// original appearance, even when an import does not retain a stable ordinal.
pub fn charge_source(callback: &Callback) -> Option<(super::Position, Option<u8>, u16)> {
    if let Some(source) = &callback.selector.native {
        return Some((source.position, Some(source.ordinal), source.appearance));
    }
    let selector = &callback.selector;
    if selector.action.is_some()
        || selector.unique.is_some()
        || !selector.position_aliases.is_empty()
    {
        return None;
    }
    Some((
        selector.position?,
        selector.ordinal.map(u8::try_from).transpose().ok()?,
        selector.appearance?,
    ))
}

pub(super) fn valid(callback: &Callback, charge: &ChargeWindow) -> bool {
    let Some((_, _, initial)) = charge_source(callback) else {
        return false;
    };
    if callback.trigger != Trigger::Use
        || callback.actor_kind != ActorKind::Player
        || callback.lever.is_some()
        || callback.reversible
        || callback.effects.transform.is_some()
        || callback.effects.reset_after_ms.is_some()
        || callback.effects.target_transform.is_some()
        || callback.effects.encounter.is_some()
        || callback.effects.scene.is_some()
        || callback.teleport.is_some()
        || !callback.passages.is_empty()
        || !callback.offerings.is_empty()
        || !crate::quest::valid_quest_key(charge.group())
    {
        return false;
    }
    let text_valid = |text: &str, radius| {
        !text.is_empty()
            && text.len() <= 200
            && !text.chars().any(char::is_control)
            && (1..=8).contains(&radius)
    };
    match charge {
        ChargeWindow::Charge {
            tool,
            phases,
            cue,
            announcement,
            radius,
            ..
        } => {
            let sources: BTreeSet<_> = phases.iter().map(|p| p.from).collect();
            let burning: BTreeSet<_> = phases.iter().map(|p| p.burning).collect();
            *cue > 0
                && text_valid(announcement, *radius)
                && crate::quest::valid_quest_key(tool)
                && !callback.requires_durable_commit()
                && callback.item_on_target.is_none()
                && (1..=8).contains(&phases.len())
                && sources.len() == phases.len()
                && burning.len() == phases.len()
                && sources.contains(&initial)
                && sources.is_disjoint(&burning)
                && phases.iter().all(|phase| {
                    phase.from != 0
                        && phase.burning != 0
                        && phase.ready != 0
                        && phase.ready != phase.burning
                        && (1..=1_800_000).contains(&phase.burn_ms)
                        && (1..=1_800_000).contains(&phase.ready_ms)
                        && phase
                            .capacity
                            .is_none_or(|[min, max]| min > 0 && min <= max && max <= 64)
                })
        }
        ChargeWindow::Spend {
            active,
            announcement,
            radius,
            ..
        } => {
            *active != initial
                && *active != 0
                && text_valid(announcement, *radius)
                && callback.requires_world_admission
                && callback.has_state_writes()
                && callback.item_on_target.as_ref().is_some_and(|tool| {
                    tool.selected_instance
                        && !tool.reusable
                        && tool.count == 1
                        && tool.replacement.is_none()
                        && !tool.target_remove
                        && tool.target_replacement.is_none()
                        && tool.ingredients.is_empty()
                        && tool.alternatives.is_empty()
                })
        }
    }
}
