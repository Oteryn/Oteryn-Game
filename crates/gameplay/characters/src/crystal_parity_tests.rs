//! Crystal parity for re-audit rows C14 (experience, levels, skill and magic progression) and C15
//! (stamina XP effects and offline recovery), `docs/architecture/reviews/
//! OTERYN_GAME_CRYSTAL_REAUDIT_2026-10-28.md`.
//!
//! The reference numbers are data read from Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
//! (`OTS_HYPOTHESIS_ONLY`); no source is copied. Each test drives the production function:
//! - `Player::getExpForLevel` (`src/creatures/players/player.cpp:4551`);
//! - `Vocation::getReqSkillTries` / `getReqMana` (`src/creatures/players/vocations/
//!   vocation.cpp:347,376,406`) with the `data/XML/vocations.xml` multipliers;
//! - `addExperience` level gains `getHPGain` / `getManaGain` (`player.cpp:3661`);
//! - `Player.getFinalBonusStamina` (`data/libs/functions/player.lua:434`) and
//!   `regenerate_stamina.lua` with `timeToRegenMinuteStamina = 180`,
//!   `timeToRegenMinutePremiumStamina = 360` (`config.lua.dist`).
//!
//! Recorded divergences are pinned, not adopted: the Monk skill rows (Oteryn follows Canary
//! `47dfd51f`), and the XP composition rounding and low-level bonus (D118, declared differences).
#![allow(clippy::expect_used)]

use crate::hunting_experience::hunting_experience;
use crate::stamina::regenerate_stamina_minutes;
use crate::vitals::maxima;
use oteryn_gameplay_content::content::character_progression_content::CharacterProgressionContent;
use oteryn_gameplay_content::content::training_formula::CompiledTrainingFormula;
use oteryn_gameplay_domain::domain::character_build::BuildFormula;

/// Crystal `Player::getExpForLevel`, evaluated exactly.
fn crystal_experience(level: i128) -> i128 {
    (((level - 6) * level + 17) * level - 12) / 6 * 100
}

/// Crystal `Vocation::skillBase`, in skill order fist, club, sword, axe, distance, shield, fishing.
const CRYSTAL_SKILL_BASE: [u32; 7] = [50, 50, 50, 50, 30, 100, 20];

/// Crystal `vocations.xml`: key, `manamultiplier`, the seven skill multipliers, `gainhp`,
/// `gainmana`.
const CRYSTAL_VOCATIONS: [(&str, f32, [f32; 7], u32, u32); 9] = [
    ("none", 4.0, [1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1], 5, 5),
    ("sorcerer", 1.1, [1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1], 5, 30),
    ("druid", 1.1, [1.5, 1.8, 1.8, 1.8, 1.8, 1.5, 1.1], 5, 30),
    ("paladin", 1.4, [1.2, 1.2, 1.2, 1.2, 1.1, 1.1, 1.1], 10, 15),
    ("knight", 3.0, [1.1, 1.1, 1.1, 1.1, 1.4, 1.1, 1.1], 15, 5),
    (
        "master_sorcerer",
        1.1,
        [1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1],
        5,
        30,
    ),
    (
        "elder_druid",
        1.1,
        [1.5, 1.8, 1.8, 1.8, 1.8, 1.5, 1.1],
        5,
        30,
    ),
    (
        "royal_paladin",
        1.4,
        [1.2, 1.2, 1.2, 1.2, 1.1, 1.1, 1.1],
        10,
        15,
    ),
    (
        "elite_knight",
        3.0,
        [1.1, 1.1, 1.1, 1.1, 1.4, 1.1, 1.1],
        15,
        5,
    ),
];

/// Crystal Monk and Exalted Monk: `manamultiplier`, skill multipliers, `gainhp`, `gainmana`.
const CRYSTAL_MONK: (f32, [f32; 7], u32, u32) = (1.3, [1.1, 1.2, 1.2, 1.2, 1.5, 1.1, 1.1], 10, 10);
/// Canary `47dfd51f` Monk skill multipliers, which the admitted build-training profile carries.
const CANARY_MONK_SKILLS: [f32; 7] = [1.1, 1.5, 1.5, 1.5, 2.0, 1.2, 1.1];

