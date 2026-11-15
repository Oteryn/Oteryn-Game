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
fn legacy_floor_speech_is_drawn_on_the_native_live_map_floor() {
    // Chat carries the legacy floor (surface 7); a live map view is on its native floor (-7).
    let mut texts = Texts::default();
    texts.add(&speech("Ann", ChatSpeechMode::Say, "up", (1, 1, 6)), 0);
    texts.add(&speech("Bob", ChatSpeechMode::Say, "here", (1, 1, 7)), 0);
    let names: Vec<_> = texts.on_floor(-7).map(StaticText::compose).collect();
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
        new_lines(0, 1, std::slice::from_ref(&a)),
        std::slice::from_ref(&a)
    );
    assert_eq!(
        new_lines(2, 3, &[a.clone(), b.clone(), c.clone()]),
        std::slice::from_ref(&c)
    );
    // A full log drops its oldest line as it takes the newest.
    assert_eq!(
        new_lines(2, 3, &[b.clone(), c.clone()]),
        std::slice::from_ref(&c)
    );
    assert!(new_lines(3, 3, std::slice::from_ref(&b)).is_empty());
}

#[test]
fn an_unmeasured_text_stays_visible_to_layout_and_transparent() {
    use bevy::prelude::{Alpha, Color, TextColor, Visibility, World};
    let mut world = World::new();
    let entity = world.spawn(super::unmeasured(Color::WHITE)).id();
    assert_ne!(world.get::<Visibility>(entity), Some(&Visibility::Hidden));
    assert_eq!(
        world.get::<TextColor>(entity).map(|c| c.0.alpha()),
        Some(0.)
    );
}

fn bubble_app() -> Result<(bevy::prelude::App, crate::play::Outbox), Box<dyn std::error::Error>> {
    use super::super::{
        game_ui::viewport::WorldViewport,
        owner::{ActivePlay, HostOwner},
        ui::Skins,
    };
    use bevy::prelude::*;
    let mut host = HostOwner::new(None)?;
    let (link, _, outbox) = crate::play::play_channel();
    host.play = Some(ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(10, 2), 7, &[])?,
        link,
    });
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<WorldViewport>()
        .init_resource::<Time>()
        .init_resource::<super::SpeechBubbles>()
        .insert_resource(Skins {
            buttons: oteryn_client_ui_bevy::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        })
        .add_systems(Update, super::project);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    Ok((app, outbox))
}

fn receive(
    app: &mut bevy::prelude::App,
    outbox: &crate::play::Outbox,
    lines: Vec<ChatLine>,
    serial: u64,
) {
    let line = lines.last().cloned();
    if let Some(line) = line {
        outbox.push(vec![oteryn_session::SessionEvent::ChatLine(
            oteryn_session::AppliedDelta {
                server_sequence: serial,
                base_revision: serial - 1,
                new_revision: serial,
                value: line,
            },
        )]);
    }
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(outbox, Default::default(), lines);
    app.update();
}

#[test]
fn repeated_speech_in_a_full_log_refreshes_through_project()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, outbox) = bubble_app()?;
    let lines = vec![local("Ann", "again", 10); 64];
    receive(&mut app, &outbox, lines.clone(), 1);
    receive(&mut app, &outbox, lines, 2);
    let state = app.world().resource::<super::SpeechBubbles>();
    assert_eq!(first(&state.texts).map(StaticText::len), Some(2));
    Ok(())
}

#[test]
fn communication_filters_apply_before_speech_is_retained() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_client_ui::chat::CommunicationList;
    for list in [CommunicationList::Ignore, CommunicationList::Blacklist] {
        let (mut app, outbox) = bubble_app()?;
        app.world()
            .non_send::<super::super::owner::HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .chat_communication(Some(list), "Ann", true);
        receive(&mut app, &outbox, vec![local("Ann", "hidden", 10)], 1);
        assert!(first(&app.world().resource::<super::SpeechBubbles>().texts).is_none());
    }
    Ok(())
}

