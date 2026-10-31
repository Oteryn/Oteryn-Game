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
    let reply = projection::full(&commit).expect("complete projection");
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
    let first = projection::full(&commit).expect("refusal projection");
    assert_eq!(first.refusal, 5);
    commit.replayed = true;
    assert_eq!(projection::full(&commit), Some(first));
    assert!(projection::legacy(&commit).expect("legacy refusal").refused);
}
