//! Current source eligibility observation, not a mutation or client claim.
//! Raw active conditions intentionally include suppressed stored statuses, as
//! Lua getCondition does; HUD visible_statuses is not an eligibility oracle.
use super::NpcSourceGreetContext;
use oteryn_gameplay_domain::domain::condition::{ConditionType, StatusKind};
#[derive(Clone, Debug)]
pub struct NpcSourceActorObservation {
    pub actor_identity: [u8; 16],
    pub movement_revision: u64,
    pub state_revision: u64,
    pub observed_owner_us: u64,
    pub player_conditions: serde_json::Value,
    pub player_statuses: Vec<(ConditionType, i64)>,
    pub native_statuses: Vec<(ConditionType, i64)>,
}
impl NpcSourceActorObservation {
    pub(crate) fn valid_for_greet(&self, greet: &NpcSourceGreetContext) -> bool {
        let Ok(player)=oteryn_gameplay_foundation::foundation::condition::ConditionStore::<String>::from_persistent_snapshot(&self.player_conditions,0) else {return false;};
        let player_drunk = player.instances().iter().any(|instance| {
            instance.definition().condition_type() == ConditionType::Status(StatusKind::Drunk)
                && instance
                    .status_remaining_ms(0)
                    .is_some_and(|remaining| remaining != 0)
        });
        let captured_player_drunk = self.player_statuses.iter().any(|(kind, remaining)| {
            *kind == ConditionType::Status(StatusKind::Drunk) && *remaining != 0
        });
        player_drunk == captured_player_drunk
            && self.actor_identity == greet.actor_identity
            && self.movement_revision == greet.movement_revision
            && self.state_revision > 0
            && self.player_statuses.len() <= 64
            && self.native_statuses.len() <= 64
            && self
                .player_statuses
                .iter()
                .chain(&self.native_statuses)
                .all(|(_, remaining)| *remaining >= -1)
            && serde_json::to_vec(&self.observation()).is_ok_and(|body| body.len() <= 65536)
    }

    pub(crate) fn drunk(&self) -> bool {
        self.player_statuses
            .iter()
            .chain(&self.native_statuses)
            .any(|(kind, remaining)| {
                *kind == ConditionType::Status(StatusKind::Drunk) && *remaining != 0
            })
    }
    pub(crate) fn observation(&self) -> serde_json::Value {
        serde_json::json!({
            "actor_identity":self.actor_identity.iter().map(|b|format!("{b:02x}")).collect::<String>(),
            "movement_revision":self.movement_revision.to_string(),
            "state_revision":self.state_revision.to_string(),
            "observed_owner_us":self.observed_owner_us.to_string(),
            "player_conditions":self.player_conditions,
            "player_statuses":self.player_statuses,
            "native_statuses":self.native_statuses,
        })
    }
}