fn formula() -> CompiledTrainingFormula {
    CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
        "content-r1",
    )
    .expect("training profile")
}

/// Crystal `getReqSkillTries`: `skillBase * pow(double(multiplier), level - 11)`, truncated.
fn crystal_skill_tries(skill: usize, multiplier: f32, level: u16) -> u64 {
    (f64::from(CRYSTAL_SKILL_BASE[skill]) * f64::from(multiplier).powf(f64::from(level - 11)))
        as u64
}

/// Crystal `getReqMana`: `floor(1600 * pow(double(multiplier), magLevel - 1))`.
fn crystal_mana(multiplier: f32, level: u16) -> u64 {
    (1600.0 * f64::from(multiplier).powf(f64::from(level) - 1.0)).floor() as u64
}

#[test]
fn c14_experience_thresholds_equal_crystal_get_exp_for_level_at_every_level() {
    let content = CharacterProgressionContent::decode(include_bytes!(
        "../../../../rulesets/character/experience/native-section.json"
    ))
    .expect("admitted progression section");
    let policy = content
        .policy_for("profile", "ruleset", "content")
        .expect("finite policy");
    assert_eq!(policy.thresholds.len(), 2000);
    for entry in &policy.thresholds {
        assert_eq!(
            i128::from(entry.minimum_experience.get()),
            crystal_experience(i128::from(entry.level)),
            "level {}",
            entry.level
        );
    }
    assert_eq!(
        i128::from(policy.terminal_exclusive_experience.get()),
        crystal_experience(2001)
    );
}

#[test]
fn c14_skill_tries_and_magic_mana_equal_crystal_for_every_non_monk_vocation() {
    let formula = formula();
    for (key, mana, skills, _, _) in CRYSTAL_VOCATIONS {
        for level in (1..=200).take_while(|&level| crystal_mana(mana, level) < 1 << 62) {
            assert_eq!(
                formula.required(key, 0, level),
                Some(crystal_mana(mana, level)),
                "{key} magic level {level}"
            );
        }
        for (skill, multiplier) in skills.into_iter().enumerate() {
            // Crystal returns 0 at and below level 10; Oteryn starts skills at 10.
            assert_eq!(formula.required(key, skill + 1, 10), None);
            for level in 11..=200 {
                let expected = crystal_skill_tries(skill, multiplier, level);
                // Near 2^63 Crystal's u64 cast saturates; Oteryn refuses the level.
                if expected >= 1 << 62 {
                    break;
                }
                assert_eq!(
                    formula.required(key, skill + 1, level),
                    Some(expected),
                    "{key} skill {skill} level {level}"
                );
            }
        }
    }
}

#[test]
fn c14_monk_magic_matches_and_skill_rows_are_the_recorded_canary_crystal_conflict() {
    let formula = formula();
    let (mana, crystal_skills, _, _) = CRYSTAL_MONK;
    for key in ["monk", "exalted_monk"] {
        for level in (1..=200).take_while(|&level| crystal_mana(mana, level) < 1 << 62) {
            assert_eq!(
                formula.required(key, 0, level),
                Some(crystal_mana(mana, level))
            );
        }
        // UNVERIFIED: no constant set is admitted as the Global formula
        // (docs/reference/content-prep/skill-formulas-content-1). Oteryn keeps Canary's row.
        for (skill, multiplier) in CANARY_MONK_SKILLS.into_iter().enumerate() {
            assert_eq!(
                formula.required(key, skill + 1, 12),
                Some(crystal_skill_tries(skill, multiplier, 12))
            );
        }
    }
    let differing: Vec<usize> = (0..7)
        .filter(|&skill| CANARY_MONK_SKILLS[skill] != crystal_skills[skill])
        .collect();
    assert_eq!(differing, [1, 2, 3, 4, 5]);
}

