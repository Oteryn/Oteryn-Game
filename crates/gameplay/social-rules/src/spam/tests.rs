#![allow(clippy::expect_used)]

use super::*;

fn at_ms(ms: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(ms * 1_000)
}

const LEVEL_20: YellGate = YellGate {
    level: 20,
    premium_current: false,
};

#[test]
fn bucket_admits_four_lines_then_the_fifth_mutes_for_5_seconds() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    for _ in 0..BUCKET_LINES {
        assert_eq!(limiter.admit_local(false, LEVEL_20, at_ms(0)), Ok(()));
    }
    assert!(!limiter.is_muted(at_ms(0)));
    assert_eq!(
        limiter.admit_local(false, LEVEL_20, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(4_999)));
    assert_eq!(
        limiter.admit_local(false, LEVEL_20, at_ms(1_500)),
        Err(ChatRefusal::Muted { seconds: 4 })
    );
    assert!(!limiter.is_muted(at_ms(5_000)));
    // The bucket is full after the mute.
    for _ in 0..BUCKET_LINES {
        assert_eq!(limiter.admit_local(false, LEVEL_20, at_ms(5_000)), Ok(()));
    }
}

#[test]
fn bucket_refills_one_line_per_2_5_seconds() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    for _ in 0..BUCKET_LINES {
        limiter
            .admit_local(false, LEVEL_20, at_ms(0))
            .expect("line");
    }
    // One line back at 2.5 s, none more before 5 s.
    assert_eq!(limiter.admit_local(false, LEVEL_20, at_ms(2_500)), Ok(()));
    assert!(matches!(
        limiter.admit_local(false, LEVEL_20, at_ms(4_999)),
        Err(ChatRefusal::Muted { .. })
    ));

    let mut steady = ChatLimiter::new(at_ms(0));
    // One line every 2.5 s never runs dry.
    for i in 0..100 {
        assert_eq!(
            steady.admit_local(false, LEVEL_20, at_ms(i * 2_500)),
            Ok(())
        );
    }
}

fn offend(limiter: &mut ChatLimiter, now_ms: u64) -> ChatRefusal {
    for _ in 0..BUCKET_LINES {
        limiter
            .admit_local(false, LEVEL_20, at_ms(now_ms))
            .expect("line");
    }
    limiter
        .admit_local(false, LEVEL_20, at_ms(now_ms))
        .expect_err("offence")
}

#[test]
fn mute_grows_with_the_square_of_offences_in_30_minutes() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    assert_eq!(offend(&mut limiter, 0), ChatRefusal::Muted { seconds: 5 });
    assert_eq!(
        offend(&mut limiter, 5_000),
        ChatRefusal::Muted { seconds: 20 }
    );
    assert_eq!(
        offend(&mut limiter, 25_000),
        ChatRefusal::Muted { seconds: 45 }
    );
    // 30 minutes after the first offence it no longer counts: n = 3.
    assert_eq!(
        offend(&mut limiter, 30 * 60 * 1_000),
        ChatRefusal::Muted { seconds: 45 }
    );
}

/// Back-to-back offences: n counts the offences of the last 30 minutes, this one included, and at
/// most the last 16 (the window ends the growth before the cap is reached).
#[test]
fn back_to_back_offences_count_only_the_last_30_minutes() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let mut offences: Vec<u64> = Vec::new();
    let mut now = 0;
    for _ in 0..30 {
        let n = 1 + offences
            .iter()
            .filter(|at| now - **at < 30 * 60 * 1_000)
            .count()
            .min(15) as u64;
        let seconds = 5 * n * n;
        assert_eq!(
            offend(&mut limiter, now),
            ChatRefusal::Muted {
                seconds: seconds as u32
            }
        );
        offences.push(now);
        now += seconds * 1_000;
    }
}

#[test]
fn yell_needs_level_20_or_premium_and_never_level_1() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let low = YellGate {
        level: 19,
        premium_current: false,
    };
    assert_eq!(
        limiter.admit_local(true, low, at_ms(0)),
        Err(ChatRefusal::LevelTooLow)
    );
    let level_1_premium = YellGate {
        level: 1,
        premium_current: true,
    };
    assert_eq!(
        limiter.admit_local(true, level_1_premium, at_ms(0)),
        Err(ChatRefusal::LevelTooLow)
    );
    // The gate does not apply to say; each refused yell took its line, so two are left.
    assert_eq!(limiter.admit_local(false, low, at_ms(0)), Ok(()));
    let premium = YellGate {
        level: 2,
        premium_current: true,
    };
    assert_eq!(limiter.admit_local(true, premium, at_ms(0)), Ok(()));
    assert!(matches!(
        limiter.admit_local(false, low, at_ms(0)),
        Err(ChatRefusal::Muted { .. })
    ));
}

