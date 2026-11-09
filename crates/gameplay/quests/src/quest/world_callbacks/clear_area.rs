//! A named-species census is a physical stage predicate, never a client kill claim.
use super::*;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClearArea {
    pub minimum: Position,
    pub maximum: Position,
    /// Extra levels of this same bounded footprint; legacy predicates stay single-floor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_floors: Vec<i8>,
    pub creatures: Vec<String>,
}

impl ClearArea {
    pub fn contains(&self, position: Position) -> bool {
        (position.floor == self.minimum.floor || self.additional_floors.contains(&position.floor))
            && (self.minimum.x..=self.maximum.x).contains(&position.x)
            && (self.minimum.y..=self.maximum.y).contains(&position.y)
    }

    pub(super) fn valid(&self, callback: &Callback) -> bool {
        callback.requires_world_admission
            && callback.actor_kind == ActorKind::Player
            && !callback.reversible
            && !callback.opening_only
            && callback.effects.encounter.is_none()
            && callback.storage.iter().any(|state| !state.preserve)
            && callback.storage.iter().any(|state| state.minimum >= 1)
            && (-15..=0).contains(&self.minimum.floor)
            && self.minimum.floor == self.maximum.floor
            && self.additional_floors.len() <= 3
            && self
                .additional_floors
                .iter()
                .all(|floor| (-15..=0).contains(floor) && *floor != self.minimum.floor)
            && self.additional_floors.iter().collect::<BTreeSet<_>>().len()
                == self.additional_floors.len()
            && self.minimum.x <= self.maximum.x
            && self.minimum.y <= self.maximum.y
            && self.maximum.x - self.minimum.x < 64
            && self.maximum.y - self.minimum.y < 64
            && (1..=8).contains(&self.creatures.len())
            && self.creatures.iter().all(|key| {
                key.starts_with("oteryn:creature.")
                    && key.len() > "oteryn:creature.".len()
                    && super::super::valid_quest_key(key)
            })
            && self.creatures.iter().collect::<BTreeSet<_>>().len() == self.creatures.len()
    }
}
