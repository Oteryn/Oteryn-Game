#![allow(clippy::expect_used)]

use oteryn_simulation_determinism::SemanticTimeMicros;

use super::greeting::DEFAULT_TALK_RANGE;
use super::spam::BUCKET_LINES;
use oteryn_protocol_oteryn::chat::{ChatLine, ChatRoom, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

use super::*;

fn at_ms(ms: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(ms * 1_000)
}

fn pos(x: i32, y: i32, floor: u8) -> ChatPosition {
    ChatPosition { x, y, floor }
}

const LEVEL_20: YellGate = YellGate {
    level: 20,
    premium_current: false,
};

fn text(s: &str) -> ChatText {
    ChatText::parse(s).expect("valid text")
}

#[test]
fn text_is_trimmed_and_bounded_in_characters_and_bytes() {
    assert_eq!(text("  hello  ").as_str(), "hello");
    assert_eq!(ChatText::parse("   "), Err(TextError::Empty));
    assert_eq!(ChatText::parse(""), Err(TextError::Empty));
    assert!(ChatText::parse(&"a".repeat(255)).is_ok());
    assert_eq!(ChatText::parse(&"a".repeat(256)), Err(TextError::TooLong));
    // 255 four-byte scalars are exactly 1,020 bytes.
    assert!(ChatText::parse(&"😀".repeat(255)).is_ok());
    assert_eq!(
        ChatText::parse(&format!(" {}", "😀".repeat(255))),
        Err(TextError::TooLong)
    );
    assert_eq!(ChatText::parse("a\nb"), Err(TextError::ControlCharacter));
    assert_eq!(ChatText::parse("a\u{7}"), Err(TextError::ControlCharacter));
    assert_eq!(ChatText::parse("tab\t"), Err(TextError::ControlCharacter));
}

#[test]
fn yell_upper_cases_without_changing_the_length() {
    assert_eq!(text("hi there, é!").yelled().as_str(), "HI THERE, É!");
    // `ß` upper-cases to two characters and stays as it is.
    assert_eq!(text("straße").yelled().as_str(), "STRAßE");
}

#[test]
fn say_reaches_the_same_floor_within_8_by_6() {
    let speaker = pos(100, 100, 7);
    assert_eq!(hears(SpeechMode::Say, speaker, speaker), Some(Heard::Text));
    assert_eq!(
        hears(SpeechMode::Say, speaker, pos(108, 94, 7)),
        Some(Heard::Text)
    );
    assert_eq!(hears(SpeechMode::Say, speaker, pos(109, 100, 7)), None);
    assert_eq!(hears(SpeechMode::Say, speaker, pos(100, 107, 7)), None);
    assert_eq!(hears(SpeechMode::Say, speaker, pos(100, 100, 6)), None);
}

#[test]
fn whisper_text_reaches_adjacent_tiles_and_others_in_range_see_pspsps() {
    let speaker = pos(100, 100, 7);
    assert_eq!(
        hears(SpeechMode::Whisper, speaker, speaker),
        Some(Heard::Text)
    );
    assert_eq!(
        hears(SpeechMode::Whisper, speaker, pos(101, 99, 7)),
        Some(Heard::Text)
    );
    assert_eq!(
        hears(SpeechMode::Whisper, speaker, pos(102, 100, 7)),
        Some(Heard::Obscured)
    );
    assert_eq!(hears(SpeechMode::Whisper, speaker, pos(109, 100, 7)), None);
    assert_eq!(hears(SpeechMode::Whisper, speaker, pos(101, 100, 8)), None);
}

#[test]
fn yell_reaches_18_by_14_over_the_canary_floors() {
    let speaker = pos(100, 100, 7);
    assert_eq!(
        hears(SpeechMode::Yell, speaker, pos(118, 86, 7)),
        Some(Heard::Text)
    );
    assert_eq!(hears(SpeechMode::Yell, speaker, pos(119, 100, 7)), None);
    assert_eq!(hears(SpeechMode::Yell, speaker, pos(100, 115, 7)), None);
    // Floor 7 reaches 0..=9; floor 6 reaches 0..=8; floor 5 reaches 0..=7.
    assert_eq!(
        hears(SpeechMode::Yell, speaker, pos(100, 100, 0)),
        Some(Heard::Text)
    );
    assert_eq!(
        hears(SpeechMode::Yell, speaker, pos(100, 100, 9)),
        Some(Heard::Text)
    );
    assert_eq!(hears(SpeechMode::Yell, speaker, pos(100, 100, 10)), None);
    assert!(hears(SpeechMode::Yell, pos(100, 100, 6), pos(100, 100, 8)).is_some());
    assert!(hears(SpeechMode::Yell, pos(100, 100, 6), pos(100, 100, 9)).is_none());
    assert!(hears(SpeechMode::Yell, pos(100, 100, 5), pos(100, 100, 8)).is_none());
    // Underground: two floors up and down.
    let deep = pos(100, 100, 12);
    assert!(hears(SpeechMode::Yell, deep, pos(100, 100, 10)).is_some());
    assert!(hears(SpeechMode::Yell, deep, pos(100, 100, 14)).is_some());
    assert!(hears(SpeechMode::Yell, deep, pos(100, 100, 9)).is_none());
    assert!(hears(SpeechMode::Yell, deep, pos(100, 100, 15)).is_none());
    assert!(hears(SpeechMode::Yell, pos(1, 1, 15), pos(1, 1, 13)).is_some());
    assert!(hears(SpeechMode::Yell, pos(1, 1, 8), pos(1, 1, 6)).is_some());
    assert!(hears(SpeechMode::Yell, pos(1, 1, 8), pos(1, 1, 5)).is_none());
}

#[test]
fn bucket_admits_four_lines_then_the_fifth_mutes_for_5_seconds() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    for _ in 0..BUCKET_LINES {
        assert_eq!(
            limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(0)),
            Ok(())
        );
    }
    assert!(!limiter.is_muted(at_ms(0)));
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(4_999)));
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(1_500)),
        Err(ChatRefusal::Muted { seconds: 4 })
    );
    assert!(!limiter.is_muted(at_ms(5_000)));
    // The bucket is full after the mute.
    for _ in 0..BUCKET_LINES {
        assert_eq!(
            limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(5_000)),
            Ok(())
        );
    }
}

