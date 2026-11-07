#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_batch() {
    crate::decoder_fuzz_support::codec(
        "decode_batch",
        |rng| Batch {
            content: [1; 32],
            sync_unit: rng.u64().max(1),
            events: (1..=rng.below(5) as u16)
                .map(|id| Event {
                    decision_ordinal: u64::from(id),
                    emission_ordinal: 1,
                    body: Body::Effect {
                        position: ActorPosition {
                            x: rng.u64() as i32,
                            y: rng.u64() as i32,
                            floor: rng.below(16) as i16,
                        },
                        id: (rng.below(1000) + 1) as u16,
                        source: Source::Own,
                    },
                })
                .collect(),
        },
        decode_batch,
        |v| encode_batch(v).expect("batch"),
    );
}

#[test]
fn fuzz_decode_empty_snapshot() {
    crate::decoder_fuzz_support::codec(
        "decode_empty_snapshot",
        |_| (),
        decode_empty_snapshot,
        |_| Vec::new(),
    );
}

#[test]
fn fuzz_decode_cooldowns() {
    crate::decoder_fuzz_support::codec(
        "decode_cooldowns",
        |rng| Cooldowns {
            server_now_ms: rng.below(MAX_CLOCK_MS + 1),
            rtt_ms: rng.u64() as u32,
            rtt_estimated: true,
            entries: (1..=rng.below(8) as u32)
                .map(|id| Cooldown {
                    kind: CooldownKind::Spell,
                    id,
                    expires_at_ms: rng.below(MAX_CLOCK_MS + 1),
                })
                .collect(),
        },
        decode_cooldowns,
        |v| encode_cooldowns(v).expect("cooldowns"),
    );
}

#[test]
fn fuzz_binary_count_prefixes_are_bounded() {
    let mut batch = encode_batch(&Batch {
        content: [1; 32],
        sync_unit: 1,
        events: vec![],
    })
    .expect("batch");
    batch[41..43].copy_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(decode_batch(&batch), Err(Error::Limit));
    let mut cooldowns = encode_cooldowns(&Cooldowns {
        server_now_ms: 0,
        rtt_ms: 0,
        rtt_estimated: false,
        entries: vec![],
    })
    .expect("cooldowns");
    cooldowns[12..14].copy_from_slice(&u16::MAX.to_le_bytes());
    assert_eq!(decode_cooldowns(&cooldowns), Err(Error::Limit));
}
