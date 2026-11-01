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
