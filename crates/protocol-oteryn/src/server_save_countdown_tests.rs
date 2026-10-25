#![allow(clippy::expect_used)]

use super::*;

#[test]
fn server_save_countdown_golden_vectors_and_strict_bounds() -> WireResult<()> {
    assert_eq!(SERVER_SAVE_COUNTDOWN_SNAPSHOT, [8, 1]);
    for (notice, bytes) in [
        (
            ServerSaveCountdown {
                ending_epoch: 0,
                minutes_remaining: 5,
            },
            vec![8, 0, 16, 5],
        ),
        (
            ServerSaveCountdown {
                ending_epoch: 300,
                minutes_remaining: 1,
            },
            vec![8, 172, 2, 16, 1],
        ),
    ] {
        assert_eq!(encode_server_save_countdown(notice)?, bytes);
        assert_eq!(decode_server_save_countdown(&bytes)?, notice);
    }
    let largest = ServerSaveCountdown {
        ending_epoch: u64::MAX,
        minutes_remaining: 5,
    };
    let bytes = encode_server_save_countdown(largest)?;
    assert_eq!(bytes.len(), 13);
    assert_eq!(decode_server_save_countdown(&bytes)?, largest);
    for bytes in [
        vec![],
        vec![8, 1],
        vec![16, 1, 8, 1],
        vec![8, 1, 16, 0],
        vec![8, 1, 16, 6],
        vec![8, 1, 16, 128, 2],
        vec![8, 1, 16, 1, 16, 1],
        vec![8, 1, 24, 1],
        vec![0; 17],
    ] {
        assert!(decode_server_save_countdown(&bytes).is_err());
    }
    for minutes_remaining in [0, 6, u8::MAX] {
        assert!(
            encode_server_save_countdown(ServerSaveCountdown {
                ending_epoch: 1,
                minutes_remaining,
            })
            .is_err()
        );
    }
    Ok(())
}

// TEST-FUZZ-1: bounded randomized decoder and generated-value round trips.

#[test]
fn fuzz_decode_server_save_countdown() {
    crate::decoder_fuzz_support::codec(
        "decode_server_save_countdown",
        |rng| ServerSaveCountdown {
            ending_epoch: rng.u64(),
            minutes_remaining: (rng.below(5) + 1) as u8,
        },
        decode_server_save_countdown,
        |v| encode_server_save_countdown(*v).expect("countdown"),
    );
}
