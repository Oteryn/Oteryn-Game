#![allow(
    clippy::expect_used,
    reason = "native owner test fixtures fail on invalid setup"
)]

use super::*;
use crate::foundation::runtime_actor_spell_types::{
    EffectReceipt, OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, MovementLocalPosition,
};
use oteryn_gameplay_content::content::creature_policy::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
};

fn uuid(tag: u8) -> [u8; 16] {
    [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

struct Fixture {
    runtime: ChannelRuntimeV1,
    boss: ExactActorRef,
    players: Vec<(ExactActorRef, GameSessionId, CharacterId)>,
    queue: KillSettlementQueue,
}

fn policy(reward_boss: bool) -> CompiledCreaturePolicy {
    CompiledCreaturePolicy {
        definition_key: "creature:boss-contribution".into(),
        definition_revision: "test:boss-contribution-v1".into(),
        display_name: "contribution test boss".into(),
        maximum_health: 20,
        forge_difficulty: None,
        base_speed: 100,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        preferred_distance: Some(1),
        reward_boss: Some(reward_boss),
        blocked_by_nearby_players: false,
        condition_immunities: vec![],
        defense: Some(0),
        armor: Some(0),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    }
}

fn fixture(reward_boss: bool) -> Fixture {
    let (mut runtime, first, first_session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(50, 8);
    runtime
        .initialize_first_entry_position(first)
        .expect("first entry");
    let mut players = Vec::new();
    for tag in [50, 51] {
        let (actor, session) = if tag == 50 {
            (first, first_session)
        } else {
            let session = GameSessionId::decode(&uuid(tag)).expect("session");
            let reservation = runtime.reserve_fresh_session(session).expect("reservation");
            let actor = runtime.commit_fresh_session(reservation).expect("player");
            runtime
                .initialize_first_entry_position(actor)
                .expect("entry");
            (actor, session)
        };
        let character = CharacterId::decode(&uuid(tag + 10)).expect("character");
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(character, 1).expect("lease"),
            )
            .expect("current native lease");
        players.push((actor, session, character));
    }
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                vec![policy(reward_boss)],
            )
            .expect("active policy"),
        )
        .expect("policy binding");
    let boss = runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 1,
            y: 0,
            floor: 0,
        })
        .expect("native creature");
    runtime
        .install_creature_policy(boss, "creature:boss-contribution")
        .expect("boss policy");
    let queue = KillSettlementQueue::default();
    queue.boss_targets(&runtime, boss, players.iter().map(|(a, s, _)| (*a, *s)));
    Fixture {
        runtime,
        boss,
        players,
        queue,
    }
}

fn receipt(f: &Fixture, target: ExactActorRef, before: i64, after: i64) -> CombatBatchReceipt {
    CombatBatchReceipt {
        applied: true,
        caster: f.players[0].0,
        command: CommandRef::new(f.players[0].1, CommandId::new(1).expect("command")),
        effects: vec![EffectReceipt {
            target,
            sub_ordinal: 0,
            health: Some(crate::foundation::OwnerDamageResult {
                applied: true,
                health_before: before,
                health_after: after,
            }),
        }],
    }
}

fn frozen(f: &Fixture) -> Vec<FrozenBossParticipant> {
    f.queue
        .freeze_boss_participation(f.boss, &[])
        .expect("complete census")
}

