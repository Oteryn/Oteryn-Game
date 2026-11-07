use oteryn_protocol_oteryn::cast::{Envelope, Message, Selection};

#[test]
fn strict_cast_wire_roundtrips_boundaries_and_refuses_gameplay_and_unknown_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let scene = Message::Scene {
        epoch: 1,
        sequence: 1,
        scene: oteryn_protocol_oteryn::cast::Scene {
            tiles: vec![(0, 1, 2)],
            actors: vec![(7, 0, 1, 2, 100)],
        },
    };
    assert_eq!(
        Message::decode(&scene.encode().map_err(|e| format!("{e:?}"))?),
        Ok(scene)
    );
    for delay in [0, 300] {
        let message = Message::Start {
            revision: 0,
            access_revision: 1,
            delay,
            cap: 50,
            password: false,
        };
        let bytes = message.encode().map_err(|e| format!("{e:?}"))?;
        assert_eq!(Message::decode(&bytes), Ok(message));
        let mut unknown = bytes.clone();
        unknown.extend_from_slice(&[0x78, 1]);
        assert!(Message::decode(&unknown).is_err());
        for end in 0..bytes.len() {
            assert!(Message::decode(&bytes[..end]).is_err());
        }
    }
    assert!(
        Message::Start {
            revision: 0,
            access_revision: 1,
            delay: 301,
            cap: 50,
            password: false
        }
        .encode()
        .is_err()
    );
    let input = Envelope {
        generation: 1,
        message: Message::Chat("hello".into()),
    };
    assert_eq!(
        Envelope::decode(&input.encode().map_err(|e| format!("{e:?}"))?, true),
        Ok(input)
    );
    assert!(Envelope::decode(&[8, 7, 16, 1, 34, 0], true).is_err());
    let server = Envelope {
        generation: 1,
        message: Message::Slate {
            epoch: 1,
            sequence: 1,
        },
    };
    assert!(Envelope::decode(&server.encode().map_err(|e| format!("{e:?}"))?, true).is_err());
    assert_eq!(
        Message::decode(
            &Message::CasterChatEmpty
                .encode()
                .map_err(|e| format!("{e:?}"))?
        ),
        Ok(Message::CasterChatEmpty)
    );
    assert!(
        Envelope {
            generation: 1,
            message: Message::Slate {
                epoch: 1,
                sequence: 1
            }
        }
        .encode()
        .is_ok()
    );
    assert!(
        Selection {
            view: true,
            control: false,
            chat: true,
            consent: false
        }
        .validate(true)
    );
    assert!(
        !Selection {
            view: false,
            control: false,
            chat: true,
            consent: false
        }
        .validate(true)
    );
    Ok(())
}

#[test]
fn caster_control_state_exposes_its_opaque_cast_id_only_to_the_owner()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::cast::{Envelope, Message};
    let state = Message::CasterState {
        cast_id: [1; 16],
        revision: 1,
        epoch: 1,
        delay: 0,
        count: 0,
    };
    let bytes = state.encode().map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        Message::decode(&bytes).map_err(|e| format!("{e:?}"))?,
        state
    );
    assert!(
        Envelope {
            generation: 1,
            message: state
        }
        .encode()
        .is_err()
    );
    Ok(())
}

#[test]
fn s17_public_media_is_strict_bounded_and_never_observer_input() -> Result<(), String> {
    use oteryn_protocol_oteryn::cast::{DisplayTile, PublicMedia};
    let media = PublicMedia::Map {
        reset: true,
        last: true,
        tiles: vec![DisplayTile {
            x: 100,
            y: 200,
            floor: -7,
            appearances: vec![100, 200],
        }],
    };
    let message = Message::Media {
        epoch: 1,
        sequence: 1,
        media,
    };
    let envelope = Envelope {
        generation: 1,
        message,
    };
    let bytes = envelope.encode().map_err(|e| format!("{e:?}"))?;
    assert_eq!(Envelope::decode(&bytes, false), Ok(envelope));
    assert!(Envelope::decode(&bytes, true).is_err());
    Ok(())
}

