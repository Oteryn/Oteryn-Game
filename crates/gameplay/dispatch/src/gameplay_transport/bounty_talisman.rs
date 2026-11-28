//! P05: the accepted NEWS-HIST-TALISMAN-1 profile, confirmed by PR 2764
//! comments 6542770501 and 6542842489. Damage and life leech have independent
//! 15%/10% activation draws; retries use the original physical occurrence.

use crate::durability::character_progression::CharacterProgressionError;
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
    /// Publish the quest refresh's committed bonuses for future swings. The
    /// combat pass only reads the session snapshot under its usual owner guards.
    pub(in crate::gameplay_transport) async fn bounty_talisman_for_session(
        &self,
        session: GameSessionId,
    ) -> Result<BountyTalismanSnapshot, CharacterProgressionError> {
        let Some(fence) = self.current_quest_fence_durable(session).await? else {
            return Ok(BountyTalismanSnapshot::default());
        };
        let snapshot = self
            .root
            .read_bounty_talisman(self.character, self.holder, fence)
            .await?;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == fence.runtime_scope
            && runtime.binding().scope_generation() == fence.scope_ownership_generation
            && let Ok(Some((actor, _))) = runtime.positioned_player_for_session(session)
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            states.set_elite_bounty(&runtime, session, snapshot.elite_race());
            states
                .swing_talismans
                .insert(session, (actor, snapshot.clone()));
        }
        Ok(snapshot)
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

/// Canary on_combat_taskboard.lua:17-20 and Crystal combat.cpp:918-923.
pub(in crate::gameplay_transport) fn damage(amount: i64, bonus: u16) -> i64 {
    crate::spell::magnitude_owner::boost_creature_damage(amount, 0, bonus)
}

/// Scale retained leech damage beside its hit, excluding only the separate Prey
/// stage. Existing Talisman/elite policy and deterministic draws stay unchanged.
pub(in crate::gameplay_transport) fn apply_spell_bonuses_with_leech(
    runtime: &ChannelRuntimeV1,
    batch: &mut OwnerCombatBatch,
    snapshot: &BountyTalismanSnapshot,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    leech_inputs: &mut [crate::gameplay_transport::item_spell_augments::leech_projection::Target],
    (states, mut owner): (
        &crate::gameplay_transport::actor_spell::ChannelSpellStates,
        Option<&mut crate::spell::magnitude_owner::PreparedMagnitudeOwner>,
    ),
) -> Option<Vec<SpellLeech>> {
    let mut leech = Vec::new();
    let mut cursor = 0;
    for effect in &mut batch.effects {
        let (magnitude, heals) = match &mut effect.change {
            OwnerCombatChange::Damage { magnitude, .. } => (magnitude, false),
            OwnerCombatChange::DamageWithHealing { damage, .. } => (damage, true),
            _ => continue,
        };
        let Ok(creature) = runtime.companion_snapshot(effect.target) else {
            continue;
        };
        let race = &creature.state.policy.definition_key;
        let [extra_damage, extra_life] = hit_bonuses(
            snapshot,
            race,
            root,
            occurrence,
            effect.sub_ordinal,
        )?;
        let elite = snapshot.elite(race);
        let prey = states.prey_percent(
            runtime,
            batch.caster,
            batch.command.game_session_id(),
            race,
            1,
            batch
                .now_us
                .map_or_else(|| batch.now_ms.saturating_mul(1_000), |at| at.get()),
        );
        let parts = if let Some(owner) = owner.as_deref_mut() {
            owner
                .apply_creature_damage_modifiers(
                    &mut cursor,
                    effect.target,
                    effect.sub_ordinal,
                    *magnitude,
                    (prey, extra_damage, elite),
                )
                .ok()?
        } else {
            None
        };
        let finish = |amount| {
            let amount = damage(amount, extra_damage);
            if elite { amount / 2 } else { amount }
        };
        *magnitude = parts.map_or_else(
            || {
                let base = if heals {
                    *magnitude
                } else {
                    crate::gameplay_transport::prey::damage(*magnitude, prey)
                };
                finish(base)
            },
            |(amount, _)| amount,
        );
        if let Some(input) = leech_inputs
            .iter_mut()
            .find(|input| input.target == effect.target && input.ordinal == effect.sub_ordinal)
            && let Some(basis) = input.damage.as_mut()
        {
            let scaled = parts.map_or_else(|| finish(i64::from(*basis)), |(_, leech)| leech);
            *basis = u32::try_from(scaled).unwrap_or(u32::MAX);
        }
        if extra_life > 0 {
            leech.push((effect.target, effect.sub_ordinal, extra_life));
        }
    }
    Some(leech)
}