#[test]
fn scheduled_source_damage_receipts_credit_boss_participation_once() {
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::player_lethal::{PlayerHitKind, PlayerLethalVitals};
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

    let mut f = fixture(true);
    let (actor, session, character) = f.players[0];
    let mut states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
    let mut attack = crate::gameplay_transport::attack::ChannelAttackStates::default();
    let now = SemanticTimeMicros::from_micros(10);
    let player = crate::gameplay_transport::actor_spell::tests::FACTS;
    states
        .initialize(
            &f.runtime,
            actor,
            session,
            player,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now.get()),
        )
        .expect("current player vitals");
    attack
        .kills()
        .boss_targets(&f.runtime, f.boss, [(actor, session)].into_iter());
    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()
        .expect("canonical charm catalogue");
    let digest = f.runtime.content_pin().server_artifact_digest();
    let root = GameplayDecisionRoot::from_bytes(digest);
    let facts = crate::foundation::ApplicationFacts {
        now: now.get(),
        base_speed: states
            .get(&f.runtime, actor, session)
            .expect("current player")
            .owned_base_speed()
            .try_into()
            .expect("player base speed"),
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes(actor.placement_identity()),
    };
    let secondary = {
        let mut vitals =
            crate::gameplay_transport::monster_profile_cycle::physical_spell::source_vitals(
                &mut states,
                &mut attack,
                &catalogue,
                GameplayDecisionRoot::from_bytes(digest),
            );
        let secondary = vitals
            .apply_source_typed_attack_damage(
                &mut f.runtime,
                f.boss,
                actor,
                session,
                12,
                PlayerHitKind::Physical,
                "fixture:boss-secondary:1",
                now,
            )
            .expect("real secondary HP receipt");
        assert_eq!(secondary.applied, 12);
        let stamp = f.runtime.issue_owner_work().expect("current chain work");
        let chain = vitals
            .apply_typed_composite_attack_damage_current(
                &mut f.runtime,
                actor,
                session,
                7,
                PlayerHitKind::Physical,
                "fixture:boss-chain:1",
                f.boss,
                &[],
                &facts,
                &[],
                stamp,
            )
            .expect("real delayed HP receipt");
        assert_eq!(chain.applied, 7);
        secondary
    };
    assert_eq!(
        states
            .get(&f.runtime, actor, session)
            .expect("player")
            .vitals()
            .health,
        player.max_health - 19,
        "both real HP commits applied only once"
    );
    // The existing immediate/cached-outcome observer can see an older receipt
    // after a delayed hit. Re-observing that real receipt must not add it again.
    for _ in 0..2 {
        attack.kills().boss_damage_received(
            &f.runtime,
            f.boss,
            actor,
            session,
            u64::from(secondary.applied),
            secondary.vitals_revision,
        );
    }
    let participants = attack
        .kills()
        .freeze_boss_participation(f.boss, &[])
        .expect("boss participation");
    let participant = participants
        .iter()
        .find(|participant| participant.character == character)
        .expect("current boss participant");
    assert_eq!(participant.contribution.damage_received, 19);
}

#[test]
fn boss_contribution_counts_new_received_damage_only_and_rejects_foreign_session() {
    let f = fixture(true);
    let (actor, session, character) = f.players[0];
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 12, 1);
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 12, 1);
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 7, 2);
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 999, 1);
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, f.players[1].1, 999, 3);
    let participants = frozen(&f);
    let participant = participants
        .iter()
        .find(|p| p.character == character)
        .expect("participant");
    assert_eq!(participant.contribution.damage_received, 19);
    assert_eq!(participant.session, session);
}

#[test]
fn boss_contribution_ignores_ordinary_monsters_and_self_healing() {
    let ordinary = fixture(false);
    let (actor, session, _) = ordinary.players[0];
    ordinary
        .queue
        .boss_damage_received(&ordinary.runtime, ordinary.boss, actor, session, 20, 1);
    assert!(frozen(&ordinary).is_empty());
    let f = fixture(true);
    f.queue
        .boss_spell_contribution(&f.runtime, &receipt(&f, f.players[0].0, 20, 40));
    assert!(
        frozen(&f)
            .iter()
            .all(|p| p.contribution.healing_others == 0)
    );
}

#[test]
fn boss_contribution_healing_requires_an_active_target_and_new_applied_receipt() {
    let f = fixture(true);
    let healed = receipt(&f, f.players[1].0, 20, 32);
    f.queue.boss_spell_contribution(&f.runtime, &healed);
    let mut replay = healed.clone();
    replay.applied = false;
    f.queue.boss_spell_contribution(&f.runtime, &replay);
    let mut uncommitted = healed.clone();
    uncommitted.effects[0]
        .health
        .as_mut()
        .expect("health")
        .applied = false;
    f.queue.boss_spell_contribution(&f.runtime, &uncommitted);
    f.queue.boss_targets(&f.runtime, f.boss, std::iter::empty());
    f.queue.boss_spell_contribution(&f.runtime, &healed);
    let participants = frozen(&f);
    assert_eq!(
        participants
            .iter()
            .find(|p| p.character == f.players[0].2)
            .expect("healer")
            .contribution
            .healing_others,
        12
    );
}

