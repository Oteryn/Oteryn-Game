#![allow(clippy::expect_used)]
use super::*;
use crate::durability::task_board::{TaskBoardRefusal, TaskBoardView};
use oteryn_gameplay_quests::quest::task_board::full as rules;
use std::collections::BTreeMap;

fn committed() -> TaskBoardCommit {
    TaskBoardCommit {
        view: TaskBoardView {
            data: rules::TaskBoardData::default(),
            balances: rules::TaskBoardBalances::default(),
            tracks: BTreeMap::new(),
            level: 300,
            period: 20_754,
            database_now: 1_800_000_000,
            reroll_tokens: 0,
            daily_reroll_at: 0,
            delivery_definition_refs: BTreeMap::new(),
            delivery_available: BTreeMap::new(),
        },
        refusal: None,
        transition: None,
        replayed: false,
    }
}

#[test]
fn the_board_reply_projects_every_task_and_the_stored_tracks() {
    // P04 replaces the five static TASKBOARD-1 rows with every generated weekly row.
    let mut commit = committed();
    commit.view.data.expansion = true;
    commit.view.data.weekly = Some(rules::WeeklyBoard {
        period: 20_754,
        difficulty: 3,
        any_required: 4_000,
        any_kills: 4_000,
        kills: (0..8)
            .map(|i| rules::WeeklyKillTask {
                race: format!("oteryn:creature.test{i}"),
                required: 500,
                current: 500,
            })
            .collect(),
        deliveries: (0..9)
            .map(|i| rules::WeeklyDeliveryTask {
                item_id: 3031 + i,
                required: 100,
                delivered: i < 8,
            })
            .collect(),
        finished: false,
    });
    commit.view.balances = rules::TaskBoardBalances {
        points: 100,
        bounty_points: 50,
        soulseals: 9,
        talisman_levels: [1, 15, 55, 355],
    };
    commit.view.reroll_tokens = 256;
    // Crystal protocolgame.cpp:6210–6222 shows owned units, even above required.
    commit.view.delivery_available.insert(3039, 175);
    let reply = projection::full(&commit, false, None).expect("complete projection");
    assert_eq!(
        (reply.weekly.kills.len(), reply.weekly.deliveries.len()),
        (9, 9)
    );
    assert_eq!(
        (reply.points, reply.bounty_points, reply.soulseals),
        (100, 50, 9)
    );
    assert_eq!(reply.weekly.multiplier, 8);
    assert_eq!(reply.bounty.rerolls, 256);
    assert_eq!(reply.weekly.deliveries[8].current, 175);
    assert_eq!(reply.weekly.deliveries[0].current, 100);
    assert_eq!(
        (reply.talisman[3].level, reply.talisman[3].next_cost),
        (355, 0)
    );
    let bytes = wire::full::encode_reply(&reply).expect("all generated rows fit");
    assert!(bytes.len() <= wire::full::MAX_REPLY_BYTES);
    assert_eq!(wire::full::decode_reply(&bytes), Ok(reply));
    let legacy = projection::legacy(&commit).expect("legacy balances projection");
    assert!(legacy.tasks.is_empty());
    assert_eq!((legacy.selected, legacy.kills, legacy.points), (0, 0, 100));
    let bytes = wire::encode_task_board_reply(&legacy).expect("legacy balance encoding");
    assert_eq!(wire::decode_task_board_reply(&bytes), Ok(legacy));
}

#[test]
fn board_wire_rejects_an_overflowed_durable_completion_snapshot() {
    let mut commit = committed();
    commit.view.data.weekly = Some(rules::WeeklyBoard {
        period: 20_754,
        difficulty: 0,
        any_required: 1,
        any_kills: 1,
        kills: (0..255)
            .map(|index| rules::WeeklyKillTask {
                race: format!("oteryn:creature.overflow{index}"),
                required: 1,
                current: 1,
            })
            .collect(),
        deliveries: vec![rules::WeeklyDeliveryTask {
            item_id: 3031,
            required: 1,
            delivered: true,
        }],
        finished: false,
    });

    assert_eq!(
        commit
            .view
            .data
            .weekly
            .as_ref()
            .expect("weekly fixture")
            .completion_summary(),
        Err(rules::TaskBoardRefusal::ArithmeticOverflow)
    );
    assert_eq!(projection::full(&commit, false, None), None);
}

