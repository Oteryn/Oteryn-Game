#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::durability::DurabilityError;
use crate::durability::highscore_snapshot::reply::highscores_reply;
use crate::durability::highscore_snapshot::{
    SnapshotError, SnapshotInfo, SnapshotPage, SnapshotRow,
};
use crate::foundation::{CharacterId, FoundationProtocolError};
use oteryn_protocol_oteryn::highscores::{MAX_HIGHSCORES_PAGE_ROWS, decode_highscores_result};

fn character(index: u32) -> CharacterId {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&index.to_be_bytes());
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    CharacterId::decode(&bytes).expect("v7 character id")
}

fn row(rank: u64, index: u32, name: &str, value: (u64, u64)) -> SnapshotRow {
    SnapshotRow {
        rank,
        character_id: character(index),
        name: name.to_owned(),
        vocation: "knight".to_owned(),
        level: 8,
        value,
    }
}

fn highscores_reply_for(page: Result<SnapshotPage, SnapshotError>) -> HighscoresReply {
    highscores_reply(page, character(2))
}

fn page(rows: Vec<SnapshotRow>, own_row: Option<SnapshotRow>) -> SnapshotPage {
    SnapshotPage {
        snapshot: SnapshotInfo {
            snapshot_id: 300,
            category: 5,
            computed_at_ms: 1_700_000_000_000,
            source_cut: "cut".to_owned(),
        },
        page: 1,
        has_more: false,
        rows,
        own_row,
    }
}

#[test]
fn a_page_encodes_its_rows_in_order_with_the_primary_value_only() {
    let own = row(2, 2, "Bea", (61, 90));
    let reply = highscores_reply_for(Ok(page(
        vec![
            row(1, 1, "Ada", (61, 95)),
            own.clone(),
            row(2, 3, "Cyd", (61, 90)),
        ],
        Some(own),
    )));
    let HighscoresReply::Result(payload) = reply else {
        panic!("expected a result, got {reply:?}");
    };
    let decoded = decode_highscores_result(&payload).expect("decodes");
    assert_eq!(decoded.category, 5);
    assert_eq!(decoded.snapshot_id, 300);
    assert_eq!(decoded.computed_at, 1_700_000_000_000);
    assert_eq!((decoded.page, decoded.has_more), (1, false));
    let ranks: Vec<(u64, &str, u64)> = decoded
        .rows
        .iter()
        .map(|row| (row.rank, row.name.as_str(), row.value))
        .collect();
    assert_eq!(ranks, [(1, "Ada", 61), (2, "Bea", 61), (2, "Cyd", 61)]);
    assert_eq!(decoded.own_row.map(|row| row.name), Some("Bea".to_owned()));
}

#[test]
fn the_snapshot_outcomes_fail_closed_with_their_registered_codes() {
    assert_eq!(
        highscores_reply_for(Err(SnapshotError::NotSupported)),
        HighscoresReply::Terminal(FoundationProtocolError::NotSupported)
    );
    assert_eq!(
        highscores_reply_for(Err(SnapshotError::SnapshotExpired)),
        HighscoresReply::Terminal(FoundationProtocolError::SnapshotExpired)
    );
    for error in [
        SnapshotError::InvalidQuery,
        SnapshotError::RetryableConflict,
        SnapshotError::CapacityExceeded,
        SnapshotError::Unavailable(DurabilityError::InvalidStoredState),
    ] {
        assert_eq!(highscores_reply_for(Err(error)), HighscoresReply::Rejected);
    }
}

#[test]
fn a_stored_row_outside_the_wire_grammar_or_bounds_is_never_truncated() {
    // A malformed stored name is a server fault: REJECTED, no payload, no code.
    assert_eq!(
        highscores_reply_for(Ok(page(vec![row(1, 1, "A1", (1, 0))], None))),
        HighscoresReply::Rejected
    );
    // A page over the 50-row bound fails closed with PAYLOAD_LIMIT_EXCEEDED.
    let rows = (1..=u32::try_from(MAX_HIGHSCORES_PAGE_ROWS + 1).expect("bound"))
        .map(|index| row(u64::from(index), index, "Ada", (1, 0)))
        .collect();
    assert_eq!(
        highscores_reply_for(Ok(page(rows, None))),
        HighscoresReply::Terminal(FoundationProtocolError::PayloadLimitExceeded)
    );
}

#[test]
fn an_own_row_of_another_character_is_a_store_fault() {
    assert_eq!(
        highscores_reply(
            Ok(page(Vec::new(), Some(row(1, 7, "Ada", (1, 0))))),
            character(2)
        ),
        HighscoresReply::Rejected
    );
}

#[test]
fn the_achievement_catalogue_owns_category_10_points() {
    use crate::durability::highscore_snapshot::AchievementPoints;
    let catalogue =
        crate::achievement_catalogue::AchievementCatalogue::embedded().expect("embedded catalogue");
    assert_eq!(catalogue.total_points(&[]), Some(0));
}
