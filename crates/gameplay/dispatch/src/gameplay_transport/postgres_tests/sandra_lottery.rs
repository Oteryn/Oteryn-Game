//! Transport-composed PostgreSQL scenarios for Sandra's durable exchange.
use oteryn_gameplay_quests::quest::npc_rules::random_outcome::SANDRA_LOTTERY;

#[path = "../../../../../../apps/game-server/src/lifted_tests/auto/sandra_lottery_rules.rs"]
mod rules;

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
