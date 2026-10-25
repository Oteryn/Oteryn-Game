#![allow(clippy::expect_used)]
use super::*;
use std::collections::BTreeMap;

#[test]
fn the_board_reply_projects_every_task_and_the_stored_tracks() {
    let empty = board_reply(false, &BTreeMap::new());
    assert_eq!((empty.selected, empty.kills, empty.points), (0, 0, 0));
    assert_eq!(empty.tasks.len(), task_board::TASKS.len());
    for (task, wire_task) in task_board::TASKS.iter().zip(&empty.tasks) {
        assert_eq!(wire_task.race, task.race);
        assert_eq!(wire_task.required, task.required);
        assert_eq!(wire_task.claimed_at_unix, 0);
    }
    wire::encode_task_board_reply(&empty).expect("an empty board encodes");
    let tracks = BTreeMap::from([
        (task_board::SELECTED.to_owned(), 3),
        (task_board::KILLS.to_owned(), 7),
        (task_board::POINTS.to_owned(), 50),
        (task_board::claimed_at_track(2), 1_800_000_000),
    ]);
    let reply = board_reply(true, &tracks);
    assert!(reply.refused);
    assert_eq!((reply.selected, reply.kills, reply.points), (3, 7, 50));
    assert_eq!(reply.tasks[1].claimed_at_unix, 1_800_000_000);
    let bytes = wire::encode_task_board_reply(&reply).expect("a stored board encodes");
    assert!(bytes.len() <= wire::MAX_REPLY_BYTES);
    assert_eq!(wire::decode_task_board_reply(&bytes), Ok(reply));
}
