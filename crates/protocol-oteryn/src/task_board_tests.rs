#![allow(clippy::expect_used)]
use super::*;

fn board(selected: u8, kills: i64) -> TaskBoardReply {
    TaskBoardReply {
        refused: selected == 2,
        selected,
        kills,
        points: 75,
        tasks: (0..MAX_TASKS)
            .map(|index| TaskBoardTask {
                race: format!("oteryn:creature.race_{index}"),
                required: 30,
                claimed_at_unix: if index == 0 { 1_800_000_000 } else { 0 },
            })
            .collect(),
    }
}

#[test]
fn task_board_request_strict_codec() {
    let valid = [(0, 0), (1, 1), (1, 5), (2, 3)];
    for (action, task) in valid {
        let r = TaskBoardRequest { action, task };
        let bytes = encode_task_board_request(&r).expect("request");
        assert_eq!(decode_task_board_request(&bytes), Ok(r));
        for end in 0..bytes.len() {
            assert!(decode_task_board_request(&bytes[..end]).is_err());
        }
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&[8, action]);
        assert!(decode_task_board_request(&duplicate).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[24, 1]);
        assert!(decode_task_board_request(&unknown).is_err());
    }
    for (action, task) in [(0, 1), (1, 0), (2, 6), (3, 1)] {
        let r = TaskBoardRequest { action, task };
        assert!(encode_task_board_request(&r).is_err(), "{r:?}");
        assert!(decode_task_board_request(&[8, action, 16, task]).is_err());
    }
    assert_eq!(
        decode_task_board_request(&[0; MAX_REQUEST_BYTES + 1]),
        Err(Error::LimitExceeded)
    );
    // A task number over u8 never truncates into a valid one.
    assert!(decode_task_board_request(&[8, 1, 16, 0x81, 0x02]).is_err());
}

#[test]
fn task_board_reply_strict_codec() {
    for (selected, kills) in [(0, 0), (1, 30), (2, 7)] {
        let r = board(selected, kills);
        let bytes = encode_task_board_reply(&r).expect("reply");
        assert!(bytes.len() <= MAX_REPLY_BYTES);
        assert_eq!(decode_task_board_reply(&bytes), Ok(r.clone()));
        // A prefix that ends on a task boundary is a shorter board; any other prefix is refused.
        for end in 0..bytes.len() {
            if let Ok(shorter) = decode_task_board_reply(&bytes[..end]) {
                assert!(shorter.tasks.len() < r.tasks.len());
            }
        }
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&[16, 0]);
        assert!(decode_task_board_reply(&duplicate).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[48, 1]);
        assert!(decode_task_board_reply(&unknown).is_err());
    }
    for invalid in [
        board(0, 1),
        board(1, 31),
        board(6, 0),
        TaskBoardReply {
            points: -1,
            ..board(0, 0)
        },
    ] {
        assert!(encode_task_board_reply(&invalid).is_err(), "{invalid:?}");
    }
    let mut bad_race = board(0, 0);
    bad_race.tasks[0].race = "creature.rat".into();
    assert!(encode_task_board_reply(&bad_race).is_err());
    let mut oversized_race = board(0, 0);
    oversized_race.tasks[0].race = format!("oteryn:{}", "a".repeat(MAX_RACE_BYTES));
    assert!(encode_task_board_reply(&oversized_race).is_err());
    let mut too_many = board(0, 0);
    too_many.tasks.push(too_many.tasks[0].clone());
    assert_eq!(
        encode_task_board_reply(&too_many),
        Err(Error::LimitExceeded)
    );
    // The largest valid board stays inside the reply bound.
    let mut largest = board(1, 30);
    for task in &mut largest.tasks {
        task.race = format!("oteryn:{}", "a".repeat(MAX_RACE_BYTES - "oteryn:".len()));
        task.required = i64::MAX;
        task.claimed_at_unix = i64::MAX;
    }
    largest.kills = i64::MAX;
    largest.points = i64::MAX;
    let bytes = encode_task_board_reply(&largest).expect("largest reply");
    assert_eq!(decode_task_board_reply(&bytes), Ok(largest));
    assert_eq!(
        decode_task_board_reply(&[0; MAX_REPLY_BYTES + 1]),
        Err(Error::LimitExceeded)
    );
    // A refused flag other than 0 or 1 is no boolean.
    assert!(decode_task_board_reply(&[8, 2, 16, 0, 24, 0, 32, 0]).is_err());
    // Out-of-order fields are not the canonical encoding.
    assert!(decode_task_board_reply(&[16, 0, 8, 0, 24, 0, 32, 0]).is_err());
}
