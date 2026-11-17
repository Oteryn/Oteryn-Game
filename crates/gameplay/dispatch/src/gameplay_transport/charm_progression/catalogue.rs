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
    let named_races = if payload.starts_with(&[255,3]) && (4..=12).contains(&payload.len()) && payload.len().is_multiple_of(2) {
        let races:Vec<_>=payload[2..].as_chunks::<2>().0.iter().map(|b|u16::from_le_bytes(*b)).collect();
        if races.iter().any(|r|!(1..=1024).contains(r)) || races.windows(2).any(|p|p[0]>=p[1]) { return GuildReply::Rejected; }
        Some(races)
    } else { None };
    let charm_after = match payload { [255, 2, after] if *after <= 32 => Some(*after), _ => None };
    let metadata_after = if payload.first() == Some(&254) { match wire::decode_charm_request(payload) { Ok(after) => Some(after), Err(_) => return GuildReply::Rejected } } else { None };
    let request = if metadata_after.is_some() || charm_after.is_some() || named_races.is_some() { Request { after: 0, race: 0, category: 0 } } else {
        let Ok(request) = wire::decode_request(payload) else { return GuildReply::Rejected; };
        request
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
    if let Some(after) = metadata_after {
        return super::metadata::project(owner.imported_charms, after)
            .and_then(|page| wire::encode_page(&page).ok())
            .map_or(GuildReply::Rejected, GuildReply::Payload);
    }
    let Some(native) = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return GuildReply::Rejected;
    };
    if let Some(ids) = named_races {
        let Some(races) = super::canonical_races() else { return GuildReply::Rejected; };
        let cards:Option<Vec<_>>=ids.into_iter().map(|id|card(native,races.get(usize::from(id-1))?,id)).collect();
        return cards.and_then(|cards| { let categories=cards.iter().map(|c|c.category.clone()).collect::<BTreeSet<_>>().into_iter().collect(); wire::encode_page(&Page {next:None,cards,categories}).ok() }).map_or(GuildReply::Rejected,GuildReply::Payload);
    }
    if let Some(after) = charm_after {
        return super::charm_details::page(owner.imported_charms, after).and_then(|page| wire::encode_page(&page).ok()).map_or(GuildReply::Rejected, GuildReply::Payload);
    }
    let Some(page) = project(native, request) else { return GuildReply::Rejected; };
    if payload.len() == 8 {
        owner.echo_warden_bestiary_page(admitted, page).await
    } else {
        wire::encode_page(&page).map_or(GuildReply::Rejected, GuildReply::Payload)
    }
}
fn category<'a>(native: &'a NativeGameplayState, key: &str) -> &'a str {
    native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == key)
        .and_then(|record| match &record.profile.data {
            ProjectV2AuthoringProfileData::Creature(profile) => {
                profile.details.as_ref()?.bestiary.as_ref()
            }
            _ => None,
        })
        .map_or("Unclassified", |bestiary| bestiary.class.as_str())
}

fn card(native: &NativeGameplayState, race: &BestiaryRace, ordinal: u16) -> Option<Card> {
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|p| p.definition_key == race.key())?;
    Some(Card { echo_warden_completed: None,
        outfit_look_type: policy.outfit_look_type,
        object_look_type: policy.object_look_type,
        race: ordinal,
        name: policy.display_name.clone(),
        category: category(native, race.key()).into(),
        detail: String::new(),
    })
}
fn project(native: &NativeGameplayState, request: Request) -> Option<Page> {
    let races = super::canonical_races()?;
    let categories: Vec<_> = races
        .iter()
        .map(|r| category(native, r.key()))
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
                let race_key = super::canonical_races().ok_or("races")?[usize::from(first.race - 1)].key();
                let policy = native.policies().policies().iter().find(|p| p.definition_key == race_key).ok_or("policy")?;
                assert_eq!(detail.cards[0].outfit_look_type, policy.outfit_look_type);
                assert_eq!(detail.cards[0].object_look_type, policy.object_look_type);
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
                let mut after = 0;
                let mut seen = BTreeSet::new();
                loop {
                    let page = project(
                        native,
                        Request {
                            after,
                            race: 0,
                            category: 0,
                        },
                    )
                    .ok_or("full catalogue page")?;
                    wire::encode_page(&page).map_err(|e| format!("{e:?}"))?;
                    for card in page.cards {
                        assert!(seen.insert(card.race));
                    }
                    let Some(next) = page.next else { break };
                    assert!(next > after);
                    after = next;
                }
                assert_eq!(
                    seen.len(),
                    super::super::canonical_races()
                        .ok_or("canonical races")?
                        .len()
                );
                Ok(())
            })?
            .join()
            .map_err(|_| "catalogue test thread panicked")?
    }
}