#[test]
fn an_under_level_yell_takes_its_line_and_a_yell_beyond_the_bucket_mutes() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let low = YellGate {
        level: 19,
        premium_current: false,
    };
    for _ in 0..BUCKET_LINES {
        assert_eq!(
            limiter.admit_local(true, low, at_ms(0)),
            Err(ChatRefusal::LevelTooLow)
        );
    }
    // The bucket is checked before the level: the fifth under-level yell is an offence.
    assert_eq!(
        limiter.admit_local(true, low, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(4_999)));
}

#[test]
fn yell_has_a_30_second_cooldown_that_does_not_block_say() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    assert_eq!(limiter.admit_local(true, LEVEL_20, at_ms(0)), Ok(()));
    assert_eq!(
        limiter.admit_local(true, LEVEL_20, at_ms(100)),
        Err(ChatRefusal::Exhausted { seconds: 30 })
    );
    assert_eq!(
        limiter.admit_local(true, LEVEL_20, at_ms(29_001)),
        Err(ChatRefusal::Exhausted { seconds: 1 })
    );
    assert_eq!(limiter.admit_local(false, LEVEL_20, at_ms(100)), Ok(()));
    assert_eq!(limiter.admit_local(true, LEVEL_20, at_ms(30_000)), Ok(()));
}

#[test]
fn a_muted_character_is_refused_before_any_other_check() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    offend(&mut limiter, 0);
    let low = YellGate {
        level: 1,
        premium_current: false,
    };
    assert_eq!(
        limiter.admit_local(true, low, at_ms(1_000)),
        Err(ChatRefusal::Muted { seconds: 4 })
    );
}

/// A `now` earlier than one already seen counts as the latest time seen: it refills nothing, and
/// the mute it starts is anchored to the latest time, so it cannot end early.
#[test]
fn a_clock_going_backwards_neither_refills_nor_shortens_a_mute() {
    let mut limiter = ChatLimiter::new(at_ms(10_000));
    for _ in 0..BUCKET_LINES {
        limiter
            .admit_local(false, LEVEL_20, at_ms(10_000))
            .expect("line");
    }
    assert_eq!(
        limiter.admit_local(false, LEVEL_20, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(14_999)));
    assert_eq!(
        limiter.admit_local(false, LEVEL_20, at_ms(10_000)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert_eq!(limiter.admit_local(false, LEVEL_20, at_ms(15_000)), Ok(()));
}

#[test]
fn a_clock_going_backwards_does_not_shorten_the_yell_cooldown() {
    let mut limiter = ChatLimiter::new(at_ms(10_000));
    assert_eq!(limiter.admit_local(true, LEVEL_20, at_ms(10_000)), Ok(()));
    assert_eq!(
        limiter.admit_local(true, LEVEL_20, at_ms(0)),
        Err(ChatRefusal::Exhausted { seconds: 30 })
    );
    assert_eq!(
        limiter.admit_local(true, LEVEL_20, at_ms(39_999)),
        Err(ChatRefusal::Exhausted { seconds: 1 })
    );
}

#[test]
fn the_pre_gate_charges_a_repeated_under_level_yell_a_line_and_mutes_a_flood() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let low = YellGate {
        level: 19,
        premium_current: false,
    };
    // The first under-level yell reaches the level check of the full path and takes a line.
    assert_eq!(limiter.pre_gate(true, at_ms(0)), Ok(()));
    assert_eq!(
        limiter.admit_local(true, low, at_ms(0)),
        Err(ChatRefusal::LevelTooLow)
    );
    // Repeats inside the recheck window are refused at the gate, each taking a line.
    for ms in [0, 1_000, 2_000] {
        assert_eq!(
            limiter.pre_gate(true, at_ms(ms)),
            Err(ChatRefusal::LevelTooLow)
        );
    }
    assert!(!limiter.is_muted(at_ms(2_000)));
    // The bucket is empty: the next yell is an offence at the gate, as a say would be.
    assert_eq!(
        limiter.pre_gate(true, at_ms(2_000)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(6_999)));
    assert!(matches!(
        limiter.pre_gate(false, at_ms(2_000)),
        Err(ChatRefusal::Muted { .. })
    ));
    // After the mute (and the window) the full path checks the level again; a level gained is
    // admitted and clears the recheck.
    assert_eq!(limiter.pre_gate(true, at_ms(7_000)), Ok(()));
    assert_eq!(limiter.admit_local(true, LEVEL_20, at_ms(7_000)), Ok(()));
    assert_eq!(limiter.pre_gate(true, at_ms(40_000)), Ok(()));
}

#[test]
fn a_say_after_an_under_level_yell_is_not_refused_by_the_recheck() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let low = YellGate {
        level: 19,
        premium_current: false,
    };
    assert_eq!(limiter.pre_gate(true, at_ms(0)), Ok(()));
    assert_eq!(
        limiter.admit_local(true, low, at_ms(0)),
        Err(ChatRefusal::LevelTooLow)
    );
    // The recheck window holds yells only; the three lines left still pass as says.
    for _ in 1..BUCKET_LINES {
        assert_eq!(limiter.pre_gate(false, at_ms(0)), Ok(()));
        assert_eq!(limiter.admit_local(false, low, at_ms(0)), Ok(()));
    }
    assert!(matches!(
        limiter.pre_gate(false, at_ms(0)),
        Err(ChatRefusal::Muted { .. })
    ));
}

