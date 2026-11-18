//! Exact shared board, pillars and native portal slot.
use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoardToken {
    pub position: Position,
    pub appearance: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoardPortal {
    pub tokens: Vec<BoardToken>,
    pub pillars: Vec<Position>,
    pub pillar: u16,
    pub portal: Position,
    pub sealed: u16,
    pub opened: u16,
    pub open_on: u16,
}

impl BoardPortal {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        let positions: Vec<_> = self
            .tokens
            .iter()
            .map(|t| t.position)
            .chain(self.pillars.iter().copied())
            .chain([self.portal])
            .collect();
        callback.trigger == Trigger::Use
            && callback.actor_kind == ActorKind::Player
            && callback.selector.position.is_some()
            && !callback.has_personal_state()
            && callback.reversible
            && callback
                .lever
                .is_some_and(|pair| pair.contains(&self.open_on))
            && callback.passages.is_empty()
            && callback.offerings.is_empty()
            && callback.item_on_target.is_none()
            && callback.teleport.is_none()
            && callback.remote_teleport.is_none()
            && callback.effects
                == CallbackEffectSet::from_shared(CallbackEffects {
                    board_portal: Some(self.clone()),
                    ..CallbackEffects::default()
                })
            && (1..=16).contains(&self.tokens.len())
            && (1..=8).contains(&self.pillars.len())
            && self.tokens.iter().all(|token| token.appearance != 0)
            && self.pillar != 0
            && self.sealed != 0
            && self.opened != 0
            && self.sealed != self.opened
            && positions
                .iter()
                .all(|p| p.floor == self.portal.floor && Some(*p) != callback.selector.position)
            && (-15..=0).contains(&self.portal.floor)
            && positions.iter().copied().collect::<BTreeSet<_>>().len() == positions.len()
    }
}