#[test]
fn boss_contribution_native_damage_ledger_survives_death_and_replay_without_duplicate_score() {
    let mut f = fixture(true);
    let (actor, session, character) = f.players[0];
    let batch = OwnerCombatBatch {
        caster: actor,
        attacker: character,
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(1).expect("command")),
        occurrence: SpellOccurrenceBinding {
            id: "boss-contribution:lethal".into(),
            revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
        },
        binding: b"native-boss-contribution-lethal".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 100,
        effects: vec![OwnerCombatEffect {
            target: f.boss,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: f
                    .runtime
                    .creature_spell_target_atom(f.boss)
                    .expect("target atom"),
                magnitude: 100,
            },
        }],
        deferred: None,
    };
    let staged = f
        .runtime
        .stage_spell_batch(&batch)
        .expect("stage native damage");
    let hit = f
        .runtime
        .commit_spell_batch(staged)
        .expect("committed native damage");
    f.queue.boss_spell_contribution(&f.runtime, &hit);
    let staged = f.runtime.stage_spell_batch(&batch).expect("replay stage");
    let replay = f
        .runtime
        .commit_spell_batch(staged)
        .expect("replay receipt");
    assert!(!replay.applied);
    f.queue.boss_spell_contribution(&f.runtime, &replay);
    let shares = f
        .runtime
        .borrow_combat_death()
        .reward_damage_shares(f.boss)
        .expect("native shares");
    let participants = f
        .queue
        .freeze_boss_participation(f.boss, &shares)
        .expect("frozen death");
    assert_eq!(
        participants
            .iter()
            .find(|p| p.character == character)
            .expect("damage dealer")
            .contribution
            .damage_dealt,
        20
    );
    assert!(f.runtime.companion_snapshot(f.boss).is_err());
}

#[test]
fn boss_contribution_freeze_is_value_owned_and_refuses_unqualified_damage_identity() {
    let f = fixture(true);
    let (actor, session, character) = f.players[0];
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 10, 1);
    let frozen = frozen(&f);
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 10, 2);
    assert_eq!(
        frozen
            .iter()
            .find(|p| p.character == character)
            .expect("frozen participant")
            .contribution
            .damage_received,
        10
    );
    let unqualified = crate::foundation::RewardDamageShare {
        character,
        total: 20,
        session: None,
        lease_generation: None,
        last_damage_at_ms: Some(100),
    };
    assert!(
        f.queue
            .freeze_boss_participation(f.boss, &[unqualified])
            .is_none()
    );
}

#[test]
fn boss_contribution_healing_selects_one_active_fight_in_registration_order() {
    let mut f = fixture(true);
    let second = f
        .runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: 2,
            y: 0,
            floor: 0,
        })
        .expect("second native boss");
    f.runtime
        .install_creature_policy(second, "creature:boss-contribution")
        .expect("second boss policy");
    f.queue.boss_targets(
        &f.runtime,
        second,
        f.players.iter().map(|(a, s, _)| (*a, *s)),
    );
    f.queue
        .boss_spell_contribution(&f.runtime, &receipt(&f, f.players[1].0, 20, 32));
    let first = frozen(&f);
    let second = f
        .queue
        .freeze_boss_participation(second, &[])
        .expect("second census");
    assert_eq!(
        first
            .iter()
            .find(|p| p.character == f.players[0].2)
            .expect("first healer")
            .contribution
            .healing_others,
        12
    );
    assert!(second.iter().all(|p| p.contribution.healing_others == 0));
}

