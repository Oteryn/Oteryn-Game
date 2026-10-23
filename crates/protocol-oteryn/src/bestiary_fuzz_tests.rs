#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_bestiary_view() {
    crate::decoder_fuzz_support::codec(
        "decode_bestiary_view",
        |rng| {
            (1..=rng.below(4) as u32 + 1)
                .map(|i| BestiaryRaceProgress {
                    race: NonZeroU32::new(i).expect("race"),
                    kill_count: rng.below(1000) as u32 + 1,
                    kill_thresholds: [1, 100, 1000],
                })
                .collect::<Vec<_>>()
        },
        decode_bestiary_view,
        |v| encode_bestiary_view(v).expect("view"),
    );
}
