//! ARCH-KILL-REWARD-LOGOUT-1 §1.4: the FireReport receipt walk. A lethal due receipt appends
//! one entry in the same owner turn; a non-lethal one appends none.
#![allow(
    clippy::expect_used,
    reason = "test fixtures fail loudly on an impossible setup"
)]

use oteryn_simulation_determinism::ExactI64;

use super::super::super::kill_reward::{KillSettlementQueue, KillSink, record_batch_kills};
use crate::combat::{LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition};
use crate::content::creature_reward::{CreatureRewardRow, CreatureRewardTable};
use crate::foundation::runtime_actor_spell_types::{
    CombatBatchReceipt, OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect,
    SpellOccurrenceBinding,
};
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CharacterId, CharacterLease, CommandId,
    CommandRef, CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags, ExactActorRef,
    GameSessionId, MovementLocalPosition, NodeId, WorldId,
};

fn uuid(tag: u8) -> [u8; 16] {
    [1, 0x91, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

fn position(x: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y: 10, floor: 7 }
}

fn rat_policy(digest: [u8; 32]) -> CompiledCreaturePolicies {
    CompiledCreaturePolicies::from_active_artifact(
        digest,
        vec![CompiledCreaturePolicy {
            definition_key: "fixture:rat".into(),
            definition_revision: "fixture:1".into(),
            display_name: "fixture Rat".into(),
            maximum_health: 20,
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }],
    )
    .expect("qualified fixture table")
}

struct Fixture {
    runtime: ChannelRuntimeV1,
    caster: ExactActorRef,
    rat: ExactActorRef,
    session: GameSessionId,
}

fn fixture() -> Fixture {
    let world = WorldId::decode(&uuid(0x40)).expect("world");
    let pin = ChannelContentPin::test(world);
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(0x41)).expect("channel"),
        NodeId::decode(&uuid(0x42)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin.clone(),
    )
    .expect("runtime");
    runtime
        .install_companion_policies(rat_policy(pin.server_artifact_digest()))
        .expect("policies");
    let session = GameSessionId::decode(&uuid(0x21)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let caster = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .initialize_pinned_test_position(caster, position(10))
        .expect("position");
    let lease = CharacterLease::new(CharacterId::decode(&uuid(0x22)).expect("character"), 1)
        .expect("lease");
    runtime
        .bind_attacker_lease(caster, session, lease)
        .expect("lease bound");
    let rat = runtime
        .realize_native_qualification_spawn(pin, "fixture:rat", position(11))
        .expect("rat")
        .actor;
    Fixture {
        runtime,
        caster,
        rat,
        session,
    }
}

fn hit(f: &mut Fixture, sequence: u64, magnitude: i64) -> CombatBatchReceipt {
    let batch = OwnerCombatBatch {
        caster: f.caster,
        attacker: CharacterId::decode(&uuid(0x22)).expect("character"),
        current_lease_generation: 1,
        command: CommandRef::new(f.session, CommandId::new(sequence).expect("command")),
        occurrence: SpellOccurrenceBinding {
            id: "spell-batch:due".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"source-qualified:due-step".to_vec(),
        anchor: None,
        now_ms: 100,
        effects: vec![OwnerCombatEffect {
            target: f.rat,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: "fixture:rat".into(),
                magnitude,
            },
        }],
        deferred: None,
    };
    let staged = f.runtime.stage_spell_batch(&batch).expect("stage");
    f.runtime.commit_spell_batch(staged).expect("commit")
}

fn rewards() -> CreatureRewardTable {
    let mut table = CreatureRewardTable::default();
    table.insert(
        "fixture:rat",
        Ok(CreatureRewardRow {
            xp_amount: ExactI64::new(5),
            corpse_item: LootDefinitionRef::new("Item", "oteryn:item.corpse", "definition-r1"),
            loot_table_ref: LootDefinitionRef::new("Loot", "oteryn:loot.rat", "definition-r1"),
            loot_table: LootTableDefinition {
                algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
                entries: vec![],
            },
            race: None,
        }),
    );
    table
}

#[test]
fn a_lethal_due_receipt_appends_one_entry_and_a_non_lethal_one_none() {
    let mut f = fixture();
    let queue = KillSettlementQueue::default();
    let rewards = rewards();
    let sink = KillSink {
        queue: &queue,
        rewards: &rewards,
    };
    let wound = hit(&mut f, 1, 5);
    record_batch_kills(&mut f.runtime, sink, &wound, 1_000);
    assert_eq!(queue.counts(), (0, 0, 0));

    let lethal = hit(&mut f, 2, 1_000);
    assert!(lethal.applied);
    record_batch_kills(&mut f.runtime, sink, &lethal, 1_000);
    assert_eq!(queue.counts(), (1, 0, 0));

    // A replayed receipt of the same death enqueues nothing more.
    let mut replayed = lethal;
    replayed.applied = false;
    record_batch_kills(&mut f.runtime, sink, &replayed, 1_000);
    assert_eq!(queue.counts(), (1, 0, 0));
}

#[test]
fn a_lethal_receipt_without_a_reward_row_appends_nothing() {
    let mut f = fixture();
    let queue = KillSettlementQueue::default();
    let rewards = CreatureRewardTable::default();
    let sink = KillSink {
        queue: &queue,
        rewards: &rewards,
    };
    let lethal = hit(&mut f, 1, 1_000);
    record_batch_kills(&mut f.runtime, sink, &lethal, 1_000);
    assert_eq!(queue.counts(), (0, 0, 0));
}
