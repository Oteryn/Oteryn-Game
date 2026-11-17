//! Donor odds and boundary draws shared with the durable NPC exchange.
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
use oteryn_gameplay_quests::quest::npc_rules::random_outcome::SANDRA_LOTTERY;

#[test]
fn sandra_lottery_missing_input_refuses_without_drawing() {
    let called = std::cell::Cell::new(false);
    assert_eq!(
        SANDRA_LOTTERY.effects(0, |_| {
            called.set(true);
            0
        }),
        None
    );
    assert!(!called.get());
}

#[test]
fn sandra_lottery_resolves_exact_input_and_selected_output() {
    let take = NpcRuleEffect::TakeItem {
        item: "oteryn:item.tibia.i5957".into(),
        count: 1,
        subtype: None,
        optional: false,
    };
    assert_eq!(
        SANDRA_LOTTERY.effects(1, |total| {
            assert_eq!(total, 98);
            0
        }),
        Some(vec![
            take.clone(),
            NpcRuleEffect::GiveItem {
                item: "oteryn:item.tibia.i5958".into(),
                count: 1,
                subtype: None,
            }
        ])
    );
    assert_eq!(SANDRA_LOTTERY.effects(1, |_| 97), Some(vec![take]));
    assert_eq!(SANDRA_LOTTERY.effects(1, |_| 98), None);
}

#[test]
fn sandra_lottery_odds_equal_crystal_201189f8() {
    assert_eq!(SANDRA_LOTTERY.total_weight(), Some(98));
    assert_eq!(SANDRA_LOTTERY.outcomes[0].weight, 1);
    assert_eq!(SANDRA_LOTTERY.outcomes[1].weight, 97);
    assert_eq!(SANDRA_LOTTERY.input, "oteryn:item.tibia.i5957");
    assert_eq!(
        SANDRA_LOTTERY.draw(0),
        Some(Some("oteryn:item.tibia.i5958"))
    );
    for draw in 1..98 {
        assert_eq!(SANDRA_LOTTERY.draw(draw), Some(None));
    }
    assert_eq!(SANDRA_LOTTERY.draw(98), None);
}

#[test]
fn sandra_lottery_server_rng_is_seeded_and_receipt_bound() {
    use crate::durability::quest_state::npc_program::sandra_lottery::draw;
    let binding = [7; 33];
    let draws: Vec<_> = (0..=255_u8).map(|seed| draw([seed; 32], binding)).collect();
    assert!(draws.contains(&Some(0)), "seeded win must be reachable");
    assert!(
        draws
            .iter()
            .any(|value| value.is_some_and(|value| value > 0))
    );
    for seed in 0..=255_u8 {
        assert_eq!(draw([seed; 32], binding), draws[usize::from(seed)]);
    }
}

#[test]
fn sandra_lottery_distinct_receipts_have_distinct_rng_streams() {
    use crate::durability::quest_state::npc_program::sandra_lottery::draw;
    let first: Vec<_> = (0..=255_u8).map(|seed| draw([seed; 32], [1; 33])).collect();
    let second: Vec<_> = (0..=255_u8).map(|seed| draw([seed; 32], [2; 33])).collect();
    assert_ne!(first, second);
    assert!(
        first
            .iter()
            .chain(&second)
            .all(|value| value.is_some_and(|value| value < 98))
    );
}

#[test]
fn sandra_lottery_dialogue_is_registered_for_production() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog;
    let source = NpcRuleCatalog::runtime()?;
    let rule = source
        .programs()
        .iter()
        .flat_map(|program| &program.document.rules)
        .find(|rule| rule.key == "oteryn:quest-npc/sandra/lottery")
        .ok_or("Sandra lottery dialogue missing")?;
    assert_eq!(rule.npc, "oteryn:npc.sandra");
    assert_eq!(rule.keywords, ["lottery"]);
    assert!(source.item_exchange_keys(&rule.npc).is_some());
    Ok(())
}

#[test]
fn sandra_lottery_claim_is_repeatable_per_ticket_and_stable_on_replay() {
    use crate::durability::quest_state::npc_program::sandra_lottery::claim_key;
    let first = claim_key([1; 16], 1);
    assert_eq!(first, claim_key([1; 16], 1));
    assert_ne!(first, claim_key([1; 16], 2));
    assert_ne!(first, claim_key([2; 16], 1));
}

