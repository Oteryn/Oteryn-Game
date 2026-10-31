//! P05: the accepted NEWS-HIST-TALISMAN-1 profile, confirmed by PR 2764
//! comments 6542770501 and 6542842489. Damage and life leech have independent
//! 15%/10% activation draws; retries use the original physical occurrence.

use crate::durability::task_board::BountyTalismanSnapshot;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::spell::combat_batch::{OwnerCombatBatch, OwnerCombatChange};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

pub(in crate::gameplay_transport) type SpellLeech = (ExactActorRef, u16, u16);

pub(in crate::gameplay_transport) fn damage_effect(change: &OwnerCombatChange) -> bool {
    matches!(
        change,
        OwnerCombatChange::Damage { .. } | OwnerCombatChange::DamageWithHealing { .. }
    )
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Read before taking Channel guards and while the caller owns the spell lane.
    /// Commands that change bounty/equipment use that same lane.
    pub(in crate::gameplay_transport) async fn bounty_talisman_for_session(
        &self,
        session: GameSessionId,
    ) -> Result<BountyTalismanSnapshot, ()> {
        let Some(fence) = self.current_quest_fence(session).await.map_err(|_| ())? else {
            return Ok(BountyTalismanSnapshot::default());
        };
        self.root
            .read_bounty_talisman(self.character, self.holder, fence)
            .await
            .map_err(|_| ())
    }
}

fn activated(bonus: i64, roll: u64, chance: u64) -> u16 {
    if (1..=chance).contains(&roll) {
        u16::try_from(bonus.max(0)).unwrap_or(u16::MAX)
    } else {
        0
    }
}

/// Canary rules.lua:1215-1238: one uniform 1..=100 draw per effect. Each path
/// has its own deterministic stream and therefore shifts no combat/charm draw.
pub(in crate::gameplay_transport) fn hit_bonuses(
    snapshot: &BountyTalismanSnapshot,
    race: &str,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    ordinal: u16,
) -> Option<[u16; 2]> {
    let levels = snapshot.bonuses(race);
    let mut result = [0; 2];
    for (path, (key, chance)) in [("bounty.talisman.damage", 15), ("bounty.talisman.life", 10)]
        .into_iter()
        .enumerate()
    {
        if levels[path] <= 0 {
            continue;
        }
        let mut attempt = 0_u64;
        loop {
            let index = (u64::from(ordinal) << 32) | attempt;
            let value = deterministic_decision_u64(root, occurrence, key, index).ok()?;
            if value < u64::MAX - u64::MAX % 100 {
                result[path] = activated(levels[path], value % 100 + 1, chance);
                break;
            }
            attempt = attempt.checked_add(1).filter(|v| *v < (1 << 32))?;
        }
    }
    Some(result)
}

/// Canary on_combat_taskboard.lua:17-20 and Crystal combat.cpp:918-923:
/// signed damage adds ceil(value * bonus); its positive magnitude adds floor.
pub(in crate::gameplay_transport) fn damage(amount: i64, bonus: u16) -> i64 {
    if amount <= 0 {
        return amount;
    }
    let result = i128::from(amount) * (10_000 + i128::from(bonus)) / 10_000;
    i64::try_from(result).unwrap_or(i64::MAX)
}

