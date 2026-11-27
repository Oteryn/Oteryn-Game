//! Personal reward-boss rolls. The existing loot owner retains its shape/size validators;
//! the donor contribution factor and recipient are additional immutable draw inputs.
use super::*;

/// Crystal `reward_chest.lua:79-108`, `monstertype.lua:95-123`, and
/// `monster.lua:207-224`. A replay uses the same death, Character, table and factor.
/// Unlike ordinary corpse loot, the donor factor also applies to a nominal 100% entry.
pub fn plan_boss_loot(
    death: [u8; 16],
    character: [u8; 16],
    reference: &LootDefinitionRef,
    table: &LootTableDefinition,
    loot_factor: f64,
    top_score: bool,
) -> Result<LootPlan, LootPlanError> {
    plan_boss_loot_with_bonus(
        death,
        character,
        reference,
        table,
        loot_factor,
        top_score,
        0,
        |_| false,
    )
}

/// Crystal reward_chest.lua:84-111: extra equipment-only rolls retain each recipient's
/// contribution factor, exclude unique items and never acquire top-contributor privileges.
#[allow(
    clippy::too_many_arguments,
    reason = "existing personal loot identity plus bounded bonus rolls"
)]
pub fn plan_boss_loot_with_bonus(
    death: [u8; 16],
    character: [u8; 16],
    reference: &LootDefinitionRef,
    table: &LootTableDefinition,
    loot_factor: f64,
    top_score: bool,
    extra_rolls: u8,
    is_equipment: impl Fn(&str) -> bool,
) -> Result<LootPlan, LootPlanError> {
    if !loot_factor.is_finite() || loot_factor < 0.0 {
        return Err(LootPlanError::InvalidProbability);
    }
    if !matches!(
        table.algorithm,
        LootSelectionAlgorithm::IndependentBernoulliPpm | LootSelectionAlgorithm::NestedGroups
    ) {
        return Err(LootPlanError::UnsupportedAlgorithm);
    }
    let count = validate_tree(&table.entries, 0)?;
    if count > COMBAT01_LOOT_PLAN_ENTRIES_MAX {
        return Err(LootPlanError::TooManyTableEntries);
    }
    if count * 2 > COMBAT01_LOOT_RNG_DRAWS_MAX {
        return Err(LootPlanError::TooManyRngDraws);
    }
    let equipment: Vec<_> = table
        .entries
        .iter()
        .filter(|e| !e.details.unique && is_equipment(&e.item.production_key))
        .cloned()
        .collect();
    let extra_count = validate_tree(&equipment, 0)?;
    if extra_rolls > 3
        || (count + extra_count * usize::from(extra_rolls)) * 2 > COMBAT01_LOOT_RNG_DRAWS_MAX
    {
        return Err(LootPlanError::TooManyRngDraws);
    }
    let mut state = RollState {
        entries: Vec::new(),
        unique: std::collections::HashSet::new(),
        ordinal: 0,
        bytes: charge_plan_bytes(0, header_footprint_bytes(reference))?,
    };
    let context = BossRoll {
        death,
        character,
        reference,
        loot_factor,
        top_score,
    };
    roll(&context, &table.entries, None, &mut state)?;
    let extra = BossRoll {
        top_score: false,
        ..context
    };
    for _ in 0..extra_rolls {
        roll(&extra, &equipment, None, &mut state)?;
    }
    Ok(LootPlan {
        loot_table: reference.clone(),
        entries: state.entries,
    })
}

struct BossRoll<'a> {
    death: [u8; 16],
    character: [u8; 16],
    reference: &'a LootDefinitionRef,
    loot_factor: f64,
    top_score: bool,
}