#[test]
fn sandra_lottery_source_planner_requires_owned_ticket() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::from_json_documents(&[include_str!(
        "../../../../../../content/dialogues/quest-rules-supplemental/sandra-lottery.json"
    )])?;
    let tracks = BTreeMap::from([(
        "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt".into(),
        2,
    )]);
    let outfits = BTreeMap::new();
    for count in [0, 1] {
        let mut inventory = NpcInventorySnapshot::default();
        if count > 0 {
            inventory.items.insert(
                NpcItemKey {
                    item: SANDRA_LOTTERY.input.into(),
                    subtype: None,
                },
                count,
            );
        }
        let before = inventory.clone();
        let result = source.plan_item_exchange(&NpcRuleInput {
            npc: "oteryn:npc.sandra",
            trigger: NpcRuleTrigger::Say,
            message: "lottery",
            player_name: None,
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: false,
            level: 1,
            vocation: "oteryn:vocation.none",
            sex: 0,
            outfits: &outfits,
        })?;
        assert_eq!(result.is_some(), count == 1);
        if let Some(plan) = result {
            assert_eq!(plan.selected_rule_keys, ["oteryn:quest-npc/sandra/lottery"]);
        }
        assert_eq!(inventory, before, "planning must never mutate custody");
    }
    Ok(())
}