#[test]
fn c14_health_and_mana_level_gains_equal_crystal_vocation_gains() {
    let pool = |key: &str, level: u32| maxima(key, level, (1 << 28) - 1, 16_383).expect("maxima");
    let monk = [("monk", CRYSTAL_MONK.2, CRYSTAL_MONK.3)];
    let exalted = [("exalted_monk", CRYSTAL_MONK.2, CRYSTAL_MONK.3)];
    let rows = CRYSTAL_VOCATIONS
        .iter()
        .map(|&(key, _, _, hp, mana)| (key, hp, mana))
        .chain(monk)
        .chain(exalted);
    let (_, _, _, none_hp, none_mana) = CRYSTAL_VOCATIONS[0];
    for (key, hp, mana) in rows {
        // Below level 8 a character advances as Crystal's vocation None.
        for level in 1..8 {
            let (h0, m0, _) = pool(key, level);
            let (h1, m1, _) = pool(key, level + 1);
            assert_eq!((h1 - h0, m1 - m0), (none_hp, none_mana), "{key} {level}");
        }
        for level in 8..1000 {
            let (h0, m0, _) = pool(key, level);
            let (h1, m1, _) = pool(key, level + 1);
            assert_eq!((h1 - h0, m1 - m0), (hp, mana), "{key} {level}");
        }
    }
}

/// Crystal `regenerate_stamina.lua`: Lua number division, truncated by `setStamina`'s `uint16`.
fn crystal_offline_stamina(stamina: u32, offline: u64) -> u32 {
    let offline = offline.min(86_400 * 21) as f64 - 600.0;
    if offline < 180.0 {
        return stamina;
    }
    let minutes = f64::from(stamina);
    let max_normal = 2340.0 - minutes.min(2340.0);
    let regain = offline / 180.0;
    let after = if regain > max_normal {
        (2520.0f64).min(minutes.max(2340.0) + (offline - max_normal * 180.0) / 360.0)
    } else {
        minutes + regain
    };
    after as u32
}

#[test]
fn c15_offline_stamina_recovery_equals_crystal_for_every_start_value() {
    let elapsed = [
        0,
        599,
        600,
        779,
        780,
        959,
        960,
        1_139,
        1_140,
        1_319,
        1_320,
        3_600,
        28_800,
        421_799,
        421_800,
        422_159,
        422_160,
        486_599,
        486_600,
        1_814_399,
        1_814_400,
        1_814_401,
        u64::MAX,
    ];
    for stamina in 0..=2_520 {
        for seconds in elapsed {
            assert_eq!(
                regenerate_stamina_minutes(stamina, seconds),
                Ok(crystal_offline_stamina(stamina, seconds)),
                "stamina {stamina}, offline {seconds}"
            );
        }
    }
}

#[test]
fn c15_stamina_bands_equal_crystal_final_bonus_stamina_without_other_modifiers() {
    // Crystal `onGainExperience` above level 50 with no Prey, boost or stage:
    // `exp * getFinalBonusStamina()`, truncated to uint64; stamina 0 awards nothing (player.cpp:6613).
    let crystal = |share: i64, stamina: i32, premium: bool| -> i64 {
        if stamina == 0 {
            return 0;
        }
        let bonus = if stamina > 2340 && premium {
            1.5
        } else if stamina <= 840 {
            0.5
        } else {
            1.0
        };
        (share as f64 * bonus) as i64
    };
    for stamina in [0, 1, 839, 840, 841, 2339, 2340, 2341, 2519, 2520] {
        for premium in [false, true] {
            for share in (0..=1_000).chain([123_456_789, 9_007_199_254]) {
                assert_eq!(
                    hunting_experience(share, 0, 0, stamina, premium),
                    Ok(crystal(share, stamina, premium)),
                    "share {share}, stamina {stamina}, premium {premium}"
                );
            }
        }
    }
}

#[test]
fn c15_prey_and_boost_rounding_is_the_recorded_d118_divergence() {
    // Crystal `onGainExperience`: `ceil(exp * (100 + prey) / 100)`, then
    // `exp * (1 + boost / 100 + lowLevel / 100) * stamina`, truncated once. Oteryn floors each
    // D118 step. These pins name the divergence; they are not adopted.
    assert_eq!(hunting_experience(1, 5, 0, 2000, false), Ok(1)); // Crystal ceil(1.05) = 2
    assert_eq!(hunting_experience(1, 0, 50, 2520, true), Ok(1)); // Crystal 1 * 1.5 * 1.5 = 2.25 -> 2
}