#[test]
fn the_pre_gate_refuses_a_yell_in_its_cooldown_and_takes_no_line() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let premium = YellGate {
        level: 20,
        premium_current: false,
    };
    assert_eq!(limiter.pre_gate(true, at_ms(0)), Ok(()));
    assert_eq!(limiter.admit_local(true, premium, at_ms(0)), Ok(()));
    assert_eq!(
        limiter.pre_gate(true, at_ms(1_000)),
        Err(ChatRefusal::Exhausted { seconds: 29 })
    );
    // The cooldown does not block say, and the refused yell took no line.
    assert_eq!(limiter.pre_gate(false, at_ms(1_000)), Ok(()));
    assert_eq!(limiter.pre_gate(true, at_ms(30_000)), Ok(()));
}

#[test]
fn the_pre_gate_mutes_an_empty_bucket_yell_before_the_level_is_known() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    let low = YellGate {
        level: 19,
        premium_current: false,
    };
    for _ in 0..BUCKET_LINES {
        assert_eq!(limiter.admit_local(false, low, at_ms(0)), Ok(()));
    }
    // A yell beyond the bucket is an offence at the gate, without the Channel's level.
    assert_eq!(
        limiter.pre_gate(true, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(0)));
}

#[test]
fn a_relay_line_takes_its_line_at_the_gate_and_an_empty_bucket_mutes() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    for _ in 0..BUCKET_LINES {
        limiter.pre_gate_relay(false, at_ms(0)).expect("line");
    }
    assert_eq!(
        limiter.pre_gate_relay(false, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    // Room and private lines share the bucket and the mute with local speech.
    assert_eq!(
        limiter.admit_local(false, LEVEL_20, at_ms(1_000)),
        Err(ChatRefusal::Muted { seconds: 4 })
    );
    assert!(limiter.is_muted(at_ms(1_000)));
    limiter
        .pre_gate_relay(false, at_ms(5_000))
        .expect("after the mute");
}

#[test]
fn advertising_allows_one_line_per_two_minutes() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    limiter.pre_gate_relay(true, at_ms(0)).expect("line");
    limiter
        .claim_advertising(at_ms(0))
        .expect("first advertisement");
    assert_eq!(
        limiter.pre_gate_relay(true, at_ms(10_000)),
        Err(ChatRefusal::Exhausted { seconds: 110 })
    );
    // The interval is checked again at the claim: a line racing past the gate is refused.
    assert_eq!(
        limiter.claim_advertising(at_ms(10_000)),
        Err(ChatRefusal::Exhausted { seconds: 110 })
    );
    // Other rooms are not held back by it.
    limiter
        .pre_gate_relay(false, at_ms(10_000))
        .expect("world line");
    limiter.pre_gate_relay(true, at_ms(120_000)).expect("line");
    limiter
        .claim_advertising(at_ms(120_000))
        .expect("second advertisement");
}

#[test]
fn a_twenty_first_distinct_recipient_in_ten_minutes_mutes() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    for id in 0..20_u8 {
        limiter
            .admit_recipient([id; 16], at_ms(0))
            .expect("recipient");
    }
    // A recipient already written to is always admitted.
    limiter
        .admit_recipient([3; 16], at_ms(1_000))
        .expect("known");
    assert_eq!(
        limiter.admit_recipient([20; 16], at_ms(1_000)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(1_000)));
    // Ten minutes after the first ones, they leave the window (recipient 3 stays: it was written
    // to again), so new recipients are admitted.
    for id in 20..39_u8 {
        limiter
            .admit_recipient([id; 16], at_ms(600_000))
            .expect("new window");
    }
    assert!(limiter.admit_recipient([99; 16], at_ms(600_000)).is_err());
}
