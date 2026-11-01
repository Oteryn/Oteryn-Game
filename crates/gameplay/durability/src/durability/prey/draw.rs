use super::super::DurabilityError;
use oteryn_gameplay_content::content::{
    ProjectV2AuthoringProfileData, native_gameplay::NativeGameplayState,
};
use oteryn_gameplay_progression::prey::{BonusType, PreyBonus, PreyRules};
use sha2::{Digest, Sha256};

/// The active generation's eligible native creatures, in canonical key order.
#[derive(Debug, Clone)]
pub struct PreyCatalogue {
    pub(super) creatures: Vec<(String, u8, bool)>,
    pub(super) rules: PreyRules,
    /// Creatures whose loot table exceeds the PREY-0 R4 budget (`PREY0-RL-16`, 7 entries).
    pub(super) unsupported_loot: std::collections::BTreeSet<String>,
}
impl TryFrom<&NativeGameplayState> for PreyCatalogue {
    type Error = DurabilityError;
    fn try_from(native: &NativeGameplayState) -> Result<Self, Self::Error> {
        let rules = serde_json::from_value(
            native
                .creature_profiles()
                .prey_rules
                .clone()
                .ok_or(DurabilityError::InvalidStoredState)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        Self::with_rules(native, rules)
    }
}
impl PreyCatalogue {
    pub fn rules(&self) -> &PreyRules {
        &self.rules
    }

