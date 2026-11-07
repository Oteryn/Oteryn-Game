//! REWARD-CHEST-WIRE-1 / SV11: source-derived bag, present and box outcomes.
//! Lists and weights come from the generated Canary/Crystal overlay; unplayable bags fail closed.
use crate::item_craft::{CookingConversion, output_definition};
use oteryn_gameplay_content::content::ReferenceItemTarget;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use oteryn_gameplay_domain::domain::timed_item_host::resolve_timed_definition;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub mod event_containers;

/// Its own tag, so no combat or world-item draw shifts.
const DRAW_TAG: &[u8] = b"oteryn:reward-bag-draw:v1";

#[derive(Debug, serde::Deserialize)]
pub struct RewardBag {
    #[serde(default)]
    selection: Selection,
    #[serde(default = "one")]
    picks: u32,
    roll_max: Option<u64>,
    pub constant: String,
    pub bag: String,
    pub playable: bool,
    pub rewards: Vec<Reward>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Reward {
    #[serde(default = "one")]
    minimum: u32,
    #[serde(default = "one")]
    maximum: u32,
    pub item: String,
    pub name: String,
    pub weight: u64,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum Selection {
    #[default]
    Weighted,
    One,
    All,
    WithoutReplacement,
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
            "../../../../tools/content-schema/native-gameplay/reward-bag-catalogue.json"
        ))
        .ok()
        .map(|o| o.bags)
    })
    .as_deref()
}

/// The craft owner may mint a captured container reward empty, never with children.
pub fn is_reward_bag_output(recipe: &str, key: &str) -> bool {
    reward_bags().is_some_and(|bags| {
        bags.iter().any(|bag| {
            bag.playable
                && recipe == format!("oteryn:reward-bag.{}", bag.constant.to_ascii_lowercase())
                && bag.rewards.iter().any(|reward| reward.item == key)
        })
    })
}