#[test]
fn bucket_refills_one_line_per_2_5_seconds() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    for _ in 0..BUCKET_LINES {
        limiter
            .admit_local(SpeechMode::Say, LEVEL_20, at_ms(0))
            .expect("line");
    }
    // One line back at 2.5 s, none more before 5 s.
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(2_500)),
        Ok(())
    );
    assert!(matches!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(4_999)),
        Err(ChatRefusal::Muted { .. })
    ));

    let mut steady = ChatLimiter::new(at_ms(0));
    // One line every 2.5 s never runs dry.
    for i in 0..100 {
        assert_eq!(
            steady.admit_local(SpeechMode::Say, LEVEL_20, at_ms(i * 2_500)),
            Ok(())
        );
    }
}

fn offend(limiter: &mut ChatLimiter, now_ms: u64) -> ChatRefusal {
    for _ in 0..BUCKET_LINES {
        limiter
            .admit_local(SpeechMode::Say, LEVEL_20, at_ms(now_ms))
            .expect("line");
    }
    limiter
        .admit_local(SpeechMode::Say, LEVEL_20, at_ms(now_ms))
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
        limiter.admit_local(SpeechMode::Yell, low, at_ms(0)),
        Err(ChatRefusal::LevelTooLow)
    );
    let level_1_premium = YellGate {
        level: 1,
        premium_current: true,
    };
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, level_1_premium, at_ms(0)),
        Err(ChatRefusal::LevelTooLow)
    );
    // A refused yell takes nothing from the bucket; the gate does not apply to say.
    assert_eq!(limiter.admit_local(SpeechMode::Say, low, at_ms(0)), Ok(()));
    let premium = YellGate {
        level: 2,
        premium_current: true,
    };
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, premium, at_ms(0)),
        Ok(())
    );
}