#[test]
fn full_actions_preserve_legacy_reply_and_retire_fixed_task_mutations() {
    let query = wire::full::encode_request(&wire::full::Request::query()).expect("full query");
    assert_eq!(decode_action(&query), Some((TaskBoardAction::Query, false)));
    for action in 0..=3 {
        let task = u8::from(action != 0);
        let payload = wire::encode_task_board_request(&wire::TaskBoardRequest { action, task })
            .expect("legacy request");
        let expected = match action {
            0 => TaskBoardAction::Query,
            1 | 2 => TaskBoardAction::RefuseLegacy { action, task },
            _ => TaskBoardAction::UpgradeTalisman { path: 0 },
        };
        assert_eq!(decode_action(&payload), Some((expected, true)));
    }
    let mut invalid = query;
    invalid.push(0);
    assert!(decode_action(&invalid).is_none());
}

#[test]
fn unavailable_content_cannot_offer_or_reenable_the_bundle_only_capability() {
    let base = [OfferedCapability {
        id: 72,
        requires: &[],
    }];
    assert_eq!(offered(&base, true).len(), 1);
    assert!(offered(&base, false).is_empty());
    assert!(offered(&[], true).is_empty());
}

#[test]
fn refused_and_replayed_commands_project_their_immutable_receipt() {
    let mut commit = committed();
    commit.refusal = Some(TaskBoardRefusal::Rule(rules::TaskBoardRefusal::Cooldown));
    let first = projection::full(&commit, false, None).expect("refusal projection");
    assert_eq!(first.refusal, 5);
    commit.replayed = true;
    assert_eq!(projection::full(&commit, false, None), Some(first));
    assert!(projection::legacy(&commit).expect("legacy refusal").refused);
}

#[test]
fn finished_weekly_results_require_negotiation_and_do_not_change_balances() {
    let mut commit = committed();
    commit.view.data.weekly = Some(rules::WeeklyBoard {
        period: commit.view.period - 7,
        difficulty: 0,
        any_required: 500,
        any_kills: 500,
        kills: Vec::new(),
        deliveries: Vec::new(),
        finished: true,
    });
    let original = projection::full(&commit, false, None).expect("original client");
    assert!(original.weekly_results.is_none());
    let current = projection::full(&commit, true, None).expect("negotiated client");
    assert_eq!(
        current.weekly_results,
        Some(wire::full::WeeklyResults {
            period: u64::try_from(commit.view.period - 7).expect("period"),
            points: 25,
            soulseals: 1,
        })
    );
    assert_eq!(
        (current.points, current.soulseals),
        (original.points, original.soulseals)
    );
    assert!(!current.weekly.started);
}

#[test]
fn weekly_master_candidates_match_the_global_source_set() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeSet;

    let world = oteryn_protocol_oteryn::WorldId::decode(&[42; 16])
        .map_err(|error| format!("{error:?}"))?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let generation = controller.active().ok_or("activated generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let catalogue = crate::durability::task_board::TaskBoardCatalogue::try_from(native)?;
    let expected: BTreeSet<_> = include_str!(
        "../../../../../apps/game-server/src/qualification/client_phases/p04_master_creature_oracle.txt"
    )
    .lines()
    .filter(|line| !line.starts_with('#'))
    .map(|line| match line {
        "oteryn:creature.sabretooth_creature" => "oteryn:creature.sabretooth",
        key => key,
    })
    .collect();
    let context = rules::TaskBoardContext {
        level: 400,
        has_vocation: true,
        now_unix_seconds: 1_800_000_000,
        period: 42,
        reset_day: Some(42),
        creatures: &catalogue.creatures,
        available_gold: 0,
        reroll_tokens: 0,
    };
    let mut observed = BTreeSet::new();
    let mut index = 0;
    loop {
        let mut data = rules::TaskBoardData::default();
        let mut first_draw_width = None;
        rules::apply_action(
            &mut data,
            &mut rules::TaskBoardBalances::default(),
            &rules::TaskBoardAction::StartWeekly { difficulty: 3 },
            &context,
            &mut |width| {
                if first_draw_width.is_some() {
                    return 0;
                }
                first_draw_width = Some(width);
                index
            },
        )
        .map_err(|error| format!("Master draw {index}: {error:?}"))?;
        let board = data.weekly.ok_or("started Master board")?;
        observed.insert(board.kills.into_iter().next().ok_or("first kill task")?.race);
        index += 1;
        if Some(index) == first_draw_width {
            break;
        }
    }
    let actual: BTreeSet<_> = observed.iter().map(String::as_str).collect();
    assert!(actual.contains("oteryn:creature.cliff_strider"), "Cliff Strider");
    assert!(!actual.contains("oteryn:creature.moonspawn_juggernaut"), "Moonspawn Juggernaut");
    assert_eq!(actual, expected);
    Ok(())
}
