//! Bounded source scene edits; every effect publishes through one map projection.
use super::*;

pub const MAX_SCENE_EDITS: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneEdit {
    pub position: Position,
    pub remove: Option<u16>,
    pub create: Option<u16>,
    /// Collision of the resulting replacement or created immovable object.
    pub passable: Option<bool>,
    #[serde(default)]
    pub allow_missing: bool,
}

/// Bounded source occupants transported to one safe, shared landing.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneRelocation {
    pub from: Vec<Position>,
    pub to: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneEvacuation {
    pub from: Position,
    pub players_to: Position,
    pub creatures_to: Position,
    pub all: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneEdits {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relocations: Vec<SceneRelocation>,
    pub open: Vec<SceneEdit>,
    pub close: Vec<SceneEdit>,
    #[serde(default)]
    pub remove_self: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evacuate_open: Vec<SceneEvacuation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evacuate_close: Vec<SceneEvacuation>,
    /// Move movable ground custody through the existing item-loop/MINT owners on close.
    #[serde(default, skip_serializing_if = "is_false")]
    pub relocate_items_close: bool,
}

pub(super) fn valid_scene(callback: &Callback, scene: &SceneEdits) -> bool {
    let valid = |edits: &[SceneEdit]| {
        edits.len() <= MAX_SCENE_EDITS
            && edits
                .iter()
                .map(|edit| (edit.position, edit.remove))
                .collect::<BTreeSet<_>>()
                .len()
                == edits.len()
            && edits.iter().all(|edit| {
                (-15..=0).contains(&edit.position.floor)
                    && callback.selector.position != Some(edit.position)
                    && (edit.remove.is_some() || edit.create.is_some())
                    && edit.remove != Some(0)
                    && edit.create != Some(0)
                    && (edit.remove.is_none() || edit.remove != edit.create)
                    && edit.create.is_some() == edit.passable.is_some()
                    && (!edit.allow_missing || edit.remove.is_some())
            })
    };
    let valid_evacuation = |moves: &[SceneEvacuation], edits: &[SceneEdit]| {
        moves.len() <= 8
            && moves.iter().map(|m| m.from).collect::<BTreeSet<_>>().len() == moves.len()
            && moves.iter().all(|m| {
                edits.iter().any(|e| e.position == m.from)
                    && [m.players_to, m.creatures_to].into_iter().all(|to| {
                        to.floor == m.from.floor && !edits.iter().any(|e| e.position == to)
                    })
            })
    };
    let origins: Vec<_> = scene
        .relocations
        .iter()
        .flat_map(|relocation| relocation.from.iter())
        .collect();
    let edited: BTreeSet<_> = scene
        .open
        .iter()
        .chain(&scene.close)
        .map(|edit| edit.position)
        .collect();
    let relocation_valid = scene.relocations.len() <= MAX_SCENE_EDITS
        && origins.len() <= MAX_SCENE_EDITS
        && origins.iter().copied().collect::<BTreeSet<_>>().len() == origins.len()
        && scene.relocations.iter().all(|relocation| {
            !relocation.from.is_empty()
                && (-15..=0).contains(&relocation.to.floor)
                && !edited.contains(&relocation.to)
                && relocation.from.iter().all(|position| {
                    position.floor == relocation.to.floor && edited.contains(position)
                })
        });
    callback.trigger == Trigger::Use
        && callback.lever.is_some()
        && callback.passages.is_empty()
        && callback.teleport.is_none()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.native_rule.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.effects.transform.is_none()
        && callback.effects.magic.is_none()
        && callback.effects.facing.is_none()
        && callback.effects.item_reward.is_none()
        && callback.effects.outfit_reward.is_none()
        && !callback.completes
        && !callback.premium_required
        && (!scene.open.is_empty() || scene.remove_self)
        && (!scene.remove_self || (!callback.reversible && scene.close.is_empty()))
        && (!callback.reversible || !scene.close.is_empty())
        && relocation_valid
        && (scene.relocations.is_empty()
            || (scene.evacuate_open.is_empty() && scene.evacuate_close.is_empty()))
        && valid(&scene.open)
        && valid(&scene.close)
        && valid_evacuation(&scene.evacuate_open, &scene.open)
        && valid_evacuation(&scene.evacuate_close, &scene.close)
        && (!scene.relocate_items_close
            || (!scene.evacuate_close.is_empty() && scene.evacuate_close.iter().all(|m| m.all)))
}