#[test]
fn s17_media_speech_effects_and_output_names_roundtrip() -> Result<(), String> {
    use oteryn_protocol_oteryn::{
        cast::PublicMedia,
        chat::ChatSpeechMode,
        presentation::{PresentationEvent, PresentationEventKind, PresentationSource},
        world_spatial::ActorPosition,
    };
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let effect = PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::MagicEffect {
            position: ActorPosition {
                floor: 7,
                ..position
            },
            effect_id: 10,
            source: PresentationSource::Own,
        },
    };
    for media in [
        PublicMedia::Effects(vec![effect]),
        PublicMedia::Speech {
            position,
            mode: ChatSpeechMode::Say,
            text: "visible speech".into(),
        },
    ] {
        let bytes = media.encode().map_err(|e| format!("{e:?}"))?;
        assert_eq!(PublicMedia::decode(&bytes), Ok(media));
        let mut invalid = bytes;
        invalid.push(0);
        assert!(PublicMedia::decode(&invalid).is_err());
    }
    let forbidden = PublicMedia::Effects(vec![PresentationEvent {
        kind: PresentationEventKind::AmbientSound { cue_id: 1 },
        ..effect
    }]);
    assert!(forbidden.encode().is_err());
    let named = Message::ChatLine {
        epoch: 1,
        sequence: 1,
        handle: [1; 16],
        text: format!("Viewer-{}: {}", "a".repeat(32), "x".repeat(256)),
    };
    assert_eq!(
        Message::decode(&named.encode().map_err(|e| format!("{e:?}"))?),
        Ok(named)
    );
    assert!(Message::Chat("x".repeat(257)).encode().is_err());
    Ok(())
}

#[test]
fn clcast_consent_query_uses_lane_kind_and_cannot_enter_observer_envelope()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::cast::{Envelope, Message};
    let bytes = Message::ConsentQuery
        .encode()
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(bytes, [8, 52]);
    assert_eq!(
        Message::decode(&bytes).map_err(|e| format!("{e:?}"))?,
        Message::ConsentQuery
    );
    assert!(Message::decode(&[8, 52, 16, 0]).is_err());
    assert!(
        Envelope {
            generation: 1,
            message: Message::ConsentQuery
        }
        .encode()
        .is_err()
    );
    Ok(())
}

#[test]
fn clcast_ready_and_caster_roster_are_strict_and_role_separated()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::cast::{Envelope, Message};
    let ready = Message::Ready {
        epoch: 7,
        chat: false,
        delay: 300,
    };
    let bytes = Envelope {
        generation: 9,
        message: ready.clone(),
    }
    .encode()
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        Envelope::decode(&bytes, false)
            .map_err(|e| format!("{e:?}"))?
            .message,
        ready
    );
    assert!(Envelope::decode(&bytes, true).is_err());
    let roster = Message::CasterRoster {
        revision: 3,
        subjects: vec![[1; 16], [2; 16]],
    };
    assert!(roster.encode().map_err(|e| format!("{e:?}"))?.len() < 1024);
    assert_eq!(
        Message::decode(&roster.encode().map_err(|e| format!("{e:?}"))?)
            .map_err(|e| format!("{e:?}"))?,
        roster
    );
    assert!(
        Envelope {
            generation: 9,
            message: roster
        }
        .encode()
        .is_err()
    );
    assert!(
        Message::CasterRoster {
            revision: 3,
            subjects: vec![[2; 16], [1; 16]]
        }
        .encode()
        .is_err()
    );
    assert!(
        Message::CasterRoster {
            revision: 3,
            subjects: vec![[1; 16]; 51]
        }
        .encode()
        .is_err()
    );
    assert_eq!(
        Message::ControlQuery
            .encode()
            .map_err(|e| format!("{e:?}"))?,
        [8, 55]
    );
    Ok(())
}
