use super::boss_reward_contribution::{FrozenBossDeath, FrozenBossParticipant};
use super::*;
use crate::domain::item_definition::ItemStackClass;
use oteryn_gameplay_domain::domain::loot_data::{
    LootDefinitionRef, LootEntryDetails, LootSelectionAlgorithm, LootTableDefinition,
    LootTableEntry,
};
use oteryn_gameplay_encounters::bosses::reward_chest::RewardParticipation;
use oteryn_gameplay_items::loot_plan::LootPlanEntry;

fn id(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
fn participant(seed: u8, damage: u64, received: u64, healing: u64) -> FrozenBossParticipant {
    FrozenBossParticipant {
        character: crate::foundation::CharacterId::decode(&id(seed)).expect("character"),
        session: GameSessionId::decode(&id(seed + 1)).expect("session"),
        lease_generation: 1,
        contribution: RewardParticipation {
            damage_dealt: damage,
            damage_received: received,
            healing_others: healing,
        },
    }
}
fn definition() -> LootDefinitionRef {
    LootDefinitionRef::new("Item", "oteryn:item.tibia.i3031", "definition-r1")
}
fn table_reference() -> LootDefinitionRef {
    LootDefinitionRef::new("Loot", "oteryn:loot.boss-fixture", "definition-r1")
}
fn frozen() -> FrozenBossDeath {
    FrozenBossDeath {
        participants: vec![
            participant(10, 100, 0, 0),
            participant(20, 0, 100, 100),
            participant(30, 1, 0, 0),
        ],
        died_at_unix_ms: 1_000_000,
        reward: Arc::new(CreatureRewardRow {
            xp_amount: oteryn_simulation_determinism::ExactI64::new(100),
            corpse_item: LootDefinitionRef::new("Item", "oteryn:item.tibia.i3058", "definition-r1"),
            loot_table_ref: table_reference(),
            loot_table: LootTableDefinition {
                algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
                entries: vec![LootTableEntry {
                    item: definition(),
                    min_count: 1,
                    max_count: 350,
                    probability_ppm: Some(1_000_000),
                    details: LootEntryDetails {
                        stackable: true,
                        ..Default::default()
                    },
                }],
            },
            race: None,
            boss: None,
        }),
    }
}
fn facts(definition: &TypedDefinitionRef, stack: ItemStackClass) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        weight: Some(1),
        definition: definition.clone(),
        stack,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}
fn selected(quantities: &[u32]) -> LootPlan {
    LootPlan {
        loot_table: table_reference(),
        entries: quantities
            .iter()
            .enumerate()
            .map(|(i, quantity)| LootPlanEntry {
                item: definition(),
                quantity: *quantity,
                draw_ordinal: i as u32,
                purpose_key: definition().production_key,
                parent: None,
                placement_ordinal: (i + 1) as u32,
                subtype: None,
                charges: None,
                unique: false,
                stackable: true,
            })
            .collect(),
    }
}

#[test]
fn boss_reward_settlement_retains_full_census_and_replays_independently_of_input_order() {
    let mut frozen = frozen();
    let first = plan_personal_death(id(90), "oteryn:boss.fixture", &frozen, |definition| {
        Some(facts(
            definition,
            ItemStackClass::Stackable {
                proven_maximum: None,
            },
        ))
    })
    .expect("plan");
    assert_eq!(
        first.contributors.len(),
        3,
        "support and below-threshold damage remain in the frozen denominator"
    );
    assert_eq!(
        first.chests.len(),
        1,
        "accepted anti-leech gate remains separate from donor loot scaling"
    );
    assert_eq!(
        first.chests[0].character_id.as_bytes(),
        frozen.participants[0].character.as_bytes()
    );
    assert!(
        first.chests[0]
            .items
            .iter()
            .all(|item| item.quantity <= 100)
    );
    frozen.participants.reverse();
    let replay = plan_personal_death(id(90), "oteryn:boss.fixture", &frozen, |definition| {
        Some(facts(
            definition,
            ItemStackClass::Stackable {
                proven_maximum: None,
            },
        ))
    })
    .expect("replay");
    assert_eq!(replay, first);
    frozen.participants.push(frozen.participants[0].clone());
    assert!(matches!(
        plan_personal_death(id(90), "oteryn:boss.fixture", &frozen, |definition| Some(
            facts(definition, ItemStackClass::NonStackable)
        )),
        Err(PlanError::InvalidCensus)
    ));
}

#[test]
fn boss_reward_settlement_freeze_cannot_persist_a_failed_roll_as_an_empty_chest() {
    let mut frozen = frozen();
    frozen.participants = vec![
        participant(10, 100, 100, 100),
        participant(20, 100, 100, 100),
    ];
    let calls = std::cell::Cell::new(0);
    let result = plan_personal_death(id(90), "oteryn:boss.fixture", &frozen, |definition| {
        calls.set(calls.get() + 1);
        (calls.get() == 1).then(|| {
            facts(
                definition,
                ItemStackClass::Stackable {
                    proven_maximum: None,
                },
            )
        })
    });
    assert_eq!(
        calls.get(),
        2,
        "both eligible donor rolls reach the pinned Item facts"
    );
    assert!(
        matches!(result, Err(PlanError::ContentUnavailable)),
        "the second participant's unresolved Item rejects the complete immutable death plan"
    );
}

