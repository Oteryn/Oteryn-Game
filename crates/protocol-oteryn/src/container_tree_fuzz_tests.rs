#![allow(clippy::expect_used)]

use super::*;

// TEST-FUZZ-1: bounded randomized decoder and generated-value round trips.

#[test]
fn fuzz_decode_container_views() {
    crate::decoder_fuzz_support::codec(
        "decode_container_views",
        |rng| {
            views(
                (0..rng.below(4) as u8)
                    .map(|id| small_view(id, rng.below(21)))
                    .collect(),
            )
        },
        decode_container_views,
        |v| encode_container_views(v).expect("views"),
    );
}

#[test]
fn fuzz_decode_container_view_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_container_view_intent",
        |rng| match rng.below(3) {
            0 => ContainerViewIntent::Open {
                handle: handle(rng.u64().max(1)),
                replace_view: rng.bool().then(|| rng.below(16) as u8),
            },
            1 => ContainerViewIntent::Close {
                view_id: rng.below(16) as u8,
            },
            _ => ContainerViewIntent::Up {
                view_id: rng.below(16) as u8,
            },
        },
        decode_container_view_intent,
        |v| encode_container_view_intent(v).expect("intent"),
    );
}

#[test]
fn fuzz_decode_container_view_result() {
    crate::decoder_fuzz_support::codec(
        "decode_container_view_result",
        |rng| {
            *rng.pick(&[
                ContainerViewOutcome::Opened,
                ContainerViewOutcome::Closed,
                ContainerViewOutcome::Stale,
                ContainerViewOutcome::TooFar,
                ContainerViewOutcome::NotAContainer,
                ContainerViewOutcome::TooManyViews,
            ])
        },
        decode_container_view_result,
        |v| encode_container_view_result(*v),
    );
}