#[test]
fn boss_contribution_reconnect_retains_character_score_but_rejects_the_old_session() {
    let mut f = fixture(true);
    let (old_actor, old_session, character) = f.players[0];
    f.queue
        .boss_damage_received(&f.runtime, f.boss, old_actor, old_session, 12, 8);
    f.runtime
        .remove_terminal_session(old_session, old_actor)
        .expect("terminal native removal");
    let session = GameSessionId::decode(&uuid(52)).expect("successor session");
    let reservation = f
        .runtime
        .reserve_fresh_session(session)
        .expect("fresh reservation");
    let actor = f
        .runtime
        .commit_fresh_session(reservation)
        .expect("fresh actor");
    f.runtime
        .initialize_first_entry_position(actor)
        .expect("fresh position");
    f.runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(character, 2).expect("new lease"),
        )
        .expect("current successor binding");
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 7, 1);
    f.queue
        .boss_damage_received(&f.runtime, f.boss, old_actor, old_session, 999, 9);
    let participants = frozen(&f);
    let participant = participants
        .iter()
        .find(|p| p.character == character)
        .expect("same character");
    assert_eq!(participant.contribution.damage_received, 19);
    assert_eq!(
        (participant.session, participant.lease_generation),
        (session, 2)
    );
}

#[test]
fn boss_contribution_freezes_timestamp_table_and_census_after_native_death_for_later_replay() {
    let mut f = fixture(true);
    let (actor, session, character) = f.players[0];
    let reward = Arc::new(CreatureRewardRow {
        xp_amount: oteryn_simulation_determinism::ExactI64::new(5),
        corpse_item: crate::combat::LootDefinitionRef::new("Item", "oteryn:item.corpse", "r1"),
        loot_table_ref: crate::combat::LootDefinitionRef::new("Loot", "oteryn:loot.boss", "r1"),
        loot_table: crate::combat::LootTableDefinition {
            algorithm: crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![],
        },
        race: None,
        boss: None,
    });
    assert!(
        f.queue
            .freeze_boss_death(&f.runtime, f.boss, &[], Arc::clone(&reward))
            .is_none(),
        "live bosses have no reward death"
    );
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 10, 1);
    let batch = OwnerCombatBatch {
        caster: actor,
        attacker: character,
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(1).expect("command")),
        occurrence: SpellOccurrenceBinding {
            id: "boss-freeze:lethal".into(),
            revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
        },
        binding: b"boss-freeze-lethal".to_vec(),
        anchor: None,
        now_us: None,
        now_ms: 100,
        effects: vec![OwnerCombatEffect {
            target: f.boss,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: f
                    .runtime
                    .creature_spell_target_atom(f.boss)
                    .expect("target atom"),
                magnitude: 100,
            },
        }],
        deferred: None,
    };
    let staged = f.runtime.stage_spell_batch(&batch).expect("native stage");
    let receipt = f.runtime.commit_spell_batch(staged).expect("native lethal");
    f.queue.boss_spell_contribution(&f.runtime, &receipt);
    crate::combat::project_fixed_one_creature_death(&mut f.runtime.borrow_combat_death(), f.boss)
        .expect("real projection");
    let shares = f
        .runtime
        .borrow_combat_death()
        .reward_damage_shares(f.boss)
        .expect("real shares");
    let first = f
        .queue
        .freeze_boss_death(&f.runtime, f.boss, &shares, Arc::clone(&reward))
        .expect("first freeze");
    f.queue
        .boss_damage_received(&f.runtime, f.boss, actor, session, 999, 2);
    f.queue.boss_targets(&f.runtime, f.boss, std::iter::empty());
    let mut changed = (*reward).clone();
    changed.xp_amount = oteryn_simulation_determinism::ExactI64::new(999);
    std::thread::sleep(std::time::Duration::from_millis(2));
    let replay = f
        .queue
        .freeze_boss_death(&f.runtime, f.boss, &[], Arc::new(changed))
        .expect("later replay");
    assert_eq!(
        first, replay,
        "later queue insertion cannot change expiry, loot or contribution"
    );
    let dealer = replay
        .participants
        .iter()
        .find(|row| row.character == character)
        .expect("dealer");
    assert_eq!(dealer.contribution.damage_received, 10);
    assert_eq!(dealer.contribution.damage_dealt, 20);
}
