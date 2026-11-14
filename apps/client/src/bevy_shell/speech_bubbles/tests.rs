use super::logic::*;
use oteryn_client_runtime::chat::{ActorPosition, ChatLine, ChatSpeaker, ChatSpeechMode};
use std::num::NonZeroU64;

fn speech(name: &str, mode: ChatSpeechMode, text: &str, position: Position) -> Speech {
    Speech {
        name: name.into(),
        mode,
        text: text.into(),
        position,
    }
}

fn local(name: &str, text: &str, x: i32) -> ChatLine {
    ChatLine::Local {
        speaker: ChatSpeaker {
            identity: Default::default(),
            generation: NonZeroU64::MIN,
        },
        speaker_name: name.into(),
        mode: ChatSpeechMode::Say,
        text: text.into(),
        position: ActorPosition { x, y: 2, floor: 7 },
    }
}

const HERE: Position = (10, 20, 7);

fn first(texts: &Texts) -> Option<&StaticText> {
    texts.on_floor(7).next()
}

#[test]
fn same_position_name_and_mode_join_one_text() {
    let mut texts = Texts::default();
    texts.add(&speech("Ann", ChatSpeechMode::Say, "one", HERE), 0);
    texts.add(&speech("Ann", ChatSpeechMode::Say, "two", HERE), 10);
    assert_eq!(texts.on_floor(7).count(), 1);
    assert_eq!(
        first(&texts).map(StaticText::compose).as_deref(),
        Some("Ann says:\none\ntwo")
    );
}

#[test]
fn another_name_mode_or_position_is_a_separate_text() {
    let mut texts = Texts::default();
    texts.add(&speech("Ann", ChatSpeechMode::Say, "a", HERE), 0);
    texts.add(&speech("Bob", ChatSpeechMode::Say, "b", HERE), 0);
    texts.add(&speech("Ann", ChatSpeechMode::Yell, "c", HERE), 0);
    texts.add(&speech("Ann", ChatSpeechMode::Say, "d", (11, 20, 7)), 0);
    assert_eq!(texts.on_floor(7).count(), 4);
}

#[test]
fn a_text_keeps_ten_messages_and_drops_the_oldest() {
    let mut texts = Texts::default();
    for n in 0..12 {
        texts.add(
            &speech("Ann", ChatSpeechMode::Say, &format!("m{n}"), HERE),
            0,
        );
    }
    let text = first(&texts).map(|t| (t.len(), t.compose()));
    let expected = format!(
        "Ann says:\n{}",
        (2..12)
            .map(|n| format!("m{n}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(text, Some((MAX_MESSAGES, expected)));
}

#[test]
fn lifetime_is_sixty_ms_per_char_at_least_three_seconds_and_doubled_for_a_yell() {
    assert_eq!(lifetime_ms(ChatSpeechMode::Say, "hi"), 3000);
    assert_eq!(lifetime_ms(ChatSpeechMode::Say, &"x".repeat(50)), 3000);
    assert_eq!(lifetime_ms(ChatSpeechMode::Say, &"x".repeat(100)), 6000);
    assert_eq!(lifetime_ms(ChatSpeechMode::Whisper, &"x".repeat(100)), 6000);
    assert_eq!(lifetime_ms(ChatSpeechMode::Yell, "hi"), 6000);
    assert_eq!(lifetime_ms(ChatSpeechMode::Yell, &"x".repeat(100)), 12000);
    assert_eq!(lifetime_ms(ChatSpeechMode::Say, &"ł".repeat(100)), 6000);
}

#[test]
fn the_first_message_expires_alone_then_the_empty_text_goes() {
    let mut texts = Texts::default();
    texts.add(&speech("Ann", ChatSpeechMode::Say, "one", HERE), 0);
    texts.add(&speech("Ann", ChatSpeechMode::Say, "two", HERE), 2000);
    texts.expire(2999);
    assert_eq!(first(&texts).map(StaticText::len), Some(2));
    texts.expire(3000);
    assert_eq!(
        first(&texts).map(StaticText::compose).as_deref(),
        Some("Ann says:\ntwo")
    );
    texts.expire(5000);
    assert!(first(&texts).is_none());
}

#[test]
fn header_and_colour_follow_the_mode() {
    assert_eq!(header(ChatSpeechMode::Say, "Ann"), "Ann says:\n");
    assert_eq!(header(ChatSpeechMode::Whisper, "Ann"), "Ann whispers:\n");
    assert_eq!(header(ChatSpeechMode::Yell, "Ann"), "Ann yells:\n");
    assert_eq!(header(ChatSpeechMode::MonsterSay, "Rat"), "");
    for mode in [
        ChatSpeechMode::Say,
        ChatSpeechMode::Whisper,
        ChatSpeechMode::Yell,
    ] {
        assert_eq!(colour(mode), [239, 239, 0]);
    }
    assert_eq!(colour(ChatSpeechMode::MonsterSay), [254, 101, 0]);
    let mut texts = Texts::default();
    texts.add(
        &speech("Rat", ChatSpeechMode::MonsterSay, "squeak", HERE),
        0,
    );
    assert_eq!(
        first(&texts).map(StaticText::compose).as_deref(),
        Some("squeak")
    );
}

#[test]
fn only_the_camera_floor_is_drawn() {
    let mut texts = Texts::default();
    texts.add(&speech("Ann", ChatSpeechMode::Say, "up", (1, 1, 6)), 0);
    texts.add(&speech("Bob", ChatSpeechMode::Say, "here", (1, 1, 7)), 0);
    let names: Vec<_> = texts.on_floor(7).map(StaticText::compose).collect();
    assert_eq!(names, ["Bob says:\nhere"]);
}

#[test]
fn wrap_width_is_275() {
    assert_eq!(WRAP_WIDTH_PX, 275.0);
}

#[test]
fn rect_is_centred_above_the_tile_and_bound_inside_the_viewport() {
    assert_eq!(
        layout_rect((200.0, 100.0), (80.0, 30.0), (500.0, 400.0)),
        (180.0, 75.0)
    );
    assert_eq!(
        layout_rect((0.0, 10.0), (80.0, 30.0), (500.0, 400.0)),
        (0.0, 0.0)
    );
    assert_eq!(
        layout_rect((490.0, 390.0), (80.0, 30.0), (500.0, 400.0)),
        (420.0, 365.0)
    );
}

#[test]
fn only_local_lines_become_speech_and_only_new_ones_are_taken() {
    assert_eq!(Speech::from_line(&ChatLine::Dropped), None);
    assert_eq!(
        Speech::from_line(&local("Ann", "hi", 5)),
        Some(speech("Ann", ChatSpeechMode::Say, "hi", (5, 2, 7)))
    );
    let a = local("Ann", "a", 1);
    let b = local("Ann", "b", 2);
    let c = local("Ann", "c", 3);
    assert_eq!(
        new_lines(&[], std::slice::from_ref(&a)),
        std::slice::from_ref(&a)
    );
    assert_eq!(
        new_lines(&[a.clone(), b.clone()], &[a.clone(), b.clone(), c.clone()]),
        std::slice::from_ref(&c)
    );
    // A full log drops its oldest line as it takes the newest.
    assert_eq!(
        new_lines(&[a, b.clone()], &[b.clone(), c.clone()]),
        std::slice::from_ref(&c)
    );
    assert!(new_lines(std::slice::from_ref(&b), std::slice::from_ref(&b)).is_empty());
}
