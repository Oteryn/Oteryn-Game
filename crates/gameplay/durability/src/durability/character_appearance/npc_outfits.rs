//! NPC source snapshots and grants share the account appearance owner.
use super::*;
use crate::content::native_gameplay::{NativeCosmeticsView, NativeGameplayState};
use crate::domain::appearance::{AccountUnlocks, EarnedCosmetics, UnlockFact};
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
use oteryn_gameplay_quests::quest::reward_callbacks::{CreationSex, OutfitReward};
use std::collections::BTreeMap;

/// Interpretation data only. The source writer reloads it under the Character
/// and account locks before deciding or consuming any hand-in.
pub fn snapshot(content: &NativeCosmeticsView, facts: &[(String, String)]) -> BTreeMap<u16, u8> {
    let mut result = BTreeMap::new();
    for sex in [Sex::Female, Sex::Male] {
        let earned = EarnedCosmetics { content, facts }.with_sex(sex);
        for key in content.outfit_keys() {
            let key = key.to_owned();
            if !content.outfit_is_starter(&key, sex)
                && earned.outfit(&key) != UnlockFact::Compatible
            {
                continue;
            }
            let Some(look) = content
                .look_type(&key, sex)
                .and_then(|look| u16::try_from(look).ok())
            else {
                continue;
            };
            let mut addons = 0;
            for addon in 1..=content.outfit_addon_count(&key).unwrap_or(0) {
                if content.addon_is_starter(&key, sex, addon)
                    || earned.addon(&key, addon) == UnlockFact::Compatible
                {
                    addons |= 1 << (addon - 1);
                }
            }
            *result.entry(look).or_default() |= addons;
        }
    }
    result
}

pub(crate) async fn locked_snapshot(
    tx: &mut sqlx::PgConnection,
    character: CharacterId,
    native: &NativeGameplayState,
) -> Result<BTreeMap<u16, u8>, DurabilityError> {
    let content = native
        .cosmetics()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut *tx)
    .await?;
    lock_account_grants(tx, account).await?;
    let facts = earned_unlocks::read(tx, character, &content.earned_grant_keys())
        .await
        .map_err(read_storage_error)?;
    Ok(snapshot(content, &facts))
}

/// Resolve exact source look IDs to admitted definitions, including each sex.
/// Unknown looks, addon bits or missing mappings refuse the whole hand-in.
pub(crate) fn rewards(
    native: &NativeGameplayState,
    effects: &[NpcRuleEffect],
) -> Result<Vec<OutfitReward>, DurabilityError> {
    let content = native
        .cosmetics()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut rewards = BTreeMap::<(String, u8), u8>::new();
    for effect in effects {
        let (look, addons) = match effect {
            NpcRuleEffect::Outfit { look_type, addon }
            | NpcRuleEffect::OutfitAddon { look_type, addon } => (*look_type, *addon),
            _ => continue,
        };
        if addons > 3 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut found = false;
        let mut identity: Option<&str> = None;
        for (sex, tag) in [(Sex::Female, 0), (Sex::Male, 1)] {
            for key in content.outfit_keys() {
                if content.look_type(key, sex) == Some(u32::from(look)) {
                    // A shared look is not grant authority. Only definitions with
                    // explicit compatible quest mappings can name a new grant.
                    let candidate = OutfitReward {
                        outfit: key.to_owned(),
                        addons,
                        sex: Some(if tag == 0 { CreationSex::Female } else { CreationSex::Male }),
                    };
                    let neutral = OutfitReward { sex: None, ..candidate.clone() };
                    if !super::quest_grants::outfit_compatible(native, &candidate)
                        && !super::quest_grants::outfit_compatible(native, &neutral)
                    {
                        continue;
                    }
                    if identity.is_some_and(|identity| identity != key) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    identity = Some(key);
                    *rewards.entry((key.to_owned(), tag)).or_default() |= addons;
                    found = true;
                }
            }
        }
        if !found {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let mut paired = BTreeMap::<String, [Option<u8>; 2]>::new();
    for ((outfit, sex), addons) in rewards {
        paired.entry(outfit).or_default()[usize::from(sex)] = Some(addons);
    }
    let mut result = Vec::new();
    for (outfit, masks) in paired {
        if let [Some(female), Some(male)] = masks
            && female == male
        {
            result.push(OutfitReward {
                outfit,
                addons: female,
                sex: None,
            });
        } else {
            for (sex, addons) in masks.into_iter().enumerate() {
                if let Some(addons) = addons {
                    result.push(OutfitReward {
                        outfit: outfit.clone(),
                        addons,
                        sex: Some(if sex == 0 {
                            CreationSex::Female
                        } else {
                            CreationSex::Male
                        }),
                    });
                }
            }
        }
    }
    // Native authored mappings remain the authority. A missing or sex-incompatible
    // mapping refuses the entire source plan before any carried input is consumed.
    if result
        .iter()
        .any(|reward| !super::quest_grants::outfit_compatible(native, reward))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(result)
}
