//! Bounded source scene edits; every effect publishes through one map projection.
use super::*;

pub const MAX_SCENE_EDITS: usize = 32;
pub const EARTH_TREE_LIFETIME_US: u64 = 60_000_000;

pub const EARTH_TREE: Position = Position {
    x: 32737,
    y: 32116,
    floor: -10,
};
pub const EARTH_TREE_PORTAL: Position = Position {
    x: 32736,
    y: 32117,
    floor: -10,
};
pub const EARTH_TREE_CHAMBER: Position = Position {
    x: 32720,
    y: 32927,
    floor: -14,
};
pub const EARTH_TREE_RETURN: Position = Position {
    x: 32737,
    y: 32117,
    floor: -10,
};
pub const EARTH_TREE_REQUIREMENTS: [(&str, i64); 2] = [
    (
        "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/bird_counter",
        3,
    ),
    (
        "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/plant_counter",
        5,
    ),
];

/// The Crystal world-tree action borrows a phial and shares one one-minute scene.
pub fn earth_tree_scene(callback: &Callback) -> bool {
    callback.quest == "oteryn:quest.forgotten_knowledge_quest"
        && callback.selector.position == Some(EARTH_TREE)
        && callback.selector.appearance == Some(6180)
        && callback.lever.is_none()
        && callback.requires_world_admission
        && !callback.reversible
        && callback.item_on_target.as_ref().is_some_and(|tool| {
            tool.item == "oteryn:item.tibia.i23810"
                && tool.alternatives.is_empty()
                && tool.count == 1
                && tool.reusable
                && tool.selected_instance
                && tool.ingredients.is_empty()
                && tool.replacement.is_none()
                && tool.target_replacement.is_none()
                && !tool.target_remove
        })
        && callback.storage.len() == EARTH_TREE_REQUIREMENTS.len()
        && callback.operation_count() == 0
        && EARTH_TREE_REQUIREMENTS.iter().all(|(key, minimum)| {
            callback.storage.iter().any(|state| {
                state.track == *key
                    && state.minimum == *minimum
                    && state.maximum == i64::from(i32::MAX)
                    && state.preserve
            })
        })
}

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
    let timed_tree = earth_tree_scene(callback);
    let valid = |edits: &[SceneEdit]| {
        let limit = if timed_tree { 512 } else { MAX_SCENE_EDITS };
        edits.len() <= limit
            && (timed_tree
                || edits
                    .iter()
                    .map(|edit| (edit.position, edit.remove))
                    .collect::<BTreeSet<_>>()
                    .len()
                    == edits.len())
            && edits.iter().all(|edit| {
                if timed_tree {
                    let at = edit.position;
                    if !(at.floor == -11
                        && (32731..=32745).contains(&at.x)
                        && (32108..=32121).contains(&at.y))
                        && at != EARTH_TREE_PORTAL
                        && at
                            != (Position {
                                x: 32737,
                                y: 32114,
                                floor: -10,
                            })
                    {
                        return false;
                    }
                }
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
        && (callback.lever.is_some() || timed_tree)
        && callback.passages.is_empty()
        && callback.teleport.is_none()
        && callback.offerings.is_empty()
        && (callback.item_on_target.is_none() || timed_tree)
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
        && (!timed_tree || (scene.close.is_empty() && !scene.remove_self))
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