#[test]
fn yell_has_a_30_second_cooldown_that_does_not_block_say() {
    let mut limiter = ChatLimiter::new(at_ms(0));
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(0)),
        Ok(())
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(100)),
        Err(ChatRefusal::Exhausted { seconds: 30 })
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(29_001)),
        Err(ChatRefusal::Exhausted { seconds: 1 })
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(100)),
        Ok(())
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(30_000)),
        Ok(())
    );
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
        limiter.admit_local(SpeechMode::Yell, low, at_ms(1_000)),
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
            .admit_local(SpeechMode::Say, LEVEL_20, at_ms(10_000))
            .expect("line");
    }
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(0)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert!(limiter.is_muted(at_ms(14_999)));
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(10_000)),
        Err(ChatRefusal::Muted { seconds: 5 })
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Say, LEVEL_20, at_ms(15_000)),
        Ok(())
    );
}

#[test]
fn a_clock_going_backwards_does_not_shorten_the_yell_cooldown() {
    let mut limiter = ChatLimiter::new(at_ms(10_000));
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(10_000)),
        Ok(())
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(0)),
        Err(ChatRefusal::Exhausted { seconds: 30 })
    );
    assert_eq!(
        limiter.admit_local(SpeechMode::Yell, LEVEL_20, at_ms(39_999)),
        Err(ChatRefusal::Exhausted { seconds: 1 })
    );
}

fn greetings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn a_greeting_matches_as_whole_words_within_the_talk_range() {
    let hi = greetings(&["hi", "hello"]);
    let npc = GreetingNpc {
        actor_id: 7,
        position: pos(104, 100, 7),
        greetings: &hi,
        talk_range: DEFAULT_TALK_RANGE,
    };
    let speaker = pos(100, 100, 7);
    let npcs = [npc.clone()];
    assert_eq!(greeted_npc(speaker, &text("Hi!"), &npcs), Some(7));
    assert_eq!(
        greeted_npc(speaker, &text("well, HELLO there"), &npcs),
        Some(7)
    );
    assert_eq!(greeted_npc(speaker, &text("this is it"), &npcs), None);
    assert_eq!(greeted_npc(speaker, &text("hiya"), &npcs), None);
    assert_eq!(greeted_npc(pos(95, 100, 7), &text("hi"), &npcs), None);
    assert_eq!(greeted_npc(pos(104, 100, 6), &text("hi"), &npcs), None);
}

#[test]
fn a_multi_word_greeting_needs_its_words_in_order() {
    let hail = greetings(&["hail king"]);
    let npcs = [GreetingNpc {
        actor_id: 3,
        position: pos(100, 101, 7),
        greetings: &hail,
        talk_range: DEFAULT_TALK_RANGE,
    }];
    let speaker = pos(100, 100, 7);
    assert_eq!(
        greeted_npc(speaker, &text("HAIL, King Tibianus"), &npcs),
        Some(3)
    );
    assert_eq!(greeted_npc(speaker, &text("hi king"), &npcs), None);
    assert_eq!(greeted_npc(speaker, &text("king hail"), &npcs), None);
}

#[test]
fn the_nearest_greeted_npc_wins_then_the_lowest_actor_id() {
    let hi = greetings(&["hi"]);
    let npc = |actor_id, x| GreetingNpc {
        actor_id,
        position: pos(x, 100, 7),
        greetings: &hi,
        talk_range: DEFAULT_TALK_RANGE,
    };
    let speaker = pos(100, 100, 7);
    assert_eq!(
        greeted_npc(
            speaker,
            &text("hi"),
            &[npc(9, 103), npc(8, 102), npc(5, 98)]
        ),
        Some(5)
    );
    assert_eq!(
        greeted_npc(speaker, &text("hi"), &[npc(9, 103), npc(4, 104)]),
        Some(9)
    );
}

fn room_text(n: usize) -> ChatLine {
    room_line(ChatRoom::World, "A", &text(&format!("line {n}")))
}

fn speaker() -> ChatSpeaker {
    ChatSpeaker {
        identity: [1; 16],
        generation: std::num::NonZeroU64::new(1).expect("nonzero"),
    }
}

fn drain(egress: &mut ChatEgress) -> Vec<ChatLine> {
    std::iter::from_fn(|| egress.pop()).collect()
}

#[test]
fn egress_keeps_64_lines_and_owes_one_marker_for_dropped_room_lines() {
    let mut egress = ChatEgress::new();
    for n in 0..EGRESS_MAX_LINES {
        egress.push(room_text(n));
    }
    assert_eq!(egress.len(), 64);
    for n in 64..100 {
        egress.push(room_text(n));
    }
    assert_eq!(egress.len(), 64);
    let out = drain(&mut egress);
    assert_eq!(out.len(), 65);
    assert_eq!(out[0], ChatLine::Dropped);
    assert_eq!(out[1], room_text(36));
    assert_eq!(out[64], room_text(99));
    assert!(egress.is_empty());
}

