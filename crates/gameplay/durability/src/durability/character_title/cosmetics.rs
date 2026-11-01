//! P12a interprets existing earned appearance receipts through the active content owner.
//! Neither an opaque grant key nor a stored selection is proof of owning an outfit or mount.
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetics, Sex, UnlockFact,
};
use crate::domain::{AccountId, CharacterId};
use crate::durability::DurabilityError;
use crate::durability::character_appearance::{
    AppearanceReadError, read_account_appearance_unlocks,
};
use oteryn_protocol_oteryn::character_info::FullOutfit;
use std::collections::BTreeSet;

pub(super) struct Source {
    content: NativeCosmeticsView,
    grant_keys: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Facts {
    pub mounts: u32,
    pub outfits: u32,
    pub full_outfits: [bool; 3],
}

impl Source {
    /// Prepare all content input before entering a semantic transaction.
    pub(super) fn new(content: NativeCosmeticsView) -> Self {
        Self {
            grant_keys: content.earned_grant_keys(),
            content,
        }
    }

    pub(super) async fn read(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
    ) -> Result<Facts, DurabilityError> {
        let account: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        let account = AccountId::from_bytes(*account.as_bytes())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let facts: Vec<_> = read_account_appearance_unlocks(tx, account, &self.grant_keys)
            .await
            .map_err(|error| match error {
                AppearanceReadError::Storage(error) => DurabilityError::from(error),
                AppearanceReadError::InvalidStoredState => DurabilityError::InvalidStoredState,
            })?
            .into_iter()
            // Match the appearance owner's earned-unlock seam. Store does not authorize.
            .filter(|fact| matches!(fact.source_kind.as_str(), "quest" | "npc" | "mount_taming"))
            .map(|fact| (fact.unlock_key, fact.content_revision))
            .collect();
        let outfits = self.content.outfit_keys().filter_map(|key| {
            Some((
                key,
                [
                    self.content.look_type(key, Sex::Female)?,
                    self.content.look_type(key, Sex::Male)?,
                ],
            ))
        });
        Ok(project(
            &self.content,
            &facts,
            outfits,
            self.content.mount_keys(),
        ))
    }
}

fn project<'a, C: AppearanceContent<String>>(
    content: &C,
    facts: &[(String, String)],
    outfits: impl Iterator<Item = (&'a str, [u32; 2])>,
    mounts: impl Iterator<Item = &'a str>,
) -> Facts {
    let unlocks = EarnedCosmetics { content, facts };
    let mut full_looks = BTreeSet::new();
    let mut owned_outfits = BTreeSet::new();
    for (key, looks) in outfits {
        let key = key.to_owned();
        // PR #2782 Q3=B: count each earned canonical identity once. Crystal's
        // player_functions.cpp:3306-3311 / player.cpp:6840-6850 insert an outfitsMap entry
        // for an earned addon too. Native starter content alone is not an earned grant.
        let base_granted = unlocks.outfit(&key) == UnlockFact::Compatible;
        let addon_count = content.outfit_addon_count(&key).unwrap_or(0);
        let starter = [Sex::Female, Sex::Male]
            .into_iter()
            .any(|sex| content.outfit_is_starter(&key, sex));
        let earned_starter_addon = starter
            && (1..=addon_count).any(|addon| unlocks.addon(&key, addon) == UnlockFact::Compatible);
        if base_granted || earned_starter_addon {
            owned_outfits.insert(key.clone());
        }
        if addon_count < 2 {
            continue;
        }
        for (sex, look) in [Sex::Female, Sex::Male].into_iter().zip(looks) {
            let base = content.outfit_is_starter(&key, sex) || base_granted;
            let addons = (1..=2).all(|addon| {
                content.addon_is_starter(&key, sex, addon)
                    || unlocks.addon(&key, addon) == UnlockFact::Compatible
            });
            if base && addons {
                full_looks.insert(look);
            }
        }
    }
    Facts {
        outfits: u32::try_from(owned_outfits.len()).unwrap_or(u32::MAX),
        mounts: u32::try_from(
            mounts
                .filter(|key| {
                    let key = (*key).to_owned();
                    content.mount_exists(&key) && unlocks.mount(&key) == UnlockFact::Compatible
                })
                .count(),
        )
        .unwrap_or(u32::MAX),
        full_outfits: FullOutfit::ALL.map(|outfit| {
            outfit
                .looks()
                .into_iter()
                .all(|look| full_looks.contains(&look))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::appearance::EarnedCosmetic;

    struct Content {
        starter: bool,
    }
    impl AppearanceContent<String> for Content {
        fn palette_size(&self) -> u16 {
            1
        }
        fn outfit_addon_count(&self, _: &String) -> Option<u8> {
            Some(2)
        }
        fn outfit_is_starter(&self, _: &String, _: Sex) -> bool {
            self.starter
        }
        fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
            false
        }
        fn mount_exists(&self, key: &String) -> bool {
            key == "ride"
        }
        fn earned_unlock(
            &self,
            key: &str,
            revision: &str,
            target: EarnedCosmetic<'_>,
        ) -> UnlockFact {
            let matches = matches!(
                (key, target),
                ("quest.outfit", EarnedCosmetic::Outfit("royal"))
                    | ("quest.first", EarnedCosmetic::Addon("royal", 1))
                    | ("quest.second", EarnedCosmetic::Addon("royal", 2))
                    | ("quest.mount", EarnedCosmetic::Mount("ride"))
            );
            match (matches, revision == "admitted") {
                (true, true) => UnlockFact::Compatible,
                (true, false) => UnlockFact::Incompatible,
                _ => UnlockFact::Absent,
            }
        }
    }

    fn facts(keys: &[&str]) -> Vec<(String, String)> {
        keys.iter()
            .map(|key| ((*key).to_owned(), "admitted".to_owned()))
            .collect()
    }

    #[test]
    fn cosmetic_titles_require_admitted_receipts_and_both_exact_looks() {
        let evaluate = |facts: &[(String, String)], looks| {
            project(
                &Content { starter: false },
                facts,
                [("royal", looks)].into_iter(),
                ["ride"].into_iter(),
            )
        };
        assert_eq!(
            evaluate(&facts(&["royal", "ride"]), [1457, 1456]),
            Facts::default(),
            "keys that look like cosmetics are not authored grants"
        );
        let partial = facts(&["quest.outfit", "quest.first", "quest.mount"]);
        assert_eq!(
            evaluate(&partial, [1457, 1456]),
            Facts {
                mounts: 1,
                outfits: 1,
                full_outfits: [false; 3]
            }
        );
        let full = facts(&["quest.outfit", "quest.first", "quest.second", "quest.mount"]);
        assert_eq!(
            evaluate(&full, [1457, 1456]),
            Facts {
                mounts: 1,
                outfits: 1,
                full_outfits: [true, false, false]
            }
        );
        assert_eq!(
            evaluate(&full, [1457, 128]).full_outfits,
            [false; 3],
            "one matching look is insufficient"
        );
        let mut incompatible = full;
        incompatible[2].1 = "retired".to_owned();
        assert_eq!(
            evaluate(&incompatible, [1457, 1456]).full_outfits,
            [false; 3]
        );
    }

    #[test]
    fn topmodel_counts_an_earned_addon_on_an_available_starter_once() {
        let evaluate = |starter, earned: &[(String, String)]| {
            project(
                &Content { starter },
                earned,
                [("royal", [1457, 1456])].into_iter(),
                std::iter::empty(),
            )
        };
        assert_eq!(
            evaluate(true, &[]).outfits,
            0,
            "starter content alone is not earned"
        );
        assert_eq!(
            evaluate(true, &facts(&["quest.first"])).outfits,
            1,
            "Crystal addOutfitAddon creates one outfitsMap identity for an earned addon"
        );
        assert_eq!(
            evaluate(false, &facts(&["quest.first"])).outfits,
            0,
            "an addon cannot authorize an unavailable native base outfit"
        );
        assert_eq!(
            evaluate(
                true,
                &facts(&["quest.first", "quest.second", "quest.outfit"])
            )
            .outfits,
            1,
            "base and two addons, with both sex looks, still count as one canonical identity"
        );
    }
}