/// Read the current monster race from the same staged batch's actor generation.
/// The returned leech rates are retained with the cast and applied to real damage
/// by the existing vitals owner, including its AoE reduction and replay guard.
pub(in crate::gameplay_transport) fn apply_spell_bonuses(
    runtime: &ChannelRuntimeV1,
    batch: &mut OwnerCombatBatch,
    snapshot: &BountyTalismanSnapshot,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> Option<Vec<SpellLeech>> {
    let mut leech = Vec::new();
    for effect in &mut batch.effects {
        let magnitude = match &mut effect.change {
            OwnerCombatChange::Damage { magnitude, .. } => magnitude,
            OwnerCombatChange::DamageWithHealing { damage, .. } => damage,
            _ => continue,
        };
        let Ok(creature) = runtime.companion_snapshot(effect.target) else {
            continue;
        };
        let [extra_damage, extra_life] = hit_bonuses(
            snapshot,
            &creature.state.policy.definition_key,
            root,
            occurrence,
            effect.sub_ordinal,
        )?;
        *magnitude = damage(*magnitude, extra_damage);
        if extra_life > 0 {
            leech.push((effect.target, effect.sub_ordinal, extra_life));
        }
    }
    Some(leech)
}

/// Canary game.cpp:9058-9060: round the combined life skill, with the same AoE
/// factor as the ordinary owner, bounded by the health the target actually lost.
fn life_amount(real: u32, skill: u16, affected: usize) -> u32 {
    if affected == 0 {
        return 0;
    }
    let targets = affected as u128;
    let denominator = 100_000 * targets;
    let numerator = u128::from(real) * u128::from(skill) * (targets + 9);
    u32::try_from(((numerator + denominator / 2) / denominator).min(u128::from(real)))
        .unwrap_or(real)
}

/// The timer owner has committed physical HP while retaining all Channel guards.
/// Its actual applied receipt is the replay fence for this one vitals successor.
pub(in crate::gameplay_transport) fn apply_timer_leech(
    runtime: &ChannelRuntimeV1,
    states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
    batch: &OwnerCombatBatch,
    receipt: &crate::foundation::runtime_actor_spell_types::CombatBatchReceipt,
    bounty: &[SpellLeech],
) {
    if !receipt.applied || bounty.is_empty() {
        return;
    }
    let session = batch.command.game_session_id();
    let Some(before) = states.get(runtime, batch.caster, session) else {
        return;
    };
    let ordinary = before.equipped_offence().life_leech;
    let affected = batch
        .effects
        .iter()
        .filter(|effect| damage_effect(&effect.change))
        .count();
    let gain = bounty
        .iter()
        .filter_map(|(target, ordinal, bonus)| {
            let health = receipt
                .effects
                .iter()
                .find(|effect| effect.target == *target && effect.sub_ordinal == *ordinal)?
                .health?;
            if !health.applied {
                return None;
            }
            let real = u32::try_from(
                health
                    .health_before
                    .saturating_sub(health.health_after.max(0))
                    .max(0),
            )
            .ok()?;
            Some(life_amount(real, ordinary.saturating_add(*bonus), affected))
        })
        .fold(0_u32, u32::saturating_add);
    if let Some(next) = before.after_leech(gain, 0) {
        states.commit(runtime, batch.caster, session, next);
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "qualified owner fixtures fail loudly on an impossible setup"
    )]
    use super::*;

    #[test]
    fn talisman_damage_uses_signed_source_rounding_and_keeps_components_separate() {
        assert_eq!(damage(33, 300), 33);
        assert_eq!(damage(34, 300), 35);
        assert_eq!(damage(33, 300) + damage(33, 300), 66);
        assert_eq!(damage(66, 300), 67);
        assert_eq!(damage(0, 5000), 0);
        assert_eq!(damage(-4, 5000), -4);
        assert_eq!(damage(i64::MAX, 5000), i64::MAX);
    }

    #[test]
    fn talisman_canary_activation_includes_only_the_first_fifteen_or_ten_rolls() {
        for roll in 1..=100 {
            assert_eq!(activated(300, roll, 15), if roll <= 15 { 300 } else { 0 });
            assert_eq!(activated(300, roll, 10), if roll <= 10 { 300 } else { 0 });
        }
        assert_eq!(activated(300, 0, 15), 0);
        assert_eq!(activated(0, 1, 15), 0);
    }

    #[test]
    fn talisman_life_uses_real_damage_combined_skill_rounding_and_aoe_factor() {
        assert_eq!(life_amount(100, 300, 1), 3);
        assert_eq!(life_amount(100, 300, 2), 2);
        assert_eq!(life_amount(1, 300, 1), 0);
        assert_eq!(life_amount(100, 5300, 2), 29);
        assert_eq!(life_amount(17, u16::MAX, 1), 17);
        assert_eq!(life_amount(0, 5000, 1), 0);
        assert_eq!(life_amount(100, 5000, 0), 0);
    }

    #[test]
    fn talisman_proc_draws_are_repeatable_and_race_gated() {
        use oteryn_gameplay_quests::quest::task_board::{
            self,
            full::{BountyStatus, BountyTask, TaskBoardData},
        };
        let mut board = TaskBoardData::default();
        board.bounty.state = BountyStatus::Active;
        board.bounty.active = Some(BountyTask {
            race: "oteryn:creature.rat".into(),
            required: 25,
            current: 0,
            experience: 1850,
            points: 25,
            grade: 0,
            difficulty: 0,
            slot: 0,
        });
        let tracks = [
            (task_board::TALISMAN_DAMAGE.into(), 355),
            (task_board::TALISMAN_LIFE_LEECH.into(), 355),
        ]
        .into();
        let snapshot = BountyTalismanSnapshot::from_board(&board, &tracks, true);
        let root = GameplayDecisionRoot::from_bytes([7; 32]);
        let mut damage_outcomes = std::collections::BTreeSet::new();
        let mut life_outcomes = std::collections::BTreeSet::new();
        for seed in 1..=255_u8 {
            let occurrence = DecisionOccurrenceId::from_bytes([seed; 16]);
            let first = hit_bonuses(&snapshot, "oteryn:creature.rat", &root, occurrence, 0);
            assert_eq!(
                hit_bonuses(&snapshot, "oteryn:creature.rat", &root, occurrence, 0),
                first
            );
            assert_eq!(
                hit_bonuses(&snapshot, "oteryn:creature.cave_rat", &root, occurrence, 0),
                Some([0, 0])
            );
            let [damage, life] = first.unwrap_or([u16::MAX; 2]);
            damage_outcomes.insert(damage);
            life_outcomes.insert(life);
        }
        assert_eq!(damage_outcomes, [0, 5000].into());
        assert_eq!(life_outcomes, [0, 5000].into());
    }

    #[test]
    fn talisman_timer_batch_uses_actual_overkill_aoe_and_replay_receipts() {
        use crate::foundation::runtime_actor_spell_types::{
            OwnerCombatEffect, SpellOccurrenceBinding,
        };
        use crate::foundation::{
            CharacterId, CharacterLease, CommandId, CommandRef, MovementLocalPosition,
        };
        use crate::gameplay_transport::actor_spell::{
            ChannelSpellStates,
            tests::{FACTS, runtime_with_capacity, wound},
        };
        use oteryn_gameplay_content::content::creature_policy::{
            CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
        };
        use oteryn_gameplay_quests::quest::task_board::{
            self,
            full::{BountyStatus, BountyTask, TaskBoardData},
        };
        let (mut runtime, caster, session) = runtime_with_capacity(73, 4);
        let character =
            CharacterId::decode(&[1, 0x91, 0, 0, 0, 74, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 74])
                .expect("character");
        runtime
            .bind_attacker_lease(
                caster,
                session,
                CharacterLease::new(character, 1).expect("lease"),
            )
            .expect("bound lease");
        let pin = runtime.content_pin().clone();
        runtime
            .initialize_pinned_test_position(
                caster,
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
            )
            .expect("caster position");
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![CompiledCreaturePolicy {
                        definition_key: "fixture:rat".into(),
                        definition_revision: "fixture:1".into(),
                        display_name: "fixture Rat".into(),
                        maximum_health: 20,
                        forge_difficulty: None,
                        base_speed: 110,
                        outfit_look_type: 21,
                        object_look_type: None,
                        summonable: false,
                        convinceable: false,
                        mana_cost: None,
                        is_familiar: false,
                        condition_immunities: vec![],
                        defense: Some(1),
                        armor: Some(1),
                        mitigation: None,
                        resistances: vec![],
                        damage_immunities: vec![],
                        healing_from_damage: vec![],
                        preferred_distance: Some(1),
                        reward_boss: Some(false),
                        blocked_by_nearby_players: false,
                        flags: CreatureFlags {
                            attackable: true,
                            illusionable: false,
                            health_hidden: false,
                        },
                    }],
                )
                .expect("policy"),
            )
            .expect("installed policy");
        let first = runtime
            .realize_native_qualification_spawn(
                pin.clone(),
                "fixture:rat",
                MovementLocalPosition {
                    x: 11,
                    y: 10,
                    floor: 7,
                },
            )
            .expect("first rat")
            .actor;
        let second = runtime
            .realize_native_qualification_spawn(
                pin.clone(),
                "fixture:rat",
                MovementLocalPosition {
                    x: 12,
                    y: 10,
                    floor: 7,
                },
            )
            .expect("second rat")
            .actor;
        runtime
            .set_test_creature_health(second, 10)
            .expect("wounded second rat");
        let mut states = ChannelSpellStates::default();
        assert!(
            states
                .initialize(
                    &runtime,
                    caster,
                    session,
                    FACTS,
                    (0, 0),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100_000)
                )
                .is_some()
        );
        wound(&mut states, caster, session, 10);
        states
            .get_mut(&runtime, caster, session)
            .expect("player")
            .install_equipped_offence(
                oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence {
                    life_leech: 300,
                    ..Default::default()
                },
            );
        let mut board = TaskBoardData::default();
        board.bounty.state = BountyStatus::Active;
        board.bounty.active = Some(BountyTask {
            race: "fixture:rat".into(),
            required: 25,
            current: 24,
            experience: 1850,
            points: 25,
            grade: 0,
            difficulty: 0,
            slot: 0,
        });
        let tracks = [
            (task_board::TALISMAN_DAMAGE.into(), 355),
            (task_board::TALISMAN_LIFE_LEECH.into(), 355),
        ]
        .into();
        let snapshot = BountyTalismanSnapshot::from_board(&board, &tracks, true);
        let root = GameplayDecisionRoot::from_bytes(pin.server_artifact_digest());
        let decision = (0..100_000_u64)
            .find_map(|index| {
                let mut bytes = [0; 16];
                bytes[..8].copy_from_slice(&index.to_be_bytes());
                let decision = DecisionOccurrenceId::from_bytes(bytes);
                (hit_bonuses(&snapshot, "fixture:rat", &root, decision, 0) == Some([5000, 5000])
                    && hit_bonuses(&snapshot, "fixture:rat", &root, decision, 1) == Some([0, 0]))
                .then_some(decision)
            })
            .expect("one proc and one ordinary target");
        let mut batch = OwnerCombatBatch {
            caster,
            attacker: character,
            current_lease_generation: 1,
            command: CommandRef::new(session, CommandId::new(1).expect("command")),
            occurrence: SpellOccurrenceBinding {
                id: "spell-batch:talisman-due".into(),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: b"qualified-talisman-due".to_vec(),
            anchor: None,
            now_us: Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100_999)),
            now_ms: 100,
            effects: vec![
                OwnerCombatEffect {
                    target: first,
                    sub_ordinal: 0,
                    change: OwnerCombatChange::Damage {
                        target_atom: "fixture:rat".into(),
                        magnitude: 25,
                    },
                },
                OwnerCombatEffect {
                    target: second,
                    sub_ordinal: 1,
                    change: OwnerCombatChange::DamageWithHealing {
                        target_atom: "fixture:rat".into(),
                        damage: 5,
                        healing: 3,
                    },
                },
            ],
            deferred: None,
        };
        let leech = apply_spell_bonuses(&runtime, &mut batch, &snapshot, &root, decision)
            .expect("qualified effects");
        assert!(matches!(
            batch.effects[0].change,
            OwnerCombatChange::Damage { magnitude: 37, .. }
        ));
        assert!(matches!(
            batch.effects[1].change,
            OwnerCombatChange::DamageWithHealing {
                damage: 5,
                healing: 3,
                ..
            }
        ));
        assert_eq!(leech, vec![(first, 0, 5000)]);
        // Completing the bounty cannot alter the already resolved physical hit.
        board.bounty.state = BountyStatus::Completed;
        assert_eq!(
            BountyTalismanSnapshot::from_board(&board, &tracks, true).bonuses("fixture:rat"),
            [0; 4]
        );
        let staged = runtime.stage_spell_batch(&batch).expect("stage");
        let receipt = runtime.commit_spell_batch(staged).expect("commit");
        assert!(receipt.applied);
        let first_health = receipt.effects[0].health.expect("first HP receipt");
        assert_eq!(
            (first_health.health_before, first_health.health_after),
            (20, 0)
        );
        assert_eq!(runtime.live_creature_health(second), Some(8));
        apply_timer_leech(&runtime, &mut states, &batch, &receipt, &leech);
        // round(20 actual HP * 53% * (0.1*2+0.9)/2) = 6. Planned damage or
        // omission of the AoE factor would each incorrectly restore 11 HP.
        assert_eq!(
            states
                .get(&runtime, caster, session)
                .expect("player")
                .health,
            16
        );
        let staged = runtime.stage_spell_batch(&batch).expect("stage replay");
        let replay = runtime.commit_spell_batch(staged).expect("commit replay");
        assert!(!replay.applied);
        apply_timer_leech(&runtime, &mut states, &batch, &replay, &leech);
        assert_eq!(
            states
                .get(&runtime, caster, session)
                .expect("player")
                .health,
            16
        );
        assert_eq!(runtime.live_creature_health(second), Some(8));
    }
}
