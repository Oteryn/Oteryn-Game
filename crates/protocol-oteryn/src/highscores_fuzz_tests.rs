#![allow(clippy::expect_used)]

use super::*;

// TEST-FUZZ-1: bounded randomized decoder and generated-value round trips.

#[test]
fn fuzz_decode_highscores_query() {
    crate::decoder_fuzz_support::codec(
        "decode_highscores_query",
        |rng| HighscoresQuery {
            category: 1,
            vocation: 0,
            page: 1,
            snapshot_id: rng.u64(),
        },
        decode_highscores_query,
        |v| encode_highscores_query(*v).expect("query"),
    );
}

#[test]
fn fuzz_decode_highscores_result() {
    crate::decoder_fuzz_support::codec(
        "decode_highscores_result",
        |rng| HighscoresResult {
            snapshot_id: rng.u64().max(1),
            rows: vec![HighscoreRow {
                value: rng.u64(),
                ..row()
            }],
            ..page()
        },
        decode_highscores_result,
        |v| encode_highscores_result(v).expect("page"),
    );
}