fn roll(
    context: &BossRoll<'_>,
    source: &[LootTableEntry],
    parent: Option<usize>,
    state: &mut RollState,
) -> Result<(), LootPlanError> {
    let mut placement_ordinal = 0;
    for entry in source {
        let draw_ordinal = state.ordinal;
        state.ordinal += 1;
        let probability = entry
            .probability_ppm
            .ok_or(LootPlanError::InvalidProbability)?;
        let chance = draw(context, draw_ordinal, 0);
        let jitter = loot_factor_percent(draw(context, draw_ordinal, 1));
        if (entry.details.unique && !context.top_score)
            || state.unique.contains(&entry.item.production_key)
            || (is_surprise_bag(entry) && !is_global_event_loot_table(context.reference))
            || !success(chance, probability, jitter, context.loot_factor)
        {
            state.ordinal += validate_tree(&entry.details.children, 0)? as u32;
            continue;
        }
        let index = state.entries.len();
        state.push_instances(entry, chance, draw_ordinal, parent, &mut placement_ordinal)?;
        if entry.details.unique {
            state.unique.insert(entry.item.production_key.clone());
        }
        // A missed container still reserves its children's draw ordinals. A selected
        // container owns the selected children in that roll (Crystal reward_boss.lua:24–46).
        roll(context, &entry.details.children, Some(index), state)?;
    }
    Ok(())
}

fn success(draw: u64, probability: u32, jitter: u32, contribution_factor: f64) -> bool {
    // Lua uses the same draw for quantity; the fractional scaled chance is compared before
    // rounding. No Gut bonus applies to reward-boss rolls (`monster.lua:214`).
    (draw_ppm(draw) as f64)
        < f64::from(probability) * contribution_factor * f64::from(jitter) / 100.0
}

fn draw(context: &BossRoll<'_>, ordinal: u32, kind: u8) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(b"oteryn:personal-boss-reward:v1\0");
    hasher.update(context.death);
    hasher.update(context.character);
    for field in [
        &context.reference.family,
        &context.reference.production_key,
        &context.reference.revision_ref,
    ] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    hasher.update(ordinal.to_be_bytes());
    hasher.update([kind]);
    let digest = hasher.finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "bounded test fixtures")]
mod tests {
    use super::*;
    use oteryn_gameplay_content::content::creature_reward::{
        CreatureRewardTable, LootTablesSection,
    };
    use oteryn_gameplay_content::content::native_gameplay::CreatureProfilesDocument;

