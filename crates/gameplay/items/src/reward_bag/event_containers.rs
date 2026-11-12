//! W19a/W19e reward outcomes from Crystal 201189f's captured literal tables.
//! The existing reward-bag owner consumes and mints the complete plan in one receipt.
use super::reward_bag_bounded_draw;
use crate::item_craft::{CookingConversion, output_definition};
use oteryn_gameplay_content::content::native_gameplay::{
    NativeGameplayState, NativeItemStackClass,
};
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Selection {
    One,
    WithoutReplacement,
    All,
    Weighted,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventReward {
    pub item_id: u32,
    pub minimum: u32,
    pub maximum: u32,
    pub weight: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventCue {
    pub binding: String,
    pub effect_id: u16,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventPresentation {
    pub cue: EventCue,
    pub origin: String,
    pub per_reward: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EventContainerRule {
    pub item_id: u32,
    pub donor: String,
    pub selection: Selection,
    pub picks: usize,
    #[serde(default)]
    pub roll_max: Option<u64>,
    pub rewards: Vec<EventReward>,
    pub presentation: EventPresentation,
    pub ground_output: String,
}

/// The committed source packet includes the requested presents, Christmas bundles,
/// special boxes, anniversary boxes and music box. Glooth bag 21203 remains outside
/// this slice; its captured table is validated but is not a playable event rule.
pub fn event_container_rules() -> Option<&'static [EventContainerRule]> {
    #[derive(serde::Deserialize)]
    struct Catalogue {
        schema: String,
        crystal_commit: String,
        rules: Vec<EventContainerRule>,
    }
    static RULES: OnceLock<Option<Vec<EventContainerRule>>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let catalogue: Catalogue = serde_json::from_str(include_str!(
                "../../../../../tools/content-schema/native-gameplay/event-item-actions.json"
            ))
            .ok()?;
            (catalogue.schema == "OTERYN_EVENT_ITEM_REWARDS/v1"
                && catalogue.crystal_commit == "201189f871d23597b82caf88fd611454a2ddefb8"
                && !catalogue.rules.is_empty()
                && catalogue
                    .rules
                    .windows(2)
                    .all(|rules| rules[0].item_id < rules[1].item_id)
                && catalogue.rules.iter().all(valid_rule))
            .then_some(catalogue.rules)
        })
        .as_deref()
}

pub fn event_container_rule(key: &str) -> Option<&'static EventContainerRule> {
    // SP04 owns generated playable bags, including their historical receipts.
    // Keep the captured W19 tables as evidence without claiming those sources.
    if super::playable_bag(super::reward_bags()?, key).is_some() {
        return None;
    }
    let id: u32 = key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?;
    event_container_rules()?.iter().find(|rule| {
        rule.item_id == id && (rule.selection != Selection::Weighted || rule.item_id == 12045)
    })
}

fn bounded(draw: u64, domain: u8, upper: u64) -> Option<u64> {
    // The outer draw already binds session, command and source. Independent hash
    // domains separate gift selection from quantity and the seven bundle picks.
    reward_bag_bounded_draw([0; 16], draw, [0; 16], domain, upper)
}

fn valid_rule(rule: &EventContainerRule) -> bool {
    rule.item_id != 0
        && !rule.donor.is_empty()
        && !rule.rewards.is_empty()
        && rule.rewards.len() <= 128
        && rule.picks > 0
        && rule.picks <= rule.rewards.len()
        && rule.roll_max != Some(0)
        && rule.presentation.cue.effect_id > 0
        && rule
            .presentation
            .cue
            .binding
            .starts_with("appearance:effect/")
        && matches!(
            rule.presentation.origin.as_str(),
            "wire_source" | "item" | "player" | "source_or_player"
        )
        && matches!(rule.ground_output.as_str(), "character" | "source")
        && (!rule.presentation.per_reward || rule.selection == Selection::All)
        && rule.rewards.iter().all(|reward| {
            reward.item_id != 0
                && reward.weight > 0
                && reward.minimum > 0
                && reward.minimum <= reward.maximum
                && reward.maximum <= 100
                && (rule.selection == Selection::Weighted || reward.weight == 1)
        })
}

fn quantity(
    reward: &EventReward,
    draw: &mut impl FnMut(u8, u64) -> Option<u64>,
    domain: u8,
) -> Option<u32> {
    let range = u64::from(reward.maximum.checked_sub(reward.minimum)?.checked_add(1)?);
    let offset = draw(domain, range)?;
    (offset < range).then_some(())?;
    reward.minimum.checked_add(u32::try_from(offset).ok()?)
}

