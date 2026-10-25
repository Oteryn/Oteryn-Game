//! REWARD-CHEST-WIRE-1: one weighted reward per used reward bag.
//! Lists and weights come from the generated Canary/Crystal overlay; unplayable bags fail closed.
use crate::item_craft::{CookingConversion, output_definition};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// Its own tag, so no combat or world-item draw shifts.
const DRAW_TAG: &[u8] = b"oteryn:reward-bag-draw:v1";

#[derive(Debug, serde::Deserialize)]
pub struct RewardBag {
    pub constant: String,
    pub bag: String,
    pub playable: bool,
    pub rewards: Vec<Reward>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Reward {
    pub item: String,
    pub name: String,
    pub weight: u64,
}

/// Every bag of the overlay in source order; `None` if the bundled overlay does not parse.
pub fn reward_bags() -> Option<&'static [RewardBag]> {
    #[derive(serde::Deserialize)]
    struct Overlay {
        bags: Vec<RewardBag>,
    }
    static BAGS: OnceLock<Option<Vec<RewardBag>>> = OnceLock::new();
    BAGS.get_or_init(|| {
        serde_json::from_str::<Overlay>(include_str!(
            "../../../../tools/content-schema/native-gameplay/reward-bag-profiles.json"
        ))
        .ok()
        .map(|o| o.bags)
    })
    .as_deref()
}

/// The draw for one command on one source; a receipt replay never redraws.
pub fn reward_bag_draw(game_session_id: [u8; 16], command_id: u64, source: [u8; 16]) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(DRAW_TAG);
    hasher.update(game_session_id);
    hasher.update(command_id.to_be_bytes());
    hasher.update(source);
    let digest = hasher.finalize();
    let mut first = [0_u8; 8];
    first.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(first)
}

/// Canary `selectReward`: the first reward whose cumulative weight exceeds the roll.
pub fn select_reward(rewards: &[Reward], draw: u64) -> Option<&Reward> {
    let total = rewards
        .iter()
        .try_fold(0_u64, |sum, r| sum.checked_add(r.weight))?;
    let roll = draw.checked_rem(total)?;
    let mut cumulative = 0_u64;
    rewards.iter().find(|reward| {
        cumulative += reward.weight;
        roll < cumulative
    })
}

/// Resolve USED as a playable reward bag and mint the drawn reward in its place.
pub fn reward_bag_conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    draw: u64,
) -> Option<CookingConversion> {
    let record = native
        .item_policy(&used.production_key, &used.revision_ref)?
        .record();
    if record.production_definition.family != used.family || !record.admission.materializable {
        return None;
    }
    let bag = reward_bags()?
        .iter()
        .find(|bag| bag.playable && bag.bag == used.production_key)?;
    let reward = select_reward(&bag.rewards, draw)?;
    let id = reward
        .item
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()?;
    let target = &native
        .item_policy_for_source_id(id)?
        .record()
        .production_binding
        .target;
    Some(CookingConversion {
        recipe: format!("oteryn:reward-bag.{}", bag.constant.to_ascii_lowercase()),
        outputs: vec![(output_definition(native, target)?, 1)],
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn reward(weight: u64) -> Reward {
        Reward {
            item: String::new(),
            name: String::new(),
            weight,
        }
    }

    #[test]
    fn overlay_admits_only_the_covet_bag_with_canary_weights() {
        let bags = reward_bags().expect("overlay parses");
        let playable: Vec<_> = bags.iter().filter(|b| b.playable).collect();
        assert_eq!(playable.len(), 1);
        let covet = playable[0];
        assert_eq!(
            (covet.constant.as_str(), covet.bag.as_str()),
            ("BAG_YOU_COVET", "oteryn:item.tibia.i43895")
        );
        assert_eq!(covet.rewards.len(), 26);
        assert_eq!(covet.rewards.iter().map(|r| r.weight).sum::<u64>(), 55_010);
        assert!(covet.rewards.iter().all(|r| r.weight > 0));
    }

    #[test]
    fn selection_follows_cumulative_weights() {
        let rewards = [reward(1), reward(5000), reward(2)];
        let pick = |draw| select_reward(&rewards, draw).map(|r| r.weight);
        assert_eq!(pick(0), Some(1));
        assert_eq!(pick(1), Some(5000));
        assert_eq!(pick(5000), Some(5000));
        assert_eq!(pick(5001), Some(2));
        assert_eq!(pick(5002), Some(2));
        assert_eq!(pick(5003), Some(1));
        assert!(select_reward(&[], 7).is_none());
        assert!(select_reward(&[reward(0)], 7).is_none());
    }

    #[test]
    fn draw_is_stable_and_keyed_by_session_command_and_source() {
        let draw = reward_bag_draw([1; 16], 7, [2; 16]);
        assert_eq!(draw, reward_bag_draw([1; 16], 7, [2; 16]));
        assert_ne!(draw, reward_bag_draw([3; 16], 7, [2; 16]));
        assert_ne!(draw, reward_bag_draw([1; 16], 8, [2; 16]));
        assert_ne!(draw, reward_bag_draw([1; 16], 7, [4; 16]));
    }
}
