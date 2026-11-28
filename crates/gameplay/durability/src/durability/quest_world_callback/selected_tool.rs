//! The carried selection accompanies a World turn without changing legacy contexts.
use super::{QuestWorldContext, paid_projection::PaidArrival};
use oteryn_protocol_oteryn::{
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
    world_object::{CAPABILITY_ITEM_USE_V1, MapUseTarget, UseTarget, decode_use_intent_target},
};
use serde::{Deserialize, Serialize};

const USE_CAPABILITIES: [u32; 3] = [
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_ITEM_USE_V1,
    CAPABILITY_WORLD_INTERACTION_V1,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QuestWorldToolContext {
    #[serde(flatten)]
    world: QuestWorldContext,
    /// Frozen native checkpoint data; the existing receipt binding remains
    /// compatible with historical callbacks and binds the authored destination.
    #[serde(skip)]
    pub(super) paid_arrivals: Vec<PaidArrival>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) selected_tool: Option<SelectedQuestTool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SelectedQuestTool {
    pub source: [u8; 16],
    pub intent: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) target: Option<SelectedGroundTarget>,
}

impl From<QuestWorldContext> for QuestWorldToolContext {
    fn from(world: QuestWorldContext) -> Self {
        Self {
            world,
            paid_arrivals: Vec::new(),
            selected_tool: None,
        }
    }
}
impl QuestWorldToolContext {
    pub fn with_paid_arrivals(mut self, arrivals: Vec<PaidArrival>) -> Self {
        self.paid_arrivals = arrivals;
        self
    }

    /// The replay identity of a legacy World claim. Like main's RewardClaim
    /// chest owner, its receipt is keyed by cause and claim; the actor's
    /// movement proof is re-derived by a rebuilt owner and is checked live.
    pub(crate) fn without_actor_movement(&self) -> Self {
        let mut stable = self.clone();
        stable.world.actor_placement = [0; 16];
        stable.world.movement_revision = 0;
        stable
    }
}
impl std::ops::Deref for QuestWorldToolContext {
    type Target = QuestWorldContext;
    fn deref(&self) -> &Self::Target {
        &self.world
    }
}
impl QuestWorldContext {
    /// The USE owner supplies the exact carried instance resolved from its current view.
    pub fn with_selected_tool(
        self,
        source: [u8; 16],
        intent: Vec<u8>,
    ) -> Option<QuestWorldToolContext> {
        if source == [0; 16] || intent.is_empty() || intent.len() > 4096 {
            return None;
        }
        let decoded = decode_use_intent_target(&USE_CAPABILITIES, &intent).ok()?;
        if matches!(decoded.target, UseTarget::WorldObject(_))
            || decoded.use_with.is_some()
            || decoded.use_with_position.is_some()
            || !matches!(decoded.use_with_map_item, Some(MapUseTarget::Object(_)))
        {
            return None;
        }
        Some(QuestWorldToolContext {
            world: self,
            paid_arrivals: Vec::new(),
            selected_tool: Some(SelectedQuestTool {
                source,
                intent,
                target: None,
            }),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectedGroundTarget {
    pub item: [u8; 16],
    pub actor_position: [i32; 3],
    pub target_position: [i32; 3],
}

impl QuestWorldContext {
    /// The current map-handle owner supplies the Ground instance and actor position.
    pub fn with_selected_ground_tool(
        self,
        source: [u8; 16],
        intent: Vec<u8>,
        target: [u8; 16],
        actor_position: [i32; 3],
        target_position: [i32; 3],
    ) -> Option<QuestWorldToolContext> {
        if source == [0; 16] || target == [0; 16] || source == target || intent.len() > 4096 {
            return None;
        }
        let decoded = decode_use_intent_target(&USE_CAPABILITIES, &intent).ok()?;
        if matches!(decoded.target, UseTarget::WorldObject(_))
            || decoded.use_with.is_some()
            || decoded.use_with_position.is_some()
            || !matches!(decoded.use_with_map_item, Some(MapUseTarget::Item(_)))
        {
            return None;
        }
        Some(QuestWorldToolContext {
            world: self,
            paid_arrivals: Vec::new(),
            selected_tool: Some(SelectedQuestTool {
                source,
                intent,
                target: Some(SelectedGroundTarget {
                    item: target,
                    actor_position,
                    target_position,
                }),
            }),
        })
    }
}