#[test]
fn egress_drops_room_then_local_then_private() {
    let mut egress = ChatEgress::new();
    egress.push(private_line("P", &text("secret")));
    let local = local_line(
        speaker(),
        "S",
        SpeechMode::Say,
        &text("hi"),
        pos(1, 1, 7),
        Heard::Text,
    );
    egress.push(local.clone());
    for n in 0..62 {
        egress.push(room_text(n));
    }
    // Full: a new local line evicts the oldest room line, a new room line is itself dropped
    // only when no room line is queued.
    egress.push(local.clone());
    assert_eq!(egress.len(), 64);
    for n in 0..61 {
        egress.push(local.clone());
        let _ = n;
    }
    // Now no room lines remain; a room line is dropped on arrival.
    egress.push(room_text(1000));
    let out = drain(&mut egress);
    assert!(out.iter().all(|l| !matches!(l, ChatLine::Room { .. })));
    assert_eq!(out[0], ChatLine::Dropped);
    assert!(out.contains(&private_line("P", &text("secret"))));
    // A full queue of private lines drops an arriving local line: private lines go last.
    let mut egress = ChatEgress::new();
    for _ in 0..64 {
        egress.push(private_line("P", &text("x")));
    }
    egress.push(local);
    let out = drain(&mut egress);
    assert_eq!(out.len(), 65);
    assert!(
        out[1..]
            .iter()
            .all(|l| matches!(l, ChatLine::Private { .. }))
    );
}

#[test]
fn egress_clear_drops_lines_and_marker() {
    let mut egress = ChatEgress::new();
    for n in 0..70 {
        egress.push(room_text(n));
    }
    egress.clear();
    assert!(egress.is_empty());
    assert_eq!(egress.pop(), None);
}

#[test]
fn listeners_follow_the_range_rules_in_the_order_given() {
    let speaker_at = pos(100, 100, 7);
    let listeners = [
        (1u8, pos(100, 100, 7)),
        (2, pos(120, 100, 7)),
        (3, pos(101, 100, 7)),
        (4, pos(105, 100, 7)),
        (5, pos(100, 100, 6)),
    ];
    assert_eq!(
        local_listeners(SpeechMode::Whisper, speaker_at, listeners),
        vec![(1, Heard::Text), (3, Heard::Text), (4, Heard::Obscured)]
    );
    assert_eq!(
        local_listeners(SpeechMode::Say, speaker_at, listeners)
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>(),
        vec![1, 3, 4]
    );
    assert_eq!(
        local_listeners(SpeechMode::Yell, speaker_at, listeners)
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>(),
        vec![1, 3, 4, 5]
    );
}

#[test]
fn line_values_carry_the_mode_text_and_position() {
    let at = pos(10, -2, 7);
    let say = local_line(
        speaker(),
        "Ann",
        SpeechMode::Yell,
        &text("hey"),
        at,
        Heard::Text,
    );
    assert_eq!(
        say,
        ChatLine::Local {
            speaker: speaker(),
            speaker_name: "Ann".to_owned(),
            mode: ChatSpeechMode::Yell,
            text: "HEY".to_owned(),
            position: ActorPosition {
                x: 10,
                y: -2,
                floor: 7
            },
        }
    );
    let whisper = local_line(
        speaker(),
        "Ann",
        SpeechMode::Whisper,
        &text("secret"),
        at,
        Heard::Obscured,
    );
    assert!(
        matches!(whisper, ChatLine::Local { ref text, mode: ChatSpeechMode::Whisper, .. } if text == "pspsps")
    );
    assert_eq!(
        private_line("Bob", &text("yo")),
        ChatLine::Private {
            speaker_name: "Bob".to_owned(),
            text: "yo".to_owned()
        }
    );
    // Every built line encodes on the wire.
    for line in [say, whisper, room_text(1), private_line("Bob", &text("yo"))] {
        assert!(oteryn_protocol_oteryn::chat::encode_chat_line(&line).is_ok());
    }
}