#[test]
fn sandra_lottery_postgres_missing_ticket_refuses_without_mutation() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("sandra-lottery-missing", || {
        super::quest_world_callbacks::scenario(
            "sandra-lottery-missing",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise_missing(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> super::TestResult {
    use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=1")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind("oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt")
        .bind("oteryn:quest.outfit_and_addon_quests").execute(&h.pool).await?;
    let catalogue = owner.quest_catalogue.clone().ok_or("quest catalogue")?;
    let source = oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime()?
        .admitted_for_npc("oteryn:npc.sandra", &catalogue)?
        .ok_or("Sandra source not admitted")?;
    assert!(
        source
            .programs()
            .iter()
            .flat_map(|program| &program.document.rules)
            .any(|rule| rule.key == "oteryn:quest-npc/sandra/lottery")
    );
    assert_eq!(SANDRA_LOTTERY.total_weight(), Some(98));
    assert_eq!(
        SANDRA_LOTTERY
            .outcomes
            .iter()
            .map(|outcome| outcome.weight)
            .collect::<Vec<_>>(),
        [1, 97]
    );

    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(super::fixture::debug)?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    let result = owner
        .root
        .commit_character_npc_program(
            authority,
            owner.holder,
            &owner.spell_lane_permit().await.ok_or("NPC permit")?,
            fence,
            NpcProgramRequest {
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(6275).map_err(super::fixture::debug)?,
                ),
                npc: "oteryn:npc.sandra".into(),
                trigger: NpcRuleTrigger::Say,
                message: "lottery".into(),
                topic: 0,
                transition_key: "oteryn:quest-npc/sandra/lottery".into(),
            },
            catalogue,
            &weights,
            &owner.premium_coordinator,
        )
        .await
        .map_err(super::fixture::debug)?;
    assert!(matches!(
        result.transition,
        QuestTransitionOutcome::Refused(_)
    ));
    assert!(!result.inventory_changed);
    assert_eq!(
        before,
        owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(super::fixture::debug)?
    );
    Ok(())
}

#[test]
fn sandra_lottery_selected_inventory_matches_win_and_blank() {
    use oteryn_gameplay_quests::quest::npc_rules::{NpcInventorySnapshot, NpcItemKey};
    let input = NpcItemKey {
        item: SANDRA_LOTTERY.input.into(),
        subtype: None,
    };
    let prize = NpcItemKey {
        item: "oteryn:item.tibia.i5958".into(),
        subtype: None,
    };
    let mut before = NpcInventorySnapshot::default();
    assert!(SANDRA_LOTTERY.inventory_after(&before, 0).is_none());
    before.items.insert(input.clone(), 2);
    before.items.insert(prize.clone(), 3);
    let mut expected = before.clone();
    expected.items.insert(input, 1);
    assert_eq!(
        SANDRA_LOTTERY.inventory_after(&before, 97),
        Some(expected.clone())
    );
    expected.items.insert(prize, 4);
    assert_eq!(SANDRA_LOTTERY.inventory_after(&before, 0), Some(expected));
    assert_eq!(
        before.items.get(&NpcItemKey {
            item: SANDRA_LOTTERY.input.into(),
            subtype: None
        }),
        Some(&2)
    );
}

#[test]
fn sandra_lottery_postgres_seeded_win_and_replay_grants_once() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("sandra-lottery-win", || {
        super::quest_world_callbacks::scenario(
            "sandra-lottery-win",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise_outcome(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    win: bool,
    full: bool,
) -> super::TestResult {
    use crate::durability::item_loop_move::ItemLoopContent;
    use crate::durability::quest_state::{
        QuestTransitionOutcome,
        npc_program::{NpcProgramRequest, sandra_lottery::draw_for_request},
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        npc_rules::NpcRuleTrigger,
        world_callbacks::{Offering, Position},
    };
    exercise_missing(owner, admitted, h, authority).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: SANDRA_LOTTERY.input.into(),
            count: 1,
        },
        6275,
        true,
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    if full {
        let mut tx = h.pool.begin().await?;
        let slot = crate::durability::item_transfer::load_slot(&mut tx, fence.character_id)
            .await?
            .ok_or("backpack")?;
        let entries =
            crate::durability::item_transfer::load_entries(&mut tx, slot.item_instance_id).await?;
        tx.rollback().await?;
        let capacity = weights
            .loop_facts(&slot.definition)
            .and_then(|facts| facts.container_capacity)
            .ok_or("capacity")?;
        let count = u32::try_from(entries.len())?;
        for index in count..capacity {
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    position: Position {
                        x: 33293,
                        y: 31677,
                        floor: -15,
                    },
                    // Separate, light non-stackable rings fill slots without
                    // making the fixture fail its carrying-weight check first.
                    item: "oteryn:item.tibia.i3052".into(),
                    count: 1,
                },
                7000 + u64::from(index),
                true,
            )
            .await?;
        }
        let mut tx = h.pool.begin().await?;
        let entries =
            crate::durability::item_transfer::load_entries(&mut tx, slot.item_instance_id).await?;
        tx.rollback().await?;
        assert_eq!(u32::try_from(entries.len())?, capacity);
    }
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut selected = None;
    for ordinal in 6276..16275 {
        let request = NpcProgramRequest {
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(ordinal).map_err(super::fixture::debug)?,
            ),
            npc: "oteryn:npc.sandra".into(),
            trigger: NpcRuleTrigger::Say,
            message: "lottery".into(),
            topic: 0,
            transition_key: "oteryn:quest-npc/sandra/lottery".into(),
        };
        if draw_for_request(weights.loop_content_digest(), &request, fence.character_id)
            .is_some_and(|draw| (draw == 0) == win)
        {
            selected = Some(request);
            break;
        }
    }
    let request = selected.ok_or("seeded winning occurrence")?;
    let result = owner
        .root
        .commit_character_npc_program(
            authority,
            owner.holder,
            &owner.spell_lane_permit().await.ok_or("NPC permit")?,
            fence,
            request.clone(),
            owner.quest_catalogue.clone().ok_or("catalogue")?,
            &weights,
            &owner.premium_coordinator,
        )
        .await
        .map_err(super::fixture::debug)?;
    if full {
        assert!(matches!(
            result.transition,
            QuestTransitionOutcome::Refused(
                oteryn_gameplay_quests::quest::QuestRefusal::CapacityExceeded
            )
        ));
        assert!(!result.inventory_changed);
        let remaining: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(SANDRA_LOTTERY.input).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 1);
        return Ok(());
    }
    assert!(matches!(
        result.transition,
        QuestTransitionOutcome::Committed(_)
    ));
    let input: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(SANDRA_LOTTERY.input).fetch_one(&h.pool).await?;
    let prize: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5958'")
        .fetch_one(&h.pool).await?;
    assert_eq!(input, 0);
    assert_eq!(prize, i64::from(win));
    let replay = owner
        .root
        .commit_character_npc_program(
            authority,
            owner.holder,
            &owner.spell_lane_permit().await.ok_or("NPC permit")?,
            fence,
            request,
            owner.quest_catalogue.clone().ok_or("catalogue")?,
            &weights,
            &owner.premium_coordinator,
        )
        .await
        .map_err(super::fixture::debug)?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let after: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5958'")
        .fetch_one(&h.pool).await?;
    assert_eq!(after, i64::from(win));
    Ok(())
}

#[test]
fn sandra_lottery_postgres_seeded_blank_consumes_ticket_and_replays_once() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("sandra-lottery-blank", || {
        super::quest_world_callbacks::scenario(
            "sandra-lottery-blank",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

#[test]
fn sandra_lottery_postgres_full_inventory_refuses_without_consuming_ticket() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("sandra-lottery-full", || {
        super::quest_world_callbacks::scenario(
            "sandra-lottery-full",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

#[test]
fn sandra_lottery_rejects_extra_or_optional_inventory_effects() -> super::TestResult {
    use crate::durability::quest_state::npc_program::sandra_lottery::valid_ticket_effects;
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
    let mut effects = SANDRA_LOTTERY.effects(1, |_| 0).ok_or("winning effects")?;
    assert!(valid_ticket_effects(&effects));
    effects.push(effects[0].clone());
    assert!(!valid_ticket_effects(&effects));
    effects.pop();
    if let NpcRuleEffect::TakeItem { optional, .. } = &mut effects[0] {
        *optional = true;
    }
    assert!(!valid_ticket_effects(&effects));
    assert!(!valid_ticket_effects(&[]));
    Ok(())
}

#[test]
fn sandra_lottery_refuses_unrelated_rewards_before_inventory_mutation() -> super::TestResult {
    use crate::durability::quest_state::npc_program::sandra_lottery::valid_ticket_effects;
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
    let base = SANDRA_LOTTERY.effects(1, |_| 0).ok_or("winning effects")?;
    for effect in [
        NpcRuleEffect::Experience { amount: 1 },
        NpcRuleEffect::GiveMoney {
            amount: 1,
            include_bank: false,
        },
        NpcRuleEffect::OutfitAddon {
            look_type: 138,
            addon: 1,
        },
    ] {
        let mut effects = base.clone();
        effects.push(effect);
        assert!(!valid_ticket_effects(&effects));
    }
    assert!(valid_ticket_effects(&base));
    Ok(())
}

#[test]
fn sandra_lottery_postgres_odds_table_equals_donor_values() -> super::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("sandra-lottery-odds", || {
        super::quest_world_callbacks::scenario(
            "sandra-lottery-odds",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

#[test]
fn sandra_lottery_ticket_issuance_matches_donor_same_kind_vials() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleCatalog, NpcRuleEffect, NpcRuleGuard};
    let source = NpcRuleCatalog::from_json_documents(&[include_str!(
        "../../../../../../content/dialogues/quest-rules-supplemental/sandra-lottery.json"
    )])?;
    let rules = &source.programs()[0].document.rules;
    for item in [283, 284, 285] {
        let key = format!("oteryn:item.tibia.i{item}");
        let rule = rules
            .iter()
            .find(|rule| rule.key == format!("oteryn:quest-npc/sandra/deposit-{item}"))
            .ok_or("donor vial exchange")?;
        assert!(rule.guards.iter().any(|guard| matches!(guard,
            NpcRuleGuard::Item { item, count: 100, .. } if item == &key)));
        assert!(rule.effects.iter().any(|effect| matches!(effect,
            NpcRuleEffect::TakeItem { item, count: 100, optional: false, .. } if item == &key)));
        assert!(rule.effects.iter().any(|effect| matches!(effect,
            NpcRuleEffect::GiveItem { item, count: 1, .. } if item == SANDRA_LOTTERY.input)));
    }
    assert!(source.item_exchange_keys("oteryn:npc.sandra").is_some());
    Ok(())
}

#[test]
fn sandra_lottery_vial_deposits_use_repeatable_command_claims() {
    use crate::durability::quest_state::npc_program::sandra_lottery::repeatable_exchange;
    for suffix in ["lottery", "deposit-283", "deposit-284", "deposit-285"] {
        assert!(repeatable_exchange(&format!(
            "oteryn:quest-npc/sandra/{suffix}"
        )));
    }
    for suffix in ["bonus-join", "prize", "deposit-286", "deposit-283-extra"] {
        assert!(!repeatable_exchange(&format!(
            "oteryn:quest-npc/sandra/{suffix}"
        )));
    }
    assert!(!repeatable_exchange("oteryn:quest-npc/other/lottery"));
}

#[test]
fn sandra_lottery_bonus_topics_match_donor_enrollment_flow() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::from_json_documents(&[include_str!(
        "../../../../../../content/dialogues/quest-rules-supplemental/sandra-lottery.json"
    )])?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let track = "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt";
    for (value, message, topic, expected) in [
        (0, "ticket", 0, 1),
        (0, "yes", 1, 2),
        (0, "yes", 2, 0),
        (1, "deposit", 0, 3),
        (2, "bonus", 0, 3),
    ] {
        let tracks = BTreeMap::from([(track.into(), value)]);
        let plan = source
            .plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.sandra",
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })?
            .ok_or("bonus dialogue")?;
        assert_eq!(plan.next_topic, expected);
        if topic == 2 {
            assert!(plan.track_changes.iter().any(|change| change.key
                == "oteryn:quest-progress/outfit_quest/default_start"
                && change.after == 1));
            assert!(
                plan.track_changes
                    .iter()
                    .any(|change| change.key == track && change.after == 1)
            );
        }
    }
    Ok(())
}

#[test]
fn sandra_lottery_donor_entry_keywords_match_entire_message() -> super::TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::from_json_documents(&[include_str!(
        "../../../../../../content/dialogues/quest-rules-supplemental/sandra-lottery.json"
    )])?;
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    for value in [0, 1] {
        let tracks = BTreeMap::from([(
            "oteryn:quest-progress/quest/u7_8/mage_and_summoner_outfits/addon_belt".into(),
            value,
        )]);
        for message in ["a vial", "ticket please", "bonus system", "deposit vials"] {
            let plan = source.plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.sandra",
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic: 0,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })?;
            assert!(plan.is_none(), "donor requires an exact entry keyword");
        }
    }
    Ok(())
}