    pub fn with_rules(
        native: &NativeGameplayState,
        rules: PreyRules,
    ) -> Result<Self, DurabilityError> {
        rules
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut creatures = Vec::new();
        for record in &native.creature_profiles().records {
            let ProjectV2AuthoringProfileData::Creature(value) = &record.profile.data else {
                continue;
            };
            let Some(details) = &value.details else {
                continue;
            };
            if value.bestiary.is_none()
                || value.experience.unwrap_or(0) == 0
                || !details.system_eligibility.prey
                // Reference profiles can be prey-eligible, but the Prey wire contract
                // only admits native Oteryn identities (including for SelectAny).
                || !record.profile.target.key.starts_with("oteryn:")
            {
                continue;
            }
            let tier = rules
                .creatures
                .get(&record.profile.target.key)
                .ok_or(DurabilityError::InvalidStoredState)?
                .tier;
            creatures.push((
                record.profile.target.key.clone(),
                tier,
                details.system_eligibility.exclusive_prey,
            ));
        }
        creatures.sort_by(|a, b| a.0.cmp(&b.0));
        if creatures.iter().filter(|c| !c.2).count() < rules.list_size(false) * 3 {
            return Err(DurabilityError::InvalidStoredState);
        }
        // A creature without a settleable loot row rolls no loot, so it is an empty table
        // within the budget. R4 requires the content compiler to fail closed on oversized
        // preyable tables (`PREY_LOOT_TABLE_TOO_LARGE`); until that check exists this set
        // is the runtime guard.
        let unsupported_loot = creatures
            .iter()
            .filter(|c| {
                native
                    .reward_table()
                    .row(&c.0)
                    .is_ok_and(|r| r.loot_table.entries.len() > 7)
            })
            .map(|c| c.0.clone())
            .collect();
        Ok(Self {
            rules,
            creatures,
            unsupported_loot,
        })
    }
}

pub(super) struct Draw {
    seed: Vec<u8>,
    ordinal: u64,
}
impl Draw {
    pub(super) fn new(secret: &[u8], occurrence: &[u8; 16], purpose: &[u8]) -> Self {
        Self {
            seed: [secret, occurrence.as_slice(), purpose].concat(),
            ordinal: 0,
        }
    }
    fn below(&mut self, n: usize) -> usize {
        let n = n as u64;
        loop {
            let mut hash = Sha256::new();
            hash.update(&self.seed);
            hash.update(self.ordinal.to_be_bytes());
            self.ordinal += 1;
            let bytes: [u8; 8] = hash.finalize()[..8].try_into().unwrap_or([0; 8]);
            let value = u64::from_be_bytes(bytes);
            if value < u64::MAX - u64::MAX % n {
                return (value % n) as usize;
            }
        }
    }
    pub(super) fn list(
        &mut self,
        catalogue: &PreyCatalogue,
        tier: u8,
        size: usize,
        blacklist: &[String],
    ) -> Result<Vec<String>, DurabilityError> {
        if !(1..=5).contains(&tier)
            || size == 0
            || size > oteryn_gameplay_progression::prey::difficulty::MAX_LIST_SIZE
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        // OWNER-DEVIATION PREY-DIFF: chosen tier replaces Bestiary-star quotas.
        // Offered-today keys and all other slots are added by the durable caller.
        let mut pool: Vec<_> = catalogue
            .creatures
            .iter()
            .filter(|c| {
                !c.2 && !blacklist.contains(&c.0) && !catalogue.unsupported_loot.contains(&c.0)
            })
            .collect();
        let mut out = Vec::with_capacity(size.min(pool.len()));
        for _ in 0..size.min(pool.len()) {
            // Keep the PREY-0 fallback: nearest tier down, then up.
            let candidates = (1..=tier)
                .rev()
                .chain(tier + 1..=5)
                .map(|band| {
                    pool.iter()
                        .enumerate()
                        .filter(|(_, c)| c.1 == band)
                        .map(|(index, _)| index)
                        .collect::<Vec<_>>()
                })
                .find(|c| !c.is_empty())
                .ok_or(DurabilityError::InvalidStoredState)?;
            let index = candidates[self.below(candidates.len())];
            out.push(pool.remove(index).0.clone());
        }
        Ok(out)
    }
    pub(super) fn bonus(&mut self, previous: Option<PreyBonus>) -> PreyBonus {
        let kinds = [
            BonusType::Damage,
            BonusType::Reduction,
            BonusType::Experience,
            BonusType::Loot,
        ];
        let choices: Vec<_> = kinds
            .into_iter()
            .filter(|kind| previous.is_none_or(|b| b.grade != 10 || b.kind != *kind))
            .collect();
        let kind = choices[self.below(choices.len())];
        let min = previous.map_or(2, |b| b.grade.saturating_add(1).min(10));
        let grade = min + self.below(usize::from(11 - min)) as u8;
        // The bounds above make this a valid PREY-0 grade.
        PreyBonus {
            kind,
            grade,
            percent: match kind {
                BonusType::Damage => 2 * grade + 5,
                BonusType::Reduction => 2 * grade + 10,
                _ => 3 * grade + 10,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rules() -> PreyRules {
        PreyRules {
            schema_version: 1,
            list_size: 9,
            expanded_list_size: 12,
            minimum_levels: [1, 50, 100, 300, 600],
            pick_again_list_multiplier_ppm: 2_000_000,
            pick_again_value_factor_ppm: 300_000,
            defense_mode: oteryn_gameplay_progression::prey::PreyDefenseMode::DealtOrAttacked,
            creatures: Default::default(),
            npc_sell_prices: Default::default(),
        }
    }

    #[test]
    fn prey_draw_falls_down_then_up_and_never_repeats_offered_creatures()
    -> Result<(), DurabilityError> {
        let catalogue = PreyCatalogue {
            rules: fixture_rules(),
            unsupported_loot: Default::default(),
            creatures: (1..=5)
                .flat_map(|tier| {
                    (0..3).map(move |n| (format!("oteryn:creature.b{tier}_{n}"), tier, false))
                })
                .collect(),
        };
        let list = Draw::new(&[7; 32], &[3; 16], b"list").list(&catalogue, 3, 12, &[])?;
        for (chunk, tier) in list.chunks(3).zip([3, 2, 1, 4]) {
            assert!(
                chunk
                    .iter()
                    .all(|key| key.starts_with(&format!("oteryn:creature.b{tier}_")))
            );
        }
        let rest = Draw::new(&[7; 32], &[4; 16], b"list").list(&catalogue, 3, 12, &list)?;
        assert_eq!(rest.len(), 3);
        assert!(
            rest.iter()
                .all(|key| key.starts_with("oteryn:creature.b5_"))
        );
        let offered: Vec<_> = list.into_iter().chain(rest).collect();
        assert!(
            Draw::new(&[7; 32], &[5; 16], b"list")
                .list(&catalogue, 3, 12, &offered)?
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn prey_catalogue_draws_only_wire_encodable_native_creatures()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::{
            native_gameplay::NativeGameplayInput, native_gameplay_test_world_id,
            qualified_native_gameplay_test_state, qualify_selected_native_gameplay_room,
        };
        use oteryn_protocol_oteryn::prey as wire;

        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let input = NativeGameplayInput::from_manifest(&manifest)?;
        let room = qualify_selected_native_gameplay_room(native_gameplay_test_world_id()?, &input)?;
        let native = qualified_native_gameplay_test_state(room.compiled())?;
        let catalogue = PreyCatalogue::try_from(&native)?;
        // This secret and occurrence drew canary:creature/rat before catalogue filtering.
        let secret = [
            0xa9, 0xf2, 0x48, 0xea, 0xb2, 0xb6, 0xa6, 0x01, 0xf7, 0x53, 0xa7, 0x5c, 0x77, 0x41,
            0xa6, 0xc8, 0xbe, 0x1e, 0x42, 0x8c, 0x39, 0x23, 0xb1, 0x39, 0x95, 0xe3, 0xf3, 0xef,
            0x64, 0xb0, 0xac, 0x2a,
        ];
        let creatures = Draw::new(&secret, &5_u128.to_be_bytes(), b"oteryn.prey.list.v1").list(
            &catalogue,
            1,
            9,
            &[],
        )?;
        let reply = wire::Reply {
            status: 0,
            wildcard_balance: 0,
            list_size: 9,
            expansion_owned: false,
            offered_today: vec![],
            offered_next: String::new(),
            slots: vec![wire::Slot {
                slot: 1,
                state: 1,
                creatures,
                selected: String::new(),
                bonus: 0,
                grade: 0,
                percent: 0,
                hunting_seconds: 0,
                option: 0,
                free_reroll_at: 0,
                chosen_tier: 1,
                allowed_tier: 1,
                selection_epoch: 0,
            }],
        };
        assert!(
            reply.slots[0]
                .creatures
                .iter()
                .all(|creature| !catalogue.unsupported_loot.contains(creature)),
            "listed prey creatures must be selectable"
        );
        let encoded = wire::encode_reply(&reply)
            .map_err(|error| format!("expiry draw must be wire encodable: {error:?}"))?;
        assert_eq!(wire::decode_reply(&encoded), Ok(reply));
        // Check every candidate, including exclusive creatures available through SelectAny.
        for (creature, _, _) in &catalogue.creatures {
            assert!(
                wire::encode_request(&wire::Request::SelectAny {
                    slot: 1,
                    creature: creature.clone(),
                })
                .is_ok(),
                "unencodable prey candidate: {creature}"
            );
        }
        Ok(())
    }

    #[test]
    fn prey_draw_is_receipt_stable_unique_and_respects_bands() -> Result<(), DurabilityError> {
        let catalogue = PreyCatalogue {
            rules: fixture_rules(),
            unsupported_loot: Default::default(),
            creatures: (1..=5)
                .flat_map(|stars| {
                    (0..20).map(move |n| (format!("oteryn:creature.b{stars}_{n}"), stars, false))
                })
                .collect(),
        };
        // OWNER-DEVIATION PREY-DIFF, 2026-11-01: all entries use the chosen tier.
        for tier in 1..=5 {
            let list = Draw::new(&[7; 32], &[3; 16], b"list").list(&catalogue, tier, 9, &[])?;
            assert_eq!(
                list,
                Draw::new(&[7; 32], &[3; 16], b"list").list(&catalogue, tier, 9, &[])?
            );
            assert_eq!(
                list.iter().collect::<std::collections::BTreeSet<_>>().len(),
                9
            );
            assert!(
                list.iter()
                    .all(|key| key.starts_with(&format!("oteryn:creature.b{tier}_")))
            );
            let next = Draw::new(&[7; 32], &[4; 16], b"list").list(&catalogue, tier, 9, &list)?;
            assert!(next.iter().all(|key| !list.contains(key)));
            let expanded =
                Draw::new(&[7; 32], &[5; 16], b"list").list(&catalogue, tier, 12, &[])?;
            assert_eq!(expanded.len(), 12);
        }
        Ok(())
    }
    #[test]
    fn prey_draw_never_offers_oversized_loot_creatures() -> Result<(), DurabilityError> {
        let creatures: Vec<_> = (1..=4)
            .flat_map(|stars| {
                (0..4).map(move |n| (format!("oteryn:creature.b{stars}_{n}"), stars, false))
            })
            .collect();
        let catalogue = PreyCatalogue {
            rules: fixture_rules(),
            unsupported_loot: creatures
                .iter()
                .filter(|c| c.0.ends_with("_0"))
                .map(|c| c.0.clone())
                .collect(),
            creatures,
        };
        for n in 0..50_u8 {
            let list = Draw::new(&[7; 32], &[n; 16], b"list").list(&catalogue, 1, 9, &[])?;
            assert_eq!(list.len(), 9);
            assert!(
                list.iter()
                    .all(|key| !catalogue.unsupported_loot.contains(key))
            );
        }
        Ok(())
    }
    #[test]
    fn prey_bonus_reroll_increases_grade_and_changes_maximum_type() {
        let mut seen = std::collections::BTreeSet::new();
        for n in 0..200_u8 {
            let mut draw = Draw::new(&[7; 32], &[n; 16], b"bonus");
            let mut b = draw.bonus(None);
            assert!((2..=10).contains(&b.grade));
            seen.insert(format!("{:?}", b.kind));
            for _ in 0..10 {
                let next = draw.bonus(Some(b));
                assert!(next.grade > b.grade || next.grade == 10);
                if b.grade == 10 {
                    assert_ne!(next.kind, b.kind);
                }
                b = next;
            }
            assert_eq!(b.grade, 10);
        }
        assert_eq!(seen.len(), 4);
    }
}