/// The existing USE route recognizes the same generated bags and W19 source tables
/// that the durable conversion resolves again under the item fence.
pub fn is_reward_bag(key: &str) -> bool {
    event_containers::event_container_rule(key).is_some()
        || reward_bags().is_some_and(|bags| playable_bag(bags, key).is_some())
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

/// Deterministic unbiased draw in `0..upper`; rejection avoids modulo bias.
pub fn reward_bag_bounded_draw(
    game_session_id: [u8; 16],
    command_id: u64,
    source: [u8; 16],
    domain: u8,
    upper: u64,
) -> Option<u64> {
    if upper == 0 {
        return None;
    }
    let zone = u64::MAX - u64::MAX % upper;
    for counter in 0_u32..=u32::MAX {
        let mut hasher = Sha256::new();
        hasher.update(DRAW_TAG);
        hasher.update(game_session_id);
        hasher.update(command_id.to_be_bytes());
        hasher.update(source);
        hasher.update([domain]);
        hasher.update(counter.to_be_bytes());
        let digest = hasher.finalize();
        let mut first = [0_u8; 8];
        first.copy_from_slice(&digest[..8]);
        let value = u64::from_be_bytes(first);
        if value < zone {
            return Some(value % upper);
        }
    }
    None
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
    if event_containers::event_container_rule(&used.production_key).is_some() {
        return event_containers::conversion(native, used, draw);
    }
    let bag = playable_bag(reward_bags()?, &used.production_key)?;
    let mut outputs = Vec::new();
    let mut available: Vec<_> = bag.rewards.iter().collect();
    let picks = match bag.selection {
        Selection::All => u32::try_from(available.len()).ok()?,
        Selection::WithoutReplacement => bag.picks,
        _ => 1,
    };
    for pick in 0..picks {
        let reward = match bag.selection {
            Selection::All => *available.get(usize::try_from(pick).ok()?)?,
            Selection::WithoutReplacement => {
                let index = outcome_draw(draw, pick, 0, u64::try_from(available.len()).ok()?)?;
                available.remove(usize::try_from(index).ok()?)
            }
            Selection::One => {
                let index = outcome_draw(draw, pick, 0, u64::try_from(available.len()).ok()?)?;
                *available.get(usize::try_from(index).ok()?)?
            }
            Selection::Weighted => {
                // Legacy bags keep their exact receipt draw. W19's explicit roll bound
                // preserves music-box thresholds, including the unreachable tail.
                let roll = bag
                    .roll_max
                    .map_or(Some(draw), |upper| outcome_draw(draw, pick, 0, upper))?;
                select_reward(&bag.rewards, roll)?
            }
        };
        let count = reward.minimum.checked_add(
            u32::try_from(outcome_draw(
                draw,
                pick,
                1,
                u64::from(reward.maximum.checked_sub(reward.minimum)?.checked_add(1)?),
            )?)
            .ok()?,
        )?;
        // Generated bags deliver through the same Crystal Item::CreateItem factory
        // as the captured W19 tables (music box 3549 grants base soft boots 6529).
        let id = event_containers::created_reward_item_id(
            reward
                .item
                .strip_prefix("oteryn:item.tibia.i")?
                .parse()
                .ok()?,
        );
        let record = native.item_policy_for_source_id(id)?.record();
        let target = &record.production_binding.target;
        let definition = output_definition(native, target).or_else(|| {
            // A newly minted active-time reward has no checkpoint row: TIMED-ITEM-0
            // interprets revision zero as full values when the item becomes live.
            // Validate that definition through the existing timed owner first.
            let reference = ReferenceItemTarget::new(&target.key, &target.revision).ok()?;
            resolve_timed_definition(&reference, &record.semantics).ok()??;
            record.admission.materializable.then(|| TypedDefinitionRef {
                family: record.production_definition.family.clone(),
                production_key: record.production_definition.production_key.clone(),
                revision_ref: record.production_definition.revision_ref.clone(),
            })
        })?;
        outputs.push((definition, count));
    }
    (!outputs.is_empty()).then(|| CookingConversion {
        recipe: format!("oteryn:reward-bag.{}", bag.constant.to_ascii_lowercase()),
        outputs,
    })
}

/// Independent selection/quantity streams, with rejection to avoid modulo bias.
fn outcome_draw(seed: u64, pick: u32, domain: u8, upper: u64) -> Option<u64> {
    if upper == 0 {
        return None;
    }
    let zone = u64::MAX - u64::MAX % upper;
    for counter in 0_u32..=u32::MAX {
        let mut hasher = Sha256::new();
        hasher.update(b"oteryn:event-bag-outcome:v1");
        hasher.update(seed.to_be_bytes());
        hasher.update(pick.to_be_bytes());
        hasher.update([domain]);
        hasher.update(counter.to_be_bytes());
        let digest = hasher.finalize();
        let mut first = [0; 8];
        first.copy_from_slice(&digest[..8]);
        let value = u64::from_be_bytes(first);
        if value < zone {
            return Some(value % upper);
        }
    }
    None
}

fn playable_bag<'a>(bags: &'a [RewardBag], key: &str) -> Option<&'a RewardBag> {
    bags.iter().find(|bag| bag.playable && bag.bag == key)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn bounded_draw_rejects_tail_and_preserves_seeded_domains() {
        let draw = |upper| reward_bag_bounded_draw([1; 16], 7, [2; 16], 3, upper);
        assert_eq!(draw(0), None);
        assert_eq!(draw(1), Some(0));
        assert_eq!(draw(98), Some(29));
        assert_eq!(draw(10_000), Some(4437));
        assert_eq!(draw(420), Some(197));
        // This seed rejects counters 0 and 1, then accepts counter 2.
        assert_eq!(draw((1 << 63) + 1), Some(845_863_996_175_274_660));
        assert_ne!(
            draw(10_000),
            reward_bag_bounded_draw([1; 16], 7, [2; 16], 2, 10_000)
        );
    }

    fn reward(weight: u64) -> Reward {
        Reward {
            minimum: 1,
            maximum: 1,
            item: String::new(),
            name: String::new(),
            weight,
        }
    }

    #[test]
    fn overlay_admits_all_three_bags_with_canary_weights() {
        let bags = reward_bags().expect("overlay parses");
        assert_eq!(bags.len(), 28);
        for (bag, (constant, key, count, total)) in bags.iter().zip([
            ("BAG_YOU_DESIRE", "oteryn:item.tibia.i34109", 21, 81_000),
            ("PRIMAL_BAG", "oteryn:item.tibia.i39546", 14, 54_000),
            ("BAG_YOU_COVET", "oteryn:item.tibia.i43895", 26, 55_010),
        ]) {
            assert!(bag.playable);
            assert_eq!((bag.constant.as_str(), bag.bag.as_str()), (constant, key));
            assert_eq!(bag.rewards.len(), count);
            assert_eq!(bag.rewards.iter().map(|r| r.weight).sum::<u64>(), total);
            assert!(bag.rewards.iter().all(|r| r.weight > 0));
        }
    }

    #[test]
    fn gnomish_supply_preserves_original_rare_intervals() -> Result<(), String> {
        let bag = reward_bags()
            .ok_or("catalogue")?
            .iter()
            .find(|b| b.bag == "oteryn:item.tibia.i15698")
            .ok_or("supply package")?;
        assert_eq!(bag.roll_max, Some(10001));
        assert_eq!(bag.rewards.len(), 25);
        assert_eq!(bag.rewards.iter().map(|r| r.weight).sum::<u64>(), 10001);
        for roll in [9636, 9746] {
            assert_eq!(
                select_reward(&bag.rewards, roll).ok_or("CA2")?.item,
                "oteryn:item.tibia.i16257"
            );
        }
        for roll in [9747, 9835] {
            assert_eq!(
                select_reward(&bag.rewards, roll).ok_or("MA2")?.item,
                "oteryn:item.tibia.i16254"
            );
        }
        assert_ne!(
            select_reward(&bag.rewards, 9635).ok_or("before")?.item,
            "oteryn:item.tibia.i16257"
        );
        assert_ne!(
            select_reward(&bag.rewards, 9836).ok_or("after")?.item,
            "oteryn:item.tibia.i16254"
        );
        Ok(())
    }

    #[test]
    fn unplayable_reward_bag_stays_closed_even_with_valid_rewards() {
        let key = "oteryn:item.tibia.i34109";
        let bags = [RewardBag {
            selection: Selection::Weighted,
            picks: 1,
            roll_max: None,
            constant: "SYNTHETIC_UNPLAYABLE".into(),
            bag: key.into(),
            playable: false,
            rewards: vec![reward(1)],
        }];
        assert!(select_reward(&bags[0].rewards, 0).is_some());
        assert!(playable_bag(&bags, key).is_none());
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
