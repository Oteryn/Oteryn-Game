#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_presentation_snapshot() {
    crate::decoder_fuzz_support::codec(
        "decode_presentation_snapshot",
        |_| (),
        decode_presentation_snapshot,
        |_| encode_presentation_snapshot(),
    );
}

#[test]
fn fuzz_decode_presentation_batch() {
    crate::decoder_fuzz_support::codec(
        "decode_presentation_batch",
        |rng| {
            let mut events = every_kind();
            events.truncate((rng.below(events.len() as u64) + 1) as usize);
            events
        },
        decode_presentation_batch,
        |v| encode_presentation_batch(v).expect("events"),
    );
}

#[test]
fn fuzz_decode_actor_cooldowns() {
    crate::decoder_fuzz_support::codec(
        "decode_actor_cooldowns",
        |rng| ActorCooldowns {
            server_now_ms: rng.below(MAX_CLOCK_MS + 1),
            rtt_ms: rng.bool().then(|| rng.u64() as u32),
            entries: (1..=rng.below(8) as u32)
                .map(|id| cooldown(ActorCooldownKind::Spell, id, rng.below(MAX_CLOCK_MS + 1)))
                .collect(),
        },
        // The encoder sorts entries by (kind, id), so compare decoded values in that order.
        |bytes| {
            decode_actor_cooldowns(bytes).map(|mut cooldowns| {
                cooldowns
                    .entries
                    .sort_unstable_by_key(|entry| (entry.kind, entry.id));
                cooldowns
            })
        },
        |v| encode_actor_cooldowns(v).expect("cooldowns"),
    );
}
