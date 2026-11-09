//! Read-only cards from the same active native generation used by combat and loot.
use super::*;
use crate::content::{
    ProjectV2AuthoringProfileData, ReferenceItemField::Known, native_gameplay::NativeGameplayState,
};
use oteryn_protocol_oteryn::charm_progression::catalogue::{self as wire, Card, Page, Request};
use std::{collections::BTreeSet, fmt::Write};

pub(super) async fn dispatch(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payload: &[u8],
) -> GuildReply {
    let Ok(request) = wire::decode_request(payload) else {
        return GuildReply::Rejected;
    };
    let Some(current) =
        super::super::character_gameplay_fence::current(owner, admitted.game_session_id, false)
            .await
    else {
        return GuildReply::Rejected;
    };
    if Some(current.item_fence()) != admitted.item_fence {
        return GuildReply::Rejected;
    }
    let runtime = owner.runtime.lock().await;
    if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted) {
        return GuildReply::Rejected;
    }
    if super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation).is_none() {
        return GuildReply::Rejected;
    }
    drop(runtime);
    let Some(native) = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return GuildReply::Rejected;
    };
    project(native, request)
        .and_then(|page| wire::encode_page(&page).ok())
        .map_or(GuildReply::Rejected, GuildReply::Payload)
}
fn category<'a>(native: &'a NativeGameplayState, key: &str) -> Option<&'a str> {
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == key)?;
    let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
        return None;
    };
    Some(&profile.details.as_ref()?.bestiary.as_ref()?.class)
}
fn card(native: &NativeGameplayState, race: &BestiaryRace, ordinal: u16) -> Option<Card> {
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|p| p.definition_key == race.key())?;
    Some(Card {
        race: ordinal,
        name: policy.display_name.clone(),
        category: category(native, race.key())?.into(),
        detail: String::new(),
    })
}
fn project(native: &NativeGameplayState, request: Request) -> Option<Page> {
    let races = super::canonical_races()?;
    let categories: Vec<_> = races
        .iter()
        .filter_map(|r| category(native, r.key()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_owned)
        .collect();
    if usize::from(request.category) > categories.len() {
        return None;
    }
    if request.race != 0 {
        let race = races.get(usize::from(request.race - 1))?;
        let mut card = card(native, race, request.race)?;
        let (detail, next) = details(native, race, request.after)?;
        card.detail = detail;
        return Some(Page {
            next,
            categories,
            cards: vec![card],
        });
    }
    let selected = request
        .category
        .checked_sub(1)
        .and_then(|i| categories.get(usize::from(i)));
    let mut cards: Vec<_> = races
        .iter()
        .enumerate()
        .skip(usize::from(request.after))
        .filter_map(|(i, race)| {
            let card = card(native, race, (i + 1).try_into().ok()?)?;
            selected
                .is_none_or(|category| *category == card.category)
                .then_some(card)
        })
        .take(wire::PAGE_SIZE + 1)
        .collect();
    let more = cards.len() > wire::PAGE_SIZE;
    cards.truncate(wire::PAGE_SIZE);
    let next = more.then(|| cards.last().map(|card| card.race)).flatten();
    Some(Page {
        next,
        categories,
        cards,
    })
}
fn details(
    native: &NativeGameplayState,
    race: &BestiaryRace,
    after: u16,
) -> Option<(String, Option<u16>)> {
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|p| p.definition_key == race.key())?;
    let reward = native.reward_table().row(race.key()).ok();
    let mut text = format!(
        "Health: {}\r\nSpeed: {}\r\nDiscovery thresholds: {:?}\r\n",
        policy.maximum_health,
        policy.base_speed,
        race.kill_thresholds()
    );
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|r| r.profile.target.key == race.key())?;
    if let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
        && let Some(experience) = profile.experience
    {
        let _ = writeln!(text, "Experience: {experience}\r");
    }
    if let Some(armor) = policy.armor {
        let _ = writeln!(text, "Armor: {armor}\r");
    }
    if let Some(defense) = policy.defense {
        let _ = writeln!(text, "Defense: {defense}\r");
    }
    for resistance in &policy.resistances {
        let _ = writeln!(
            text,
            "Resistance {}: {}/{}%\r",
            resistance.damage_type, resistance.percent.numerator, resistance.percent.denominator
        );
    }
    for response in &policy.healing_from_damage {
        let _ = writeln!(
            text,
            "Healed by {}: {}/{}%\r",
            response.damage_type, response.percent.numerator, response.percent.denominator
        );
    }
    for immunity in &policy.condition_immunities {
        let _ = writeln!(text, "Condition immunity: {immunity:?}\r");
    }
    for immunity in &policy.damage_immunities {
        let _ = writeln!(text, "Immune to {immunity}\r");
    }
    text.push_str("Loot (base content chances, before session modifiers):\r\n");
    let Some(reward) = reward else {
        text.push_str("No admitted loot table\r\n");
        return Some((text, None));
    };
    let mut stack: Vec<_> = reward
        .loot_table
        .entries
        .iter()
        .rev()
        .map(|entry| (entry, 0usize))
        .collect();
    let mut index = 0usize;
    let start = usize::from(after);
    let end = start + wire::PAGE_SIZE;
    while let Some((entry, depth)) = stack.pop() {
        if index >= end {
            return Some((text, Some(index.try_into().ok()?)));
        }
        if index >= start {
            let name = native
                .item_policy(&entry.item.production_key, &entry.item.revision_ref)
                .and_then(|p| {
                    let Known(presentation) = &p.record().semantics.presentation else {
                        return None;
                    };
                    let Known(name) = &presentation.name else {
                        return None;
                    };
                    Some(name.as_str())
                })
                .unwrap_or(&entry.item.production_key);
            let chance = entry
                .probability_ppm
                .map_or_else(|| "table selection".into(), |n| format!("{n} ppm"));
            let _ = writeln!(
                text,
                "{}{name}: {}-{}, {chance}{}\r",
                "  ".repeat(depth),
                entry.min_count,
                entry.max_count,
                if depth > 0 {
                    " (conditional on parent)"
                } else {
                    ""
                }
            );
        }
        index += 1;
        stack.extend(
            entry
                .details
                .children
                .iter()
                .rev()
                .map(|entry| (entry, depth + 1)),
        );
    }
    if start > index {
        return None;
    }
    if index == 0 {
        text.push_str("No loot entries\r\n");
    }
    Some((text, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bestiary_catalogue_projects_active_creature_categories_stats_and_loot()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let mut id = [1; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                let world =
                    crate::foundation::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
                let (controller, _) =
                    crate::content::native_gameplay::test_support::activated_with_loot_tables(
                        world, None,
                    );
                let native = controller
                    .active()
                    .and_then(|a| a.native_gameplay())
                    .ok_or("native fixture")?;
                let page = project(
                    native,
                    Request {
                        after: 0,
                        race: 0,
                        category: 0,
                    },
                )
                .ok_or("catalogue")?;
                let first = page.cards.first().ok_or("creature cards")?;
                let detail = project(
                    native,
                    Request {
                        after: 0,
                        race: first.race,
                        category: 0,
                    },
                )
                .ok_or("detail")?;
                assert!(detail.cards[0].detail.contains("Health:"));
                assert!(detail.cards[0].detail.contains("Loot"));
                assert!(!page.categories.is_empty());
                let filtered = project(
                    native,
                    Request {
                        after: 0,
                        race: 0,
                        category: 1,
                    },
                )
                .ok_or("category filter")?;
                assert!(
                    filtered
                        .cards
                        .iter()
                        .all(|card| card.category == page.categories[0])
                );
                let bytes = wire::encode_page(&detail).map_err(|e| format!("{e:?}"))?;
                assert_eq!(
                    wire::decode_page(&bytes).map_err(|e| format!("{e:?}"))?,
                    detail
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "catalogue test thread panicked")?
    }
}