impl crate::gameplay_transport::actor_spell::ChannelSpellStates {
    pub(in crate::gameplay_transport) fn set_elite_bounty(
        &mut self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
        race: Option<&str>,
    ) {
        self.elite_bounties.retain(|(actor, id, _)| {
            *id != session && runtime.player_control_facts(*actor, *id).is_ok()
        });
        if let Some(race) = race
            && let Ok(Some((actor, _))) = runtime.positioned_player_for_session(session)
            && runtime.player_control_facts(actor, session).is_ok()
        {
            self.elite_bounties.push((actor, session, race.to_owned()));
        }
    }
    /// The condition owner has already qualified the tick. Historical race facts
    /// identify its creature even after that exact source generation retires.
    pub(in crate::gameplay_transport) fn elite_condition_damage(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        session: GameSessionId,
        provenance: &crate::foundation::condition::ConditionProvenance<String>,
        amount: u32,
    ) -> u32 {
        use crate::foundation::condition::{ConditionHistoricalSource, ConditionSourceKind};
        if provenance.source_kind != ConditionSourceKind::Creature {
            return amount;
        }
        let race = match provenance.historical_source.as_ref() {
            Some(ConditionHistoricalSource::Creature { key, .. }) => Some(key.as_str()),
            Some(ConditionHistoricalSource::Unpublishable {
                creature_key: Some(key),
                ..
            }) => Some(key.as_str()),
            _ => None,
        };
        let elite = if let Some(race) = race {
            self.elite_bounty(runtime, target, session, race)
        } else {
            // Older live-source conditions have an exact placement label but no history.
            runtime.visible_entities().creatures.iter().any(|creature| {
                provenance.source.as_deref()
                    == Some(
                        format!(
                            "creature:{}",
                            creature
                                .actor
                                .placement_identity()
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>()
                        )
                        .as_str(),
                    )
                    && runtime
                        .companion_snapshot(creature.actor)
                        .is_ok_and(|source| {
                            self.elite_bounty(
                                runtime,
                                target,
                                session,
                                &source.state.policy.definition_key,
                            )
                        })
            })
        };
        if elite {
            amount.saturating_mul(2)
        } else {
            amount
        }
    }
    pub(in crate::gameplay_transport) fn elite_bounty(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        race: &str,
    ) -> bool {
        self.get(runtime, actor, session).is_some()
            && self
                .elite_bounties
                .iter()
                .any(|(present, id, key)| *present == actor && *id == session && key == race)
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
    fn talisman_life_uses_selected_ceiling_rule_combined_skill_and_aoe_factor() {
        use oteryn_gameplay_combat_rules::resolution::equipped_offence::leech_amount_for_targets as life_amount;
        assert_eq!(life_amount(100, 300, 1), 3);
        assert_eq!(life_amount(100, 300, 2), 2);
        assert_eq!(life_amount(1, 300, 1), 1);
        assert_eq!(life_amount(100, 5300, 2), 30);
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
            elite: false,
            dust: 0,
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
    fn talisman_joined_batch_uses_pre_overkill_aoe_and_replay_receipts() {
        timer_batch_case(false);
    }

    #[test]
    fn elite_spell_batch_halves_actual_hp_and_replay() {
        timer_batch_case(true);
    }

    fn timer_batch_case(elite: bool) {
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
        use crate::gameplay_transport::actor_spell::{commit_owner_batch, stage_player_batch};
        use crate::gameplay_transport::item_spell_augments::leech_projection;
        use oteryn_gameplay_content::content::creature_policy::{
            CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
        };
        use oteryn_gameplay_quests::quest::task_board::{
            self,
            full::{BountyStatus, BountyTask, TaskBoardData},
        };
        // Activation retains half the carrier as player headroom: three actors need six slots.
        let (mut runtime, caster, session) = runtime_with_capacity(73, 6);
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
        runtime
            .realize_activation_spawns(
                [11, 12]
                    .into_iter()
                    .map(|x| {
                        (
                            vec![(x, 10, 7)],
                            60_000,
                            5_000,
                            "fixture:rat".to_owned(),
                            20,
                        )
                    })
                    .collect(),
            )
            .expect("distinct rat sources");
        let rats = runtime.spawn_point_creatures();
        assert_eq!(rats.len(), 2);
        let first = rats[0].2.expect("first rat");
        let second = rats[1].2.expect("second rat");
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
            elite,
            dust: 0,
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
                // A neutral player effect stages a genuine player-vitals successor;
                // the retained Leech then joins that same physical/player commit.
                OwnerCombatEffect {
                    target: caster,
                    sub_ordinal: 2,
                    change: OwnerCombatChange::Heal {
                        target_atom: crate::spell::combat_execution::actor_atom(caster),
                        magnitude: 0,
                    },
                },
            ],
            deferred: None,
        };
        let mut inputs = leech_projection::capture(
            &runtime,
            &states,
            &crate::content::charm_source::CanonicalCharmCatalogue::embedded().expect("catalogue"),
            &batch,
            (300, 0),
            None,
            &root,
            decision,
        )
        .expect("source Leech before Prey and HP clipping");
        let leech = apply_spell_bonuses_with_leech(
            &runtime,
            &mut batch,
            &snapshot,
            &root,
            decision,
            &mut inputs,
            (&states, None),
        )
        .expect("qualified effects");
        assert!(matches!(
            batch.effects[0].change,
            OwnerCombatChange::Damage { magnitude, .. } if magnitude == if elite { 18 } else { 37 }
        ));
        assert!(matches!(
            batch.effects[1].change,
            OwnerCombatChange::DamageWithHealing {
                damage,
                healing: 3,
                ..
            } if damage == if elite { 2 } else { 5 }
        ));
        assert_eq!(leech, vec![(first, 0, 5000)]);
        // Completing the bounty cannot alter the already resolved physical hit.
        board.bounty.state = BountyStatus::Completed;
        assert_eq!(
            BountyTalismanSnapshot::from_board(&board, &tracks, true).bonuses("fixture:rat"),
            [0; 4]
        );
        let staged = runtime.stage_spell_batch(&batch).expect("stage");
        let mut player =
            stage_player_batch(&runtime, &states, &batch, None).expect("matching player successor");
        player
            .stage_item_spell_leech(&runtime, &staged, &inputs, &leech)
            .expect("retained Leech joined before commit");
        assert_eq!(
            states
                .get(&runtime, caster, session)
                .expect("player")
                .health,
            10
        );
        assert_eq!(runtime.live_creature_health(first), Some(20));
        assert_eq!(runtime.live_creature_health(second), Some(10));
        let receipt = commit_owner_batch(&mut runtime, &mut states, staged, Some(player))
            .expect("atomic physical and player commit");
        assert!(receipt.applied);
        let first_health = receipt.effects[0].health.expect("first HP receipt");
        assert_eq!(
            (first_health.health_before, first_health.health_after),
            (20, if elite { 2 } else { 0 })
        );
        assert_eq!(
            runtime.live_creature_health(second),
            Some(if elite { 11 } else { 8 })
        );
        // The selected Global profile uses pre-overkill source damage and ceilings:
        // non-elite: ceil(37 * 53% * 0.55) + ceil(5 * 3% * 0.55) = 12;
        // elite: ceil(18 * 53% * 0.55) + ceil(2 * 3% * 0.55) = 7.
        // Ordinary Leech remains on the non-proccing target, and DamageWithHealing
        // changes its physical HP without changing the retained damage basis.
        assert_eq!(
            states
                .get(&runtime, caster, session)
                .expect("player")
                .health,
            if elite { 17 } else { 22 }
        );
        let staged = runtime.stage_spell_batch(&batch).expect("stage replay");
        let before_replay = states
            .get(&runtime, caster, session)
            .expect("player")
            .clone();
        let replay = commit_owner_batch(&mut runtime, &mut states, staged, None)
            .expect("joined receipt replay");
        assert!(!replay.applied);
        assert_eq!(states.get(&runtime, caster, session), Some(&before_replay));
        assert_eq!(
            states
                .get(&runtime, caster, session)
                .expect("player")
                .health,
            if elite { 17 } else { 22 }
        );
        assert_eq!(
            runtime.live_creature_health(second),
            Some(if elite { 11 } else { 8 })
        );
    }
}