    #[test]
    fn arbaziloth_crystal_coin_roll_yields_three_coins() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let creatures: CreatureProfilesDocument = serde_json::from_reader(std::fs::File::open(
            root.join("content/creatures/definitions/spell-native-profiles.json"),
        )?)?;
        let loot = LootTablesSection::decode(
            &std::fs::read(root.join("tools/content-schema/native-gameplay/loot-tables.json"))?,
            &creatures,
        )?;
        let rewards = CreatureRewardTable::build(&creatures, &loot, &Default::default());
        let arbaziloth = rewards
            .row("oteryn:creature.arbaziloth")
            .map_err(|reason| format!("Arbaziloth reward row refused: {reason:?}"))?;
        let reference = arbaziloth
            .loot_table_ref
            .as_ref()
            .ok_or("missing loot reference")?;
        let plan = plan_boss_loot(
            [3; 16],
            [2; 16],
            reference,
            &arbaziloth.loot_table,
            2.0,
            true,
        )?;
        let coins = plan
            .entries
            .iter()
            .find(|entry| entry.item.production_key == "oteryn:item.tibia.i3043")
            .ok_or("missing crystal coin roll")?;
        assert_eq!(coins.quantity, 3);
        assert!(coins.stackable);
        Ok(())
    }

    fn reference() -> LootDefinitionRef {
        LootDefinitionRef::new("Loot", "oteryn:loot.test-boss", "definition-r1")
    }
    fn table(unique: bool) -> LootTableDefinition {
        LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![LootTableEntry {
                item: LootDefinitionRef::new("Item", "oteryn:item.gold", "definition-r1"),
                min_count: 1,
                max_count: 100,
                probability_ppm: Some(1_000_000),
                details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                    unique,
                    stackable: true,
                    ..Default::default()
                },
            }],
        }
    }

    #[test]
    fn bosstiary_slot_bonus_rerolls_equipment_only_without_unique_duplicates() {
        let mut source = table(false);
        source.entries[0].item =
            LootDefinitionRef::new("Item", "oteryn:item.sword", "definition-r1");
        source.entries[0].details.stackable = false;
        let mut unique = source.entries[0].clone();
        unique.details.unique = true;
        unique.item = LootDefinitionRef::new("Item", "oteryn:item.unique", "definition-r1");
        source.entries.push(unique);
        source.entries.push(table(false).entries.remove(0));
        let plan = plan_boss_loot_with_bonus(
            [1; 16],
            [2; 16],
            &reference(),
            &source,
            100.0,
            true,
            2,
            |key| key != "oteryn:item.gold",
        )
        .expect("bonus");
        assert_eq!(
            plan.entries
                .iter()
                .filter(|e| e.item.production_key == "oteryn:item.sword")
                .count(),
            3
        );
        assert_eq!(plan.entries.iter().filter(|e| e.unique).count(), 1);
        assert_eq!(
            plan.entries
                .iter()
                .filter(|e| e.item.production_key == "oteryn:item.gold")
                .count(),
            1
        );
        assert_eq!(
            plan,
            plan_boss_loot_with_bonus(
                [1; 16],
                [2; 16],
                &reference(),
                &source,
                100.0,
                true,
                2,
                |key| key != "oteryn:item.gold"
            )
            .expect("replay")
        );
    }

    #[test]
    fn boss_loot_replay_is_stable_and_characters_get_independent_rolls() {
        let mut table = table(false);
        table.entries[0].probability_ppm = Some(500_000);
        let first =
            plan_boss_loot([1; 16], [2; 16], &reference(), &table, 1.0, false).expect("plan");
        assert_eq!(
            first,
            plan_boss_loot([1; 16], [2; 16], &reference(), &table, 1.0, false).expect("replay")
        );
        let a = BossRoll {
            death: [1; 16],
            character: [2; 16],
            reference: &reference(),
            loot_factor: 1.0,
            top_score: false,
        };
        let b = BossRoll {
            character: [3; 16],
            ..a
        };
        assert_ne!(draw(&a, 0, 0), draw(&b, 0, 0));
    }

    #[test]
    fn unique_reward_only_reaches_the_top_score() {
        assert!(
            plan_boss_loot([1; 16], [2; 16], &reference(), &table(true), 2.0, false)
                .expect("other")
                .entries
                .is_empty()
        );
        assert_eq!(
            plan_boss_loot([1; 16], [2; 16], &reference(), &table(true), 2.0, true)
                .expect("top")
                .entries
                .len(),
            1
        );
    }

    #[test]
    fn contribution_and_jitter_apply_even_to_a_nominal_guaranteed_drop() {
        assert!(!success(600_000, 1_000_000, 100, 0.5));
        assert!(success(499_999, 1_000_000, 100, 0.5));
        assert!(!success(950_000, 1_000_000, 95, 1.0));
        assert!(!success(0, 0, 105, 5.0));
        assert!(!success(0, 1_000_000, 105, 0.0));
    }

    #[test]
    fn invalid_factors_and_table_inputs_are_refused() {
        for factor in [f64::NAN, f64::INFINITY, -1.0] {
            assert_eq!(
                plan_boss_loot([1; 16], [2; 16], &reference(), &table(false), factor, true),
                Err(LootPlanError::InvalidProbability)
            );
        }
        let mut bad = table(false);
        bad.entries[0].min_count = 0;
        assert_eq!(
            plan_boss_loot([1; 16], [2; 16], &reference(), &bad, 1.0, true),
            Err(LootPlanError::InvalidQuantityRange)
        );
    }
}
