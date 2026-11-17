//! TibiaWiki achievement pages, spoiler fields (fetched 2026-11-17), accepted
//! by the FULLOUT owner. Global behaviour wins over NPC-specific donor grants.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8
//! data-global/npc/lynda.lua:94-95 grants female Summoner and male Mage addons.
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::CharacterId;
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetics, Sex, UnlockFact,
};
use crate::durability::DurabilityError;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

// Outfit, male achievement, female achievement. Registry identities are preserved.
const OUTFITS: [(&str, &str, &str); 13] = [
    ("barbarian", "brutal_politeness", "brutal_politeness"),
    ("citizen", "exemplary_citizen", "exemplary_citizen"),
    ("falconer", "falconer", "falconer"),
    ("feral_trapper", "feral_trapper", "feral_trapper"),
    ("mage", "alumni", "ritualist"),
    ("norseman", "out_in_the_snowstorm", "out_in_the_snowstorm"),
    ("pirate", "swashbuckler", "swashbuckler"),
    (
        "poltergeist",
        "mainstreet_nightmare",
        "mainstreet_nightmare",
    ),
    ("rascoohan", "honorary_rascoohan", "honorary_rascoohan"),
    ("shaman", "way_of_the_shaman", "way_of_the_shaman"),
    ("summoner", "ritualist", "alumni"),
    ("warrior", "wild_warrior", "wild_warrior"),
    ("wizard", "warlock", "warlock"),
];

/// Called after addon facts are inserted under the source's Character fence and
/// account grant lock. Resolve both addons through admitted mappings, so different
/// sources can complete one outfit without trusting numeric looks or grant keys.
pub(crate) async fn evaluate(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    content: &NativeCosmeticsView,
    outfit: &String,
    event: &[u8],
) -> Result<(), DurabilityError> {
    let Some((name, male, female)) = OUTFITS.iter().find(|(name, _, _)| {
        outfit == &format!("oteryn:outfit.{name}")
            || outfit == &format!("oteryn:content.outfit.{name}_outfits")
    }) else {
        return Ok(());
    };
    let outfit = format!("oteryn:outfit.{name}");
    let sex = super::item_action_sex(tx, character).await?;
    let facts = super::earned_unlocks::read(tx, character, &content.earned_grant_keys())
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let earned = EarnedCosmetics {
        content,
        facts: &facts,
    }
    .with_sex(sex);
    if (1..=2).all(|addon| earned.addon(&outfit, addon) == UnlockFact::Compatible) {
        let achievement = format!(
            "oteryn:achievement/{}",
            match sex {
                Sex::Male => male,
                Sex::Female => female,
            }
        );
        super::super::account_achievement::grant_missing(
            tx,
            character,
            &[&achievement],
            "full_outfit",
            &Sha256::digest(event),
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn full_outfit_table_has_thirteen_rows_and_registered_achievements()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_progression::achievement_catalogue::{SHARDS, parse_shards};
        let registry = parse_shards(&SHARDS).map_err(|error| format!("{error:?}"))?;
        assert_eq!(super::OUTFITS.len(), 13);
        for (_, male, female) in super::OUTFITS {
            for slug in [male, female] {
                assert!(
                    registry.contains_key(&format!("oteryn:achievement/{slug}")),
                    "{slug}"
                );
            }
        }
        Ok(())
    }
}