#[test]
fn speech_tracks_the_walk_camera_before_layout() -> Result<(), Box<dyn std::error::Error>> {
    use bevy::prelude::*;
    let (mut app, outbox) = bubble_app()?;
    receive(&mut app, &outbox, vec![local("Ann", "hello", 10)], 1);
    app.update();
    let entity = app.world().resource::<super::SpeechBubbles>().entities[0];
    // Feed a measured layout to the real projector without loading external font assets.
    app.world_mut()
        .entity_mut(entity)
        .insert(bevy::text::TextLayoutInfo {
            size: Vec2::new(80., 20.),
            ..default()
        });
    app.update();
    let before = app
        .world()
        .get::<Transform>(entity)
        .ok_or("transform")?
        .translation;
    {
        let mut host = app
            .world_mut()
            .non_send_mut::<super::super::owner::HostOwner>();
        host.play
            .as_mut()
            .ok_or("play")?
            .view
            .apply(&oteryn_session::StepOutcome {
                step_wait_ms: 0,
                command_id: 1,
                status: oteryn_session::CommandStatus::Accepted,
                disposition: oteryn_session::StepDisposition::Moved,
                result_server_sequence: 1,
                world_spatial_delta: Some(oteryn_session::AppliedDelta {
                    server_sequence: 1,
                    base_revision: 0,
                    new_revision: 1,
                    value: oteryn_session::WorldSpatialObservation {
                        content_generation: [0; 32],
                        actor_position: ActorPosition {
                            x: 11,
                            y: 2,
                            floor: 7,
                        },
                    },
                }),
                world_object_overlay_delta: None,
                gate_message_id: None,
            })?;
    }
    app.update();
    let after = app
        .world()
        .get::<Transform>(entity)
        .ok_or("transform")?
        .translation;
    assert!(
        (after.x - before.x).abs() < 2.,
        "bubble must glide with speaker: {before:?} -> {after:?}"
    );
    Ok(())
}

#[test]
fn committed_activity_scope_discards_source_speech() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{AppliedDelta, GatedSnapshot, SessionEvent, activity_scope::*};
    let (mut app, outbox) = bubble_app()?;
    receive(&mut app, &outbox, vec![local("Ann", "source", 10)], 1);
    assert!(first(&app.world().resource::<super::SpeechBubbles>().texts).is_some());
    let id = |n| {
        let mut id = [n; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    };
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id(1),
        source_session: id(2),
        session: id(3),
        scope: ActivityScope::Instance {
            world: id(4),
            instance: id(5),
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![117],
        requires_ack: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![SessionEvent::GatedSnapshotReplaced(AppliedDelta {
        server_sequence: 2,
        base_revision: 0,
        new_revision: 1,
        value: GatedSnapshot {
            domain_id: STATE_DOMAIN_ACTIVITY_SCOPE,
            snapshot_type: 1,
            revision: 1,
            payload,
        },
    })]);
    app.update();
    assert!(first(&app.world().resource::<super::SpeechBubbles>().texts).is_none());
    Ok(())
}

#[test]
fn speech_receipts_wait_for_the_matching_log_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, outbox) = bubble_app()?;
    let first_line = local("Ann", "one", 10);
    let next_line = local("Bob", "two", 11);
    receive(&mut app, &outbox, vec![first_line.clone()], 1);
    outbox.push(vec![oteryn_session::SessionEvent::ChatLine(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: next_line.clone(),
        },
    )]);
    app.update();
    assert_eq!(
        first(&app.world().resource::<super::SpeechBubbles>().texts).map(StaticText::len),
        Some(1)
    );
    crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
        &outbox,
        Default::default(),
        vec![first_line, next_line],
    );
    app.update();
    let state = app.world().resource::<super::SpeechBubbles>();
    assert_eq!(state.texts.on_floor(7).count(), 2);
    assert!(
        state
            .texts
            .on_floor(7)
            .any(|text| text.compose() == "Bob says:\ntwo")
    );
    Ok(())
}
