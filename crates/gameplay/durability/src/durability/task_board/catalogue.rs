use super::*;
use oteryn_gameplay_content::content::{
    ProjectV2AuthoringProfileData, native_gameplay::NativeGameplayState,
};

/// The canonical native creature corpus supplies eligibility and XP; reference-only
/// identities are not client-selectable Task Board candidates.
pub(super) fn creatures(native: &NativeGameplayState) -> Vec<TaskBoardCreature> {
    let mut creatures = Vec::new();
    for record in &native.creature_profiles().records {
        let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
            continue;
        };
        let Some(details) = &profile.details else {
            continue;
        };
        if profile.bestiary.is_none() || !record.profile.target.key.starts_with("oteryn:") {
            continue;
        }
        let stars = details
            .bestiary
            .as_ref()
            .and_then(|value| value.stars)
            .or_else(|| {
                match profile
                    .bestiary
                    .as_ref()
                    .map(|value| value.difficulty.as_str())
                {
                    Some("harmless") => Some(0),
                    Some("trivial") => Some(1),
                    Some("easy") => Some(2),
                    Some("medium") => Some(3),
                    Some("hard") => Some(4),
                    Some("challenging") => Some(5),
                    _ => None,
                }
            });
        let Some(stars) = stars else { continue };
        creatures.push(TaskBoardCreature {
            race: record.profile.target.key.clone(),
            bestiary_stars: stars,
            experience: profile.experience.unwrap_or(0),
            preyable: details.system_eligibility.prey,
            prey_exclusive: details.system_eligibility.exclusive_prey,
        });
    }
    creatures.sort_by(|left, right| left.race.cmp(&right.race));
    creatures
}

impl TryFrom<&NativeGameplayState> for TaskBoardCatalogue {
    type Error = &'static str;
    fn try_from(native: &NativeGameplayState) -> Result<Self, Self::Error> {
        use oteryn_gameplay_content::content::native_gameplay::{
            NativeItemDestination, NativeItemStackClass,
        };
        use oteryn_gameplay_quests::quest::task_board::full::{
            DELIVERY_ITEMS, SHOP_OFFERS, TaskBoardShopKind,
        };
        let creatures = creatures(native);
        if creatures.is_empty() {
            return Err("Task Board creature catalogue unavailable");
        }
        let index = native
            .item_index()
            .ok_or("Task Board item index unavailable")?;
        let mut ids = std::collections::BTreeSet::from([51_978_u16]);
        ids.extend(DELIVERY_ITEMS.iter().map(|item| item.item_id));
        for offer in SHOP_OFFERS {
            if matches!(
                offer.kind,
                TaskBoardShopKind::Item | TaskBoardShopKind::ItemDouble
            ) {
                ids.insert(offer.appearance_or_item);
                ids.extend(offer.second_item);
            }
        }
        let mut items = BTreeMap::new();
        for id in ids {
            let Some(policy) = native.item_policy_for_source_id(u32::from(id)) else {
                continue;
            };
            let record = policy.record();
            let definition = &record.production_definition;
            if !record.admission.materializable
                || !record
                    .admission
                    .legal_destinations
                    .contains(&NativeItemDestination::CharacterInventory)
                || index
                    .definition_ref(
                        &definition.family,
                        &definition.production_key,
                        &definition.revision_ref,
                    )
                    .is_none()
            {
                continue;
            }
            let definition_ref = super::super::item_mint::TypedDefinitionRef {
                family: definition.family.clone(),
                production_key: definition.production_key.clone(),
                revision_ref: definition.revision_ref.clone(),
            };
            let mint = oteryn_gameplay_items::item_ref_admission::generation_item_facts(
                native,
                &definition_ref,
            )
            .ok();
            items.insert(
                id,
                TaskBoardItem {
                    key: definition.production_key.clone(),
                    revision: definition.revision_ref.clone(),
                    stackable: record.admission.stack_class == NativeItemStackClass::StackCapable,
                    mint,
                },
            );
        }
        Ok(Self {
            creatures,
            items,
            content_revision: native
                .cosmetics()
                .map(|view| view.content_revision().to_owned())
                .unwrap_or_default(),
            cosmetics: native.cosmetics().cloned(),
            gameplay_content_revision: String::new(),
            progression: native.progression().cloned(),
        })
    }
}
