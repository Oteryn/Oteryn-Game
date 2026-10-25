#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_account_achievements_query() {
    crate::decoder_fuzz_support::codec(
        "decode_account_achievements_query",
        |rng| AccountAchievementsQuery {
            page: rng.u64() as u32,
        },
        decode_account_achievements_query,
        |v| encode_account_achievements_query(*v),
    );
}

#[test]
fn fuzz_decode_account_achievements_result() {
    crate::decoder_fuzz_support::codec(
        "decode_account_achievements_result",
        |rng| AccountAchievementsResult {
            total_points: rng.u64() as u32,
            fact_count: rng.u64() as u32,
            page: rng.u64() as u32,
            rows: vec![AccountAchievementRow {
                key: "oteryn:achievement/allow_cookies".into(),
                name: "Allow Cookies?!".into(),
                description: "é".repeat(rng.below(100) as usize),
                grade: (rng.below(3) + 1) as u32,
                points: rng.u64() as u32,
                earned_at: rng.u64() as i64,
                secret: rng.bool(),
            }],
            has_more: false,
        },
        decode_account_achievements_result,
        |v| encode_account_achievements_result(v).expect("result"),
    );
}