/// Return source IDs and counts before resolving the exact active Item definitions.
/// Crystal christmas_bundle.lua:16-26 draws seven without replacement. The immutable
/// captured table is reused for each command; receipt replay does not redraw.
pub fn selected_rewards(rule: &EventContainerRule, draw: u64) -> Option<Vec<(u32, u32)>> {
    selected_rewards_with(rule, |domain, upper| bounded(draw, domain, upper))
}

fn selected_rewards_with(
    rule: &EventContainerRule,
    mut draw: impl FnMut(u8, u64) -> Option<u64>,
) -> Option<Vec<(u32, u32)>> {
    if !valid_rule(rule) {
        return None;
    }
    let mut selected = Vec::new();
    match rule.selection {
        Selection::One if rule.picks == 1 => {
            let index = usize::try_from(draw(0, rule.rewards.len().try_into().ok()?)?).ok()?;
            let reward = rule.rewards.get(index)?;
            selected.push((reward.item_id, quantity(reward, &mut draw, 128)?));
        }
        Selection::WithoutReplacement => {
            let mut remaining: Vec<_> = rule.rewards.iter().collect();
            for pick in 0..rule.picks {
                let domain = u8::try_from(pick).ok()?;
                let index =
                    usize::try_from(draw(domain, remaining.len().try_into().ok()?)?).ok()?;
                if index >= remaining.len() {
                    return None;
                }
                let reward = remaining.remove(index);
                selected.push((
                    reward.item_id,
                    quantity(reward, &mut draw, 128_u8.checked_add(domain)?)?,
                ));
            }
        }
        Selection::All => {
            for (index, reward) in rule.rewards.iter().enumerate() {
                let domain = 128_u8.checked_add(u8::try_from(index).ok()?)?;
                selected.push((reward.item_id, quantity(reward, &mut draw, domain)?));
            }
        }
        Selection::Weighted if rule.picks == 1 => {
            let total = rule
                .rewards
                .iter()
                .try_fold(0_u64, |sum, reward| sum.checked_add(reward.weight))?;
            let upper = rule.roll_max.unwrap_or(total);
            if upper == 0 || upper > total {
                return None;
            }
            let roll = draw(0, upper)?;
            if roll >= upper {
                return None;
            }
            let mut cumulative = 0_u64;
            for reward in &rule.rewards {
                cumulative = cumulative.checked_add(reward.weight)?;
                if roll < cumulative {
                    selected.push((reward.item_id, quantity(reward, &mut draw, 128)?));
                    break;
                }
            }
        }
        _ => return None,
    }
    (!selected.is_empty()).then_some(selected)
}

/// Crystal Item::CreateItem normalizes activated soft boots to their base form:
/// src/items/item.cpp:50-63,106-108; utils_definitions.hpp:715-716.
/// player:addItem calls that factory (player_functions.cpp:2843), so the literal
/// music-box prize 3549 actually grants 6529 with the existing full I05 budget.
pub fn created_reward_item_id(id: u32) -> u32 {
    match id {
        3549 => 6529,
        _ => id,
    }
}

pub(super) fn conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    draw: u64,
) -> Option<CookingConversion> {
    let rule = event_container_rule(&used.production_key)?;
    let mut outputs = Vec::new();
    for (id, count) in selected_rewards(rule, draw)? {
        let id = created_reward_item_id(id);
        let record = native.item_policy_for_source_id(id)?.record();
        let output = output_definition(native, &record.production_binding.target)?;
        if record.admission.stack_class == NativeItemStackClass::NonStackable {
            // Lua addItem(id, count) creates separate instances for nonstackable
            // rewards; a durable quantity >1 on those items would be invalid.
            for _ in 0..count {
                outputs.push((output.clone(), 1));
            }
        } else if record.admission.stack_class == NativeItemStackClass::StackCapable {
            outputs.push((output, count));
        } else {
            return None;
        }
    }
    Some(CookingConversion {
        recipe: format!("oteryn:event-container.{}", rule.item_id),
        outputs,
    })
}

/// Crystal anniversary_reward_boxes.lua:70 calls remove() without a count.
/// ItemFunctions::luaItemRemove:274-279 defaults to -1; Game::internalRemoveItem:
/// 2605-2607 resolves -1 to the entire stack. Other admitted bags call remove(1).
pub fn consumes_whole_stack(recipe: &str) -> bool {
    matches!(
        recipe,
        "oteryn:event-container.39704"
            | "oteryn:event-container.39705"
            | "oteryn:event-container.39706"
            | "oteryn:event-container.39710"
    )
}

