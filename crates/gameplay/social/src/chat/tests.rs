#![allow(clippy::expect_used)]

use super::greeting::DEFAULT_TALK_RANGE;
use oteryn_protocol_oteryn::chat::{ChatLine, ChatRoom, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

use super::*;

fn pos(x: i32, y: i32, floor: u8) -> ChatPosition {
    ChatPosition { x, y, floor }
}

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

#[test]
fn a_sent_marker_covers_further_overflow_until_a_line_is_delivered() {
    let mut egress = ChatEgress::new();
    for n in 0..EGRESS_MAX_LINES {
        egress.push(room_text(n));
    }
    egress.push(room_text(100));
    assert_eq!(egress.pop(), Some(ChatLine::Dropped));
    egress.push(room_text(101));
    assert_eq!(egress.pop(), Some(room_text(2)));
    // A line was delivered: the next overflow owes a new marker.
    egress.push(room_text(102));
    egress.push(room_text(103));
    assert_eq!(egress.pop(), Some(ChatLine::Dropped));
}

#[test]
fn a_line_being_written_counts_against_the_bound_until_delivered() {
    let mut egress = ChatEgress::new();
    egress.push(private_line("Alice", &text("first")));
    for n in 1..EGRESS_MAX_LINES {
        egress.push(room_text(n));
    }
    assert_eq!(
        egress.next_for_write(),
        Some(private_line("Alice", &text("first")))
    );
    // 63 queued plus the one being written fill the bound: a new room line evicts the oldest
    // queued room line and owes one marker.
    egress.push(room_text(100));
    assert_eq!(egress.len(), EGRESS_MAX_LINES - 1);
    assert_eq!(egress.next_for_write(), Some(ChatLine::Dropped));
    assert_eq!(egress.next_for_write(), Some(room_text(2)));
}

#[test]
fn the_line_being_written_keeps_its_slot_and_is_never_retracted() {
    let mut egress = ChatEgress::new();
    egress.push(room_text(0));
    for n in 1..EGRESS_MAX_LINES {
        egress.push(private_line("Alice", &text(&format!("p{n}"))));
    }
    assert_eq!(egress.next_for_write(), Some(room_text(0)));
    // 63 queued plus the one being written fill the bound: an equal-rank private line evicts
    // the oldest queued private line, so queued plus in-flight never exceed 64.
    egress.push(private_line("Bob", &text("new")));
    assert_eq!(egress.len(), EGRESS_MAX_LINES - 1);
    assert_eq!(egress.next_for_write(), Some(ChatLine::Dropped));
}

#[test]
fn a_marker_keeps_covering_overflow_until_a_line_is_delivered() {
    let mut egress = ChatEgress::new();
    egress.push(private_line("Alice", &text("first")));
    for n in 1..EGRESS_MAX_LINES {
        egress.push(room_text(n));
    }
    egress.push(room_text(100));
    assert_eq!(egress.next_for_write(), Some(ChatLine::Dropped));
    assert!(egress.next_for_write().is_some());
    // The line is written but not yet delivered: the marker still covers this overflow.
    egress.push(room_text(101));
    assert_eq!(egress.len(), EGRESS_MAX_LINES - 1);
    egress.delivered();
    // Delivered: the marker no longer covers later overflow.
    egress.push(room_text(102));
    egress.push(room_text(103));
    assert_eq!(egress.next_for_write(), Some(ChatLine::Dropped));
}

#[test]
fn rooms_need_level_20_or_premium_except_help_and_private_needs_level_3() {
    let gate = |level, premium_current| YellGate {
        level,
        premium_current,
    };
    for room in [ChatRoom::World, ChatRoom::English, ChatRoom::Advertising] {
        assert!(!gate(19, false).allows_room(room), "{room:?}");
        assert!(gate(20, false).allows_room(room), "{room:?}");
        assert!(gate(2, true).allows_room(room), "{room:?}");
        assert!(gate(1, true).allows_room(room), "{room:?}");
    }
    assert!(gate(1, false).allows_room(ChatRoom::Help));
    assert!(!gate(2, false).allows_private());
    assert!(gate(3, false).allows_private());
}
