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
        1,
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