/// A source-listed container reward is minted empty by the existing craft writer.
/// Its native policy still decides stack, weight and custody legality.
pub fn permits_empty_container(recipe: &str, output_key: &str) -> bool {
    let Some(source) = recipe.strip_prefix("oteryn:event-container.") else {
        return false;
    };
    let Some(id) = output_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    event_container_rule(&format!("oteryn:item.tibia.i{source}"))
        .is_some_and(|rule| rule.rewards.iter().any(|reward| reward.item_id == id))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn rule(id: u32) -> &'static EventContainerRule {
        event_container_rules()
            .expect("captured catalogue")
            .iter()
            .find(|rule| rule.item_id == id)
            .expect("source rule")
    }

    #[test]
    fn reward_bag_and_event_container_routes_have_disjoint_owners() {
        for bag in super::super::reward_bags().expect("reward bags") {
            if bag.playable {
                assert!(
                    event_container_rule(&bag.bag).is_none(),
                    "dual owner: {}",
                    bag.bag
                );
            }
        }
    }

    #[test]
    fn w19_catalogue_covers_only_the_requested_container_families() {
        let ids: Vec<_> = event_container_rules()
            .expect("catalogue")
            .iter()
            .filter(|rule| rule.selection != Selection::Weighted || rule.item_id == 12045)
            .map(|rule| rule.item_id)
            .collect();
        assert_eq!(
            ids,
            [
                6506, 6507, 6508, 6570, 6571, 8853, 12045, 14751, 14759, 37457, 37461, 37462,
                37463, 37465, 37466, 37467, 37468, 37469, 37614, 39704, 39705, 39706, 39710,
            ]
        );
        assert!(event_container_rule("oteryn:item.tibia.i12045").is_none());
        assert!(event_container_rule("oteryn:item.tibia.i21203").is_none());
        assert!(event_container_rule("oteryn:item.tibia.i3600").is_none());
        assert!(event_container_rule("oteryn:creature.tibia.i6570").is_none());
    }

    #[test]
    fn w19_presents_reach_all_source_gifts_and_inclusive_quantity_endpoints() {
        for (id, count) in [(6570, 13), (6571, 17), (8853, 11), (14751, 15), (14759, 11)] {
            let rule = rule(id);
            assert_eq!(rule.rewards.len(), count);
            let mut reached = BTreeSet::new();
            for draw in 0..4096 {
                let selected = selected_rewards(rule, draw).expect("valid present");
                assert_eq!(selected.len(), 1);
                reached.insert(selected[0]);
            }
            for reward in &rule.rewards {
                assert!(reached.contains(&(reward.item_id, reward.minimum)));
                assert!(reached.contains(&(reward.item_id, reward.maximum)));
            }
        }
    }

    #[test]
    fn w19_christmas_bundles_choose_seven_distinct_fixed_count_gifts_per_use() {
        for id in [6506, 6507, 6508] {
            let rule = rule(id);
            for draw in 0..128 {
                let selected = selected_rewards(rule, draw).expect("valid bundle");
                assert_eq!(selected.len(), 7);
                assert_eq!(
                    selected.iter().map(|r| r.0).collect::<BTreeSet<_>>().len(),
                    7
                );
                for &(id, count) in &selected {
                    let reward = rule
                        .rewards
                        .iter()
                        .find(|r| r.item_id == id)
                        .expect("source gift");
                    assert_eq!(count, reward.maximum);
                    assert_eq!(reward.minimum, reward.maximum);
                }
                assert_eq!(selected_rewards(rule, draw), Some(selected));
            }
            assert_eq!(
                rule.rewards.len(),
                10,
                "using a bundle must not exhaust the shared table"
            );
        }
    }

    #[test]
    fn w19_anniversary_boxes_grant_the_complete_source_bundle() {
        for (id, rows, total) in [
            (39704, 8, 8),
            (39705, 13, 24),
            (39706, 10, 10),
            (39710, 14, 25),
        ] {
            let selected = selected_rewards(rule(id), 0).expect("anniversary bundle");
            assert_eq!(selected.len(), rows);
            assert_eq!(selected.iter().map(|entry| entry.1).sum::<u32>(), total);
            assert_eq!(selected_rewards(rule(id), u64::MAX), Some(selected));
        }
        assert_eq!(
            selected_rewards(rule(39706), 5).expect("balloons"),
            (39671..=39680).map(|id| (id, 1)).collect::<Vec<_>>()
        );
        assert_eq!(
            selected_rewards(rule(39710), 5).expect("big box")[0],
            (39693, 1)
        );
    }

    #[test]
    fn w19_special_boxes_draw_exactly_one_source_reward() {
        for (id, count) in [
            (37457, 5),
            (37461, 14),
            (37462, 6),
            (37463, 3),
            (37465, 10),
            (37466, 3),
            (37467, 16),
            (37468, 11),
            (37469, 11),
            (37614, 4),
        ] {
            let rule = rule(id);
            assert_eq!(rule.rewards.len(), count);
            let mut reached = BTreeSet::new();
            for draw in 0..1024 {
                let selected = selected_rewards(rule, draw).expect("special box");
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].1, 1);
                reached.insert(selected[0].0);
            }
            assert_eq!(reached, rule.rewards.iter().map(|r| r.item_id).collect());
            assert_eq!(rule.ground_output, "source");
            assert_eq!(rule.presentation.cue.effect_id, 203);
        }
    }

    #[test]
    fn w19_music_box_uses_100_source_rolls_without_normalizing_the_tail() {
        let music = event_container_rules()
            .expect("catalogue")
            .iter()
            .find(|rule| rule.item_id == 12045)
            .expect("captured music box");
        assert_eq!(music.roll_max, Some(100));
        let mut histogram = std::collections::BTreeMap::new();
        for roll in 0..100 {
            let selected = selected_rewards_with(music, |domain, upper| {
                if domain == 0 {
                    assert_eq!(upper, 100);
                    Some(roll)
                } else {
                    Some(0)
                }
            })
            .expect("source roll");
            assert_eq!(selected.len(), 1);
            *histogram.entry(selected[0]).or_insert(0) += 1;
        }
        assert_eq!(
            histogram,
            std::collections::BTreeMap::from([
                ((3246, 1), 1),
                ((10227, 1), 2),
                ((11588, 1), 3),
                ((3549, 1), 4),
                ((3420, 1), 5),
                ((3043, 17), 10),
                ((3031, 1), 20),
                ((3043, 1), 30),
                ((3031, 50), 25),
            ])
        );
        // The donor's last two rows are beyond the cumulative 100 cap.
        assert!(!histogram.contains_key(&(3035, 15)));
        assert!(!histogram.contains_key(&(3031, 80)));
        assert!(selected_rewards_with(music, |_, upper| Some(upper)).is_none());
    }

    #[test]
    fn w19_music_box_applies_the_donor_item_factory_soft_boots_alias() {
        assert_eq!(created_reward_item_id(3549), 6529);
        for id in [3246, 10227, 11588, 3420, 3043, 3031, 3035, 6529] {
            assert_eq!(created_reward_item_id(id), id);
        }
    }

    #[test]
    fn w19_invalid_reward_tables_and_out_of_range_draws_refuse_without_a_plan() {
        let present = rule(6570);
        assert!(selected_rewards_with(present, |_, upper| Some(upper)).is_none());
        let bundle = rule(6506);
        assert!(selected_rewards_with(bundle, |_, upper| Some(upper)).is_none());
        let mut broken = present.clone();
        broken.picks = 0;
        assert!(selected_rewards(&broken, 0).is_none());
        broken = present.clone();
        broken.rewards[0].minimum = 0;
        assert!(selected_rewards(&broken, 0).is_none());
        broken = present.clone();
        broken.rewards[0].maximum = broken.rewards[0].minimum - 1;
        assert!(selected_rewards(&broken, 0).is_none());
        broken = present.clone();
        broken.rewards[0].item_id = 0;
        assert!(selected_rewards(&broken, 0).is_none());
        broken = present.clone();
        broken.rewards[0].weight = 0;
        assert!(selected_rewards(&broken, 0).is_none());
        broken = present.clone();
        broken.roll_max = Some(0);
        assert!(selected_rewards(&broken, 0).is_none());
    }

    #[test]
    fn w19_anniversary_remove_without_count_consumes_the_whole_source_stack() {
        for id in [39704, 39705, 39706, 39710] {
            assert!(consumes_whole_stack(&format!(
                "oteryn:event-container.{id}"
            )));
        }
        for recipe in [
            "oteryn:event-container.37457",
            "oteryn:event-container.6506",
            "oteryn:event-container.6570",
            "oteryn:cooking.any",
        ] {
            assert!(!consumes_whole_stack(recipe));
        }
    }

    #[test]
    fn w19_empty_container_permission_is_bound_to_the_source_table() {
        assert!(!permits_empty_container(
            "oteryn:event-container.39710",
            "oteryn:item.tibia.i39693"
        ));
        assert!(super::super::is_reward_bag_output(
            "oteryn:reward-bag.event_39710",
            "oteryn:item.tibia.i39693"
        ));
        assert!(!permits_empty_container(
            "oteryn:event-container.39704",
            "oteryn:item.tibia.i39693"
        ));
        assert!(!permits_empty_container(
            "oteryn:cooking.any",
            "oteryn:item.tibia.i39693"
        ));
        assert!(!permits_empty_container(
            "oteryn:event-container.39710",
            "oteryn:item.tibia.i2854"
        ));
    }
}
