//! PREY-DIFF offline model. Input facts come from prey_difficulty.py; damage and
//! activity are evaluated by the same public combat/progression/clock owners.
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use oteryn_gameplay_characters::stamina::HuntingActivityClock;
use oteryn_gameplay_combat_rules::{
    attack::{
        AttackConstants, BlockBudget, FightMode, formulas::DefenceSource, player_melee_formula,
    },
    resolution::{
        creature_mitigated, creature_physical_response,
        player_defence::{HandDefence, player_defence_bounds},
        swing_draw, swing_mitigated,
    },
};
use oteryn_gameplay_content::content::{
    creature_policy::{
        CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags, CreatureResistance,
    },
    creature_reward::{CreatureRewardTable, LootTablesSection},
    native_gameplay::CreatureProfilesDocument,
    spell_data::formula::FormulaInputs,
};
use oteryn_gameplay_progression::prey::{HUNTING_SECONDS, PreyActivityKind, PreyDefenseMode};
use oteryn_simulation_determinism::{GameplayDecisionRoot, SemanticTimeMicros};
use serde::{Deserialize, Serialize};

type ModelResult<T> = Result<T, Box<dyn Error>>;
const MICROS_PER_SECOND: u64 = 1_000_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Ratio {
    numerator: i64,
    denominator: u64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Resistance {
    damage_type: String,
    percent: Ratio,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Melee {
    minimum: u32,
    maximum: u32,
    chance_ppm: u32,
    interval_ms: u64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Creature {
    name: String,
    health: u64,
    defense: u32,
    armor: u32,
    mitigation: Option<Ratio>,
    resistances: Vec<Resistance>,
    damage_immunities: Vec<String>,
    melee: Option<Melee>,
}

impl Creature {
    fn policy(&self, key: &str) -> ModelResult<CompiledCreaturePolicy> {
        // Only the source health, mitigation and elemental responses are read by
        // the pure damage owner below. This value is never a runtime authority.
        Ok(CompiledCreaturePolicy {
            definition_key: key.to_owned(),
            definition_revision: "model-input".into(),
            display_name: self.name.clone(),
            maximum_health: i64::try_from(self.health)?,
            forge_difficulty: None,
            base_speed: 0,
            outfit_look_type: 0,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(self.defense),
            armor: Some(self.armor),
            mitigation: self.mitigation.as_ref().map(|value| CreatureExactRatio {
                numerator: value.numerator,
                denominator: value.denominator,
            }),
            resistances: self
                .resistances
                .iter()
                .map(|value| CreatureResistance {
                    damage_type: value.damage_type.clone(),
                    percent: CreatureExactRatio {
                        numerator: value.percent.numerator,
                        denominator: value.percent.denominator,
                    },
                })
                .collect(),
            damage_immunities: self.damage_immunities.clone(),
            healing_from_damage: vec![],
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
            preferred_distance: None,
            reward_boss: None,
            blocked_by_nearby_players: false,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Preset {
    tier: u8,
    level: u32,
    skill: u32,
    weapon: String,
    #[serde(default)]
    equipment: Vec<String>,
    attack: u32,
    armor: u32,
    defense: u32,
    #[serde(default)]
    shield: bool,
    source: String,
    item_sources: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Point {
    creature: String,
    source_path: String,
    source_group: String,
    source_point: usize,
    cell: serde_json::Value,
    respawn_ms: Option<u64>,
    #[serde(default)]
    boss_policy: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Model {
    name: String,
    tier: u8,
    chosen: String,
    points: Vec<Point>,
}

#[derive(Debug, Deserialize)]
struct Input {
    schema_version: u8,
    seed: u64,
    horizon_hours: u32,
    duel_samples: u32,
    duel_horizon_seconds: u32,
    presets: Vec<Preset>,
    creatures: BTreeMap<String, Creature>,
    tiers: BTreeMap<String, u8>,
    models: Vec<Model>,
    input_sha256: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Event {
    at: u64,
    dealt: bool,
    received: bool,
    attacked: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Duel {
    duration: u64,
    killed: bool,
    events: Vec<Event>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Clock {
    owner: HuntingActivityClock,
    spent_seconds: u32,
    ended_at: Option<u64>,
}

impl Clock {
    fn activity(&mut self, at: u64) {
        if self.ended_at.is_some() {
            return;
        }
        let remaining = (HUNTING_SECONDS - self.spent_seconds) / 60;
        let (spent, successor) = self.owner.activity(at, remaining as u16);
        self.owner = successor;
        self.spent_seconds += u32::from(spent) * 60;
        if self.spent_seconds == HUNTING_SECONDS {
            self.ended_at = Some(at);
        }
    }
}

fn draw(
    root: &GameplayDecisionRoot,
    duel: u64,
    ordinal: u16,
    purpose: &[u8],
    minimum: i64,
    maximum: i64,
) -> ModelResult<i64> {
    swing_draw(
        root,
        [1; 16],
        ([2; 16], duel),
        ordinal,
        purpose,
        "attack.damage",
        minimum,
        maximum,
    )
    .ok_or_else(|| "PREY_MODEL_RNG_OR_BOUNDS".into())
}

fn duel(
    root: &GameplayDecisionRoot,
    constants: &AttackConstants,
    preset: &Preset,
    creature: &Creature,
    key: &str,
    occurrence: u64,
    horizon_seconds: u32,
) -> ModelResult<Duel> {
    let policy = creature.policy(key)?;
    let inputs = FormulaInputs {
        level: preset.level,
        magic_level: 0,
        base_power: None,
        attack_skill: preset.skill,
        attack_value: preset.attack,
        attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
        shielding_skill: preset.skill,
        shield_defense: Some(preset.defense),
    };
    let (low, high) = player_melee_formula(constants).bounds(&inputs)?;
    let hand = HandDefence {
        source: if preset.shield {
            DefenceSource::Shield
        } else {
            DefenceSource::Weapon
        },
        value: preset.defense,
        skill_index: Some(if preset.shield { 5 } else { 1 }),
    };
    let mut creature_blocks =
        BlockBudget::settled(&constants.block, SemanticTimeMicros::from_micros(0));
    let mut player_blocks = creature_blocks;
    let mut health = i64::try_from(creature.health)?;
    let interval = constants.attack_interval_micros();
    let mut next_player = interval;
    let mut next_monster = creature
        .melee
        .as_ref()
        .map(|m| m.interval_ms * 1_000)
        .unwrap_or(u64::MAX);
    let mut output = Duel::default();
    // Bounded offline duel. An unkillable physical target is reported as zero
    // throughput, never silently assigned a guessed kill time.
    for ordinal in 0..u16::MAX {
        let at = next_player.min(next_monster);
        if at > u64::from(horizon_seconds) * MICROS_PER_SECOND {
            break;
        }
        let mut event = Event {
            at,
            ..Event::default()
        };
        if next_monster == at {
            let melee = creature.melee.as_ref().ok_or("PREY_MODEL_MELEE")?;
            let chance = draw(root, occurrence, ordinal, b"monster-chance", 0, 999_999)?;
            if chance < i64::from(melee.chance_ppm) {
                event.attacked = true;
                let raw = draw(
                    root,
                    occurrence,
                    ordinal,
                    b"monster-damage",
                    i64::from(melee.minimum),
                    i64::from(melee.maximum),
                )?;
                let blocked =
                    player_blocks.try_block(&constants.block, SemanticTimeMicros::from_micros(at));
                let defense = player_defence_bounds(
                    constants,
                    hand,
                    preset.skill,
                    FightMode::Offensive,
                    next_player > interval,
                )
                .ok_or("PREY_MODEL_PLAYER_DEFENCE")?;
                let landed = swing_mitigated(
                    root,
                    constants,
                    [2; 16],
                    ([1; 16], occurrence),
                    ordinal,
                    raw,
                    blocked.then_some(defense),
                    preset.armor,
                )
                .ok_or("PREY_MODEL_PLAYER_MITIGATION")?;
                event.received = landed > 0;
            }
            next_monster = next_monster.saturating_add(melee.interval_ms * 1_000);
        }
        if next_player == at {
            let raw = draw(root, occurrence, ordinal, b"player-damage", low, high)?;
            let blocked =
                creature_blocks.try_block(&constants.block, SemanticTimeMicros::from_micros(at));
            let mitigated = creature_mitigated(
                root,
                constants,
                [1; 16],
                ([2; 16], occurrence),
                ordinal,
                raw,
                blocked,
                creature.defense,
                creature.armor,
            )
            .ok_or("PREY_MODEL_CREATURE_MITIGATION")?;
            let landed = creature_physical_response(&policy, mitigated)
                .ok_or("PREY_MODEL_CREATURE_RESPONSE")?;
            event.dealt = landed > 0;
            health = health.saturating_sub(landed);
            next_player = next_player.saturating_add(interval);
        }
        if event.dealt || event.received || event.attacked {
            output.events.push(event);
        }
        output.duration = at;
        if health <= 0 {
            output.killed = true;
            return Ok(output);
        }
    }
    Ok(output)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CaseResult {
    name: String,
    tier: u8,
    chosen: String,
    global_micros: Option<u64>,
    offensive_micros: Option<u64>,
    defense_v1_micros: Option<u64>,
    defense_v2_micros: Option<u64>,
    observed_micros: u64,
    kills: u64,
    chosen_kills: u64,
    consumed_seconds: [u32; 4],
}

fn record(clocks: &mut [Clock; 4], event: Event, chosen: bool, start: u64) {
    let at = start.saturating_add(event.at);
    if event.dealt || event.received {
        clocks[0].activity(at);
    }
    if !chosen {
        return;
    }
    let activities = [
        (event.dealt, PreyActivityKind::DealtDamage),
        (event.received, PreyActivityKind::ReceivedDamage),
        (event.attacked, PreyActivityKind::Attacked),
    ];
    for (index, mode, bonus) in [
        (1, PreyDefenseMode::DealtOrAttacked, 1),
        (2, PreyDefenseMode::ReceivedDamage, 2),
        (3, PreyDefenseMode::DealtOrAttacked, 2),
    ] {
        if activities
            .iter()
            .any(|(present, kind)| *present && mode.consumes(bonus, *kind))
        {
            clocks[index].activity(at);
        }
    }
}

fn simulate(
    input: &Input,
    model: &Model,
    root: &GameplayDecisionRoot,
    constants: &AttackConstants,
) -> ModelResult<CaseResult> {
    let preset = input
        .presets
        .iter()
        .find(|p| p.tier == model.tier)
        .ok_or("PREY_MODEL_PRESET")?;
    let mut clocks = [Clock::default(); 4];
    let horizon = u64::from(input.horizon_hours) * 3_600 * MICROS_PER_SECOND;
    let mut ready = vec![0_u64; model.points.len()];
    let (mut now, mut kills, mut chosen_kills, mut turn, mut encounters) =
        (0_u64, 0_u64, 0_u64, 0_usize, 0_u64);
    while now < horizon && clocks.iter().any(|clock| clock.ended_at.is_none()) {
        let candidates = (0..model.points.len()).map(|offset| (turn + offset) % model.points.len());
        let Some(index) = candidates.min_by_key(|index| ready[*index]) else {
            break;
        };
        if ready[index] == u64::MAX {
            break;
        }
        now = now.max(ready[index]);
        if now >= horizon {
            now = horizon;
            break;
        }
        let point = &model.points[index];
        let creature = input
            .creatures
            .get(&point.creature)
            .ok_or("PREY_MODEL_CREATURE")?;
        encounters += 1;
        let fight = duel(
            root,
            constants,
            preset,
            creature,
            &point.creature,
            encounters,
            input.duel_horizon_seconds,
        )?;
        let chosen = point.creature == model.chosen;
        for event in fight.events {
            if now.saturating_add(event.at) > horizon {
                break;
            }
            record(&mut clocks, event, chosen, now);
        }
        let end = now.saturating_add(fight.duration);
        if end > horizon {
            now = horizon;
            break;
        }
        now = end;
        if !fight.killed {
            ready[index] = u64::MAX;
            turn = (index + 1) % model.points.len();
            continue;
        }
        kills += 1;
        chosen_kills += u64::from(chosen);
        ready[index] = point
            .respawn_ms
            .map_or(u64::MAX, |wait| now.saturating_add(wait * 1_000));
        turn = (index + 1) % model.points.len();
    }
    Ok(CaseResult {
        name: model.name.clone(),
        tier: model.tier,
        chosen: model.chosen.clone(),
        global_micros: clocks[0].ended_at,
        offensive_micros: clocks[1].ended_at,
        defense_v1_micros: clocks[2].ended_at,
        defense_v2_micros: clocks[3].ended_at,
        observed_micros: now.min(horizon),
        kills,
        chosen_kills,
        consumed_seconds: clocks.map(|clock| clock.spent_seconds),
    })
}

#[derive(Debug, Serialize)]
struct Output {
    schema_version: u8,
    input_sha256: String,
    seed: u64,
    prey_seconds: u32,
    presets: Vec<Preset>,
    models: Vec<Model>,
    cases: Vec<CaseResult>,
    kills_per_hour_micros: BTreeMap<String, u64>,
    no_physical_kill: Vec<String>,
    no_source_melee: Vec<String>,
    available_creatures: Vec<String>,
    unsettleable_creatures: Vec<String>,
}

fn qualified_catalogue(
    keys: &BTreeMap<String, u8>,
    creature_path: &Path,
) -> ModelResult<(Vec<String>, Vec<String>)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let creatures: CreatureProfilesDocument = serde_json::from_slice(&fs::read(creature_path)?)?;
    let raw = fs::read(root.join("tools/content-schema/native-gameplay/loot-tables.json"))?;
    let section = LootTablesSection::decode(&raw, &creatures)?;
    let rewards = CreatureRewardTable::from_pin(&creatures, Some(&section));
    let mut available = Vec::new();
    let mut unsettleable = Vec::new();
    for key in keys.keys() {
        match rewards.row(key) {
            Ok(row) if row.loot_table.entries.len() > 7 => {}
            Ok(_) => available.push(key.clone()),
            Err(_) => {
                available.push(key.clone());
                unsettleable.push(key.clone());
            }
        }
    }
    Ok((available, unsettleable))
}

fn run(input: Input, creature_path: &Path) -> ModelResult<Output> {
    if input.schema_version != 1
        || input.duel_samples == 0
        || input.horizon_hours == 0
        || input.duel_horizon_seconds == 0
    {
        return Err("PREY_MODEL_INPUT".into());
    }
    let mut seed = [0_u8; 32];
    seed[..8].copy_from_slice(&input.seed.to_be_bytes());
    let root = GameplayDecisionRoot::from_bytes(seed);
    let constants = AttackConstants::checked_in().ok_or("PREY_MODEL_ATTACK_CONSTANTS")?;
    let mut rates = BTreeMap::new();
    let mut no_physical_kill = Vec::new();
    let mut no_source_melee = Vec::new();
    for (key, tier) in &input.tiers {
        let preset = input
            .presets
            .iter()
            .find(|p| p.tier == *tier)
            .ok_or("PREY_MODEL_PRESET")?;
        let creature = input.creatures.get(key).ok_or("PREY_MODEL_CREATURE")?;
        let mut total = 0_u64;
        let mut successes = 0_u64;
        for sample in 0..input.duel_samples {
            let fight = duel(
                &root,
                constants,
                preset,
                creature,
                key,
                u64::from(sample) + 1,
                input.duel_horizon_seconds,
            )?;
            if !fight.killed {
                break;
            }
            total = total
                .checked_add(fight.duration)
                .ok_or("PREY_MODEL_TIME_OVERFLOW")?;
            successes += 1;
        }
        let rate = if successes == u64::from(input.duel_samples) && total > 0 {
            u64::try_from(
                u128::from(successes) * 3_600 * u128::from(MICROS_PER_SECOND).pow(2)
                    / u128::from(total),
            )?
        } else {
            no_physical_kill.push(key.clone());
            0
        };
        rates.insert(key.clone(), rate);
        if creature.melee.is_none() {
            no_source_melee.push(key.clone());
        }
    }
    let cases = input
        .models
        .iter()
        .map(|model| simulate(&input, model, &root, constants))
        .collect::<ModelResult<Vec<_>>>()?;
    let (available_creatures, unsettleable_creatures) =
        qualified_catalogue(&input.tiers, creature_path)?;
    Ok(Output {
        schema_version: 1,
        input_sha256: input.input_sha256,
        seed: input.seed,
        prey_seconds: HUNTING_SECONDS,
        presets: input.presets,
        models: input.models,
        cases,
        kills_per_hour_micros: rates,
        no_physical_kill,
        no_source_melee,
        available_creatures,
        unsettleable_creatures,
    })
}

fn main() -> ModelResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 6
        || args[0] != "--input"
        || args[2] != "--output"
        || args[4] != "--creature-profiles"
    {
        return Err("usage: prey_time_sim --input INPUT.json --output OUTPUT.json --creature-profiles FRESH_NATIVE.json".into());
    }
    let input: Input = serde_json::from_slice(&fs::read(&args[1])?)?;
    let output = run(input, Path::new(&args[5]))?;
    let path = PathBuf::from(&args[3]);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(&output)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    println!(
        "PREY_DIFF_SIMULATION_COMPLETED cases={} creatures={}",
        output.cases.len(),
        output.kills_per_hour_micros.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prey_model_real_combat_duel_is_seed_deterministic() -> ModelResult<()> {
        let constants = AttackConstants::checked_in().ok_or("constants")?;
        let preset = Preset {
            tier: 1,
            level: 8,
            skill: 10,
            weapon: "fixture:axe".into(),
            equipment: vec![],
            attack: 12,
            armor: 0,
            defense: 6,
            shield: false,
            source: "fixture".into(),
            item_sources: BTreeMap::new(),
        };
        let creature = Creature {
            name: "Rat".into(),
            health: 20,
            defense: 1,
            armor: 1,
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            melee: Some(Melee {
                minimum: 0,
                maximum: 8,
                chance_ppm: 1_000_000,
                interval_ms: 2_000,
            }),
        };
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let first = duel(
            &root,
            constants,
            &preset,
            &creature,
            "fixture:rat",
            1,
            3_600,
        )?;
        assert!(first.killed);
        assert!(first.events.iter().any(|event| event.dealt));
        assert_eq!(
            first,
            duel(
                &root,
                constants,
                &preset,
                &creature,
                "fixture:rat",
                1,
                3_600
            )?
        );
        Ok(())
    }

    #[test]
    fn prey_model_does_not_count_kills_after_observation_horizon() -> ModelResult<()> {
        let constants = AttackConstants::checked_in().ok_or("constants")?;
        let preset = Preset {
            tier: 1,
            level: 8,
            skill: 10,
            weapon: "fixture:axe".into(),
            equipment: vec![],
            attack: 12,
            armor: 0,
            defense: 6,
            shield: false,
            source: "fixture".into(),
            item_sources: BTreeMap::new(),
        };
        let key = "fixture:rat".to_owned();
        let creature = Creature {
            name: "Rat".into(),
            health: 20,
            defense: 1,
            armor: 1,
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            melee: None,
        };
        let model = Model {
            name: "single".into(),
            tier: 1,
            chosen: key.clone(),
            points: vec![Point {
                creature: key.clone(),
                source_path: "fixture".into(),
                source_group: "fixture".into(),
                source_point: 0,
                cell: serde_json::Value::Null,
                respawn_ms: Some(3_600_000),
                boss_policy: None,
            }],
        };
        let input = Input {
            schema_version: 1,
            seed: 7,
            horizon_hours: 1,
            duel_samples: 1,
            duel_horizon_seconds: 3_600,
            presets: vec![preset],
            creatures: BTreeMap::from([(key.clone(), creature)]),
            tiers: BTreeMap::from([(key, 1)]),
            models: vec![model.clone()],
            input_sha256: "fixture".into(),
        };
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let result = simulate(&input, &model, &root, constants)?;
        assert_eq!(result.kills, 1);
        assert_eq!(result.chosen_kills, 1);
        assert_eq!(result.observed_micros, 3_600 * MICROS_PER_SECOND);
        Ok(())
    }

    #[test]
    fn prey_model_blocked_attack_distinguishes_defense_variants() {
        let mut clocks = [Clock::default(); 4];
        for at in [0, 30_000_000, 60_000_000] {
            record(
                &mut clocks,
                Event {
                    at,
                    attacked: true,
                    ..Event::default()
                },
                true,
                0,
            );
        }
        assert_eq!(clocks[0].spent_seconds, 0);
        assert_eq!(clocks[1].spent_seconds, 0);
        assert_eq!(clocks[2].spent_seconds, 0);
        assert_eq!(clocks[3].spent_seconds, 60);
    }

    #[test]
    fn prey_model_immune_target_still_records_incoming_attacks() -> ModelResult<()> {
        let constants = AttackConstants::checked_in().ok_or("constants")?;
        let preset = Preset {
            tier: 1,
            level: 8,
            skill: 10,
            weapon: "fixture:axe".into(),
            equipment: vec![],
            attack: 12,
            armor: 0,
            defense: 6,
            shield: false,
            source: "fixture".into(),
            item_sources: BTreeMap::new(),
        };
        let creature = Creature {
            name: "Immune fixture".into(),
            health: 100,
            defense: 0,
            armor: 0,
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec!["physical".into()],
            melee: Some(Melee {
                minimum: 0,
                maximum: 0,
                chance_ppm: 1_000_000,
                interval_ms: 2_000,
            }),
        };
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let fight = duel(
            &root,
            constants,
            &preset,
            &creature,
            "fixture:immune",
            1,
            62,
        )?;
        assert!(!fight.killed);
        assert_eq!(fight.duration, 62 * MICROS_PER_SECOND);
        let mut clocks = [Clock::default(); 4];
        for event in fight.events {
            assert!(event.attacked && !event.dealt && !event.received);
            record(&mut clocks, event, true, 0);
        }
        assert_eq!(clocks.map(|clock| clock.spent_seconds), [0, 0, 0, 60]);
        Ok(())
    }

    #[test]
    fn prey_model_mixed_short_encounters_do_not_scale_minutes_by_kill_share() {
        let mut clocks = [Clock::default(); 4];
        for second in 0..=60 {
            record(
                &mut clocks,
                Event {
                    at: second * MICROS_PER_SECOND,
                    dealt: true,
                    ..Event::default()
                },
                second.is_multiple_of(3),
                0,
            );
        }
        assert_eq!(clocks[0].spent_seconds, 60);
        assert_eq!(clocks[1].spent_seconds, 60);
        assert_eq!(clocks[3].spent_seconds, 60);
    }
}
