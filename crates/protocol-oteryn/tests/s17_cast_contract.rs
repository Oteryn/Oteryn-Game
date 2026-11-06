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