#[test]
fn boss_reward_settlement_stack_splitting_preserves_all_selected_quantity_and_bounds() {
    let stacks = materialize_plan(selected(&[150, 75]), &|definition| {
        Some(facts(
            definition,
            ItemStackClass::Stackable {
                proven_maximum: None,
            },
        ))
    })
    .expect("default stacks");
    assert_eq!(
        stacks.iter().map(|item| item.quantity).collect::<Vec<_>>(),
        vec![100, 100, 25]
    );
    let custom = materialize_plan(selected(&[75, 75]), &|definition| {
        Some(facts(
            definition,
            ItemStackClass::Stackable {
                proven_maximum: Some(80),
            },
        ))
    })
    .expect("proven stacks");
    assert_eq!(
        custom.iter().map(|item| item.quantity).collect::<Vec<_>>(),
        vec![80, 70]
    );
    let separate = materialize_plan(selected(&[1, 1]), &|definition| {
        Some(facts(definition, ItemStackClass::NonStackable))
    })
    .expect("separate Items");
    assert_eq!(separate.len(), 2);
    assert!(separate.iter().all(|item| item.quantity == 1));
    assert!(matches!(
        materialize_plan(selected(&[100_001]), &|definition| Some(facts(
            definition,
            ItemStackClass::Stackable {
                proven_maximum: None
            }
        ))),
        Err(PlanError::TooManyItems)
    ));
    assert!(matches!(
        materialize_plan(selected(&[1]), &|definition| Some(facts(
            definition,
            ItemStackClass::Unknown
        ))),
        Err(PlanError::ContentUnavailable)
    ));
    assert!(matches!(
        materialize_plan(selected(&[1]), &|definition| Some(facts(
            definition,
            ItemStackClass::Stackable {
                proven_maximum: Some(0)
            }
        ))),
        Err(PlanError::ContentUnavailable)
    ));
    assert!(
        matches!(
            materialize_plan(selected(&[101]), &|definition| Some(facts(
                definition,
                ItemStackClass::Stackable {
                    proven_maximum: Some(101)
                }
            ))),
            Err(PlanError::ContentUnavailable)
        ),
        "the death planner must not freeze a stack definition the existing custody owner refuses"
    );
}

#[test]
fn boss_reward_settlement_same_scope_bytes_cannot_alias_channel_and_instance_deaths() {
    let world = crate::foundation::WorldId::decode(&id(1)).expect("world");
    let channel = RuntimeScopeRefV1::channel(
        world,
        crate::foundation::ChannelId::decode(&id(2)).expect("channel"),
    );
    let instance = RuntimeScopeRefV1::instance(world, id(2)).expect("instance");
    let first = scoped_death_id(channel, 1, 1, 1);
    assert_ne!(first, scoped_death_id(instance, 1, 1, 1));
    assert_ne!(first, scoped_death_id(channel, 2, 1, 1));
    assert_ne!(first, scoped_death_id(channel, 1, 1, 2));
    assert_ne!(first, scoped_death_id(channel, 1, 2, 1));
    assert_eq!(first, scoped_death_id(channel, 1, 1, 1));
}

fn moonsilver_entry(probability_ppm: u32) -> LootTableEntry {
    LootTableEntry {
        item: LootDefinitionRef::new("Item", "oteryn:item.tibia.i53229", "definition-r1"),
        min_count: 1,
        max_count: 1,
        probability_ppm: Some(probability_ppm),
        details: LootEntryDetails::default(),
    }
}

#[test]
fn pity_raises_only_moonsilver_entries_of_the_recipient_table() {
    let mut table = frozen().reward.loot_table.clone();
    assert_eq!(pity_loot_table(&table, 2_000), None, "no Moonsilver entry");
    table.entries.push(moonsilver_entry(3_000));
    assert_eq!(
        pity_loot_table(&table, 0),
        None,
        "no bonus keeps the base table"
    );
    for (bonus, expected) in [(100, 3_300), (1_000, 6_000), (2_000, 9_000)] {
        let boosted = pity_loot_table(&table, bonus).expect("boosted table");
        assert_eq!(boosted.entries[1].probability_ppm, Some(expected));
        assert_eq!(
            boosted.entries[0], table.entries[0],
            "gold keeps its chance"
        );
    }
}

#[test]
fn pity_bonus_reaches_the_personal_roll_and_only_adds_drops() {
    let mut frozen = frozen();
    frozen.participants = vec![participant(10, 100, 0, 0)];
    let reward = Arc::make_mut(&mut frozen.reward);
    reward.loot_table.entries = vec![moonsilver_entry(300_000)];
    let character = *frozen.participants[0].character.as_bytes();
    let plan = |death: [u8; 16], pity: &[([u8; 16], u16)]| {
        plan_personal_death_with_slots(
            death,
            "oteryn:boss.mimar_haffar",
            &frozen,
            |definition| Some(facts(definition, ItemStackClass::NonStackable)),
            &[],
            pity,
            |_| false,
        )
        .expect("plan")
        .chests[0]
            .items
            .len()
    };
    let (mut base_drops, mut pity_drops) = (0, 0);
    for seed in 0..64 {
        let death = id(100 + seed);
        let base = plan(death, &[]);
        let boosted = plan(death, &[(character, 2_000)]);
        assert!(boosted >= base, "the same draw never loses a base drop");
        assert_eq!(
            plan(death, &[(id(99), 2_000)]),
            base,
            "another recipient's pity"
        );
        base_drops += base;
        pity_drops += boosted;
    }
    assert!(pity_drops > base_drops, "{pity_drops} > {base_drops}");
}
