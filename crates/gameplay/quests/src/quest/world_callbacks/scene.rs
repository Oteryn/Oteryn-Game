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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneEdits {
    pub open: Vec<SceneEdit>,
    pub close: Vec<SceneEdit>,
    #[serde(default)]
    pub remove_self: bool,
}

pub(super) fn valid_scene(callback: &Callback, scene: &SceneEdits) -> bool {
    let valid = |edits: &[SceneEdit]| {
        edits.len() <= MAX_SCENE_EDITS
            && edits
                .iter()
                .map(|edit| edit.position)
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
    callback.trigger == Trigger::Use
        && callback.lever.is_some()
        && callback.storage.is_empty()
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
        && valid(&scene.open)
        && valid(&scene.close)
}
