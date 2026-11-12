//! CLIENT-C17d replay. The production consumers are `Session::exchange_cast` in
//! `crates/session/src/session_exchange.rs` (the ACTOR_VITALS delta) and
//! `presentation::apply` in `crates/session/src/presentation.rs` (the attributed damage cue).
use super::*;
use oteryn_protocol_oteryn::actor_spell::{self, ActorVitals};
use oteryn_protocol_oteryn::presentation::{
    self, PresentationColor, PresentationEvent, PresentationEventKind, PresentationValue,
    PresentationValueKind,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

fn fixtures() -> Result<(Vec<u8>, Vec<u8>, u32, u32), BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        let expected = std::fs::read_to_string(directory.join("c17d-expected.txt"))?;
        let value = |key: &str| -> Result<u32, BoxError> {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(key))
                .ok_or_else(|| -> BoxError {
                    format!("missing {key} in c17d-expected.txt").into()
                })?
                .parse()
                .map_err(Into::into)
        };
        return Ok((
            std::fs::read(directory.join("c17d-vitals.bin"))?,
            std::fs::read(directory.join("c17d-presentation.bin"))?,
            value("mana ")?,
            value("damage ")?,
        ));
    }
    let vitals = ActorVitals {
        health: 145,
        max_health: 150,
        mana: 45,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    };
    let target = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let attacker = EntityRef {
        identity: [2; 16],
        generation: 1,
    };
    let damage = 7;
    let events = [PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::ValueText {
            kind: PresentationValueKind::DamageReceived,
            position: ActorPosition {
                x: 1,
                y: 2,
                floor: 0,
            },
            target,
            attacker: Some(attacker),
            primary: PresentationValue {
                value: u64::from(damage),
                color: PresentationColor::Red,
            },
            secondary: None,
        },
    }];
    Ok((
        actor_spell::encode_actor_vitals(&vitals).map_err(|error| format!("{error:?}"))?,
        presentation::encode_presentation_batch(&events).map_err(|error| format!("{error:?}"))?,
        vitals.mana,
        damage,
    ))
}

#[test]
fn real_challenge_payloads_update_vitals_and_deliver_forced_target_damage() -> Result<(), BoxError>
{
    block_on(async {
        let (vitals_payload, presentation_payload, expected_mana, expected_damage) = fixtures()?;
        let initial = ActorVitals {
            health: 150,
            max_health: 150,
            mana: 55,
            max_mana: 55,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        let initial_payload =
            actor_spell::encode_actor_vitals(&initial).map_err(|error| format!("{error:?}"))?;
        let raw = vec![
            (
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                3,
                actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                initial_payload,
            ),
            (
                presentation::STATE_DOMAIN_WORLD_PRESENTATION,
                0,
                1,
                presentation::encode_presentation_snapshot(),
            ),
            (
                presentation::STATE_DOMAIN_ACTOR_COOLDOWNS,
                0,
                1,
                presentation::encode_actor_cooldowns(&presentation::ActorCooldowns {
                    server_now_ms: 0,
                    rtt_ms: None,
                    entries: vec![],
                })
                .map_err(|error| format!("{error:?}"))?,
            ),
        ];
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[6, 23],
                &[6, 23],
                &[],
                entity_snapshot(vec![]),
                &raw,
            )
            .await
        });
        let mut session = Session::admit(client, admission_with(&[6, 23])?).await?;
        peer.await??;

        let vitals_frame = encode_state_delta(
            1,
            41,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &vitals_payload,
        )?;
        session.apply_state_delta(41, decode_wire_envelope(&vitals_frame)?.payload())?;
        let presentation_frame = encode_state_delta(
            1,
            42,
            presentation::STATE_DOMAIN_WORLD_PRESENTATION,
            0,
            1,
            1,
            &presentation_payload,
        )?;
        let presentation_event =
            session.apply_state_delta(42, decode_wire_envelope(&presentation_frame)?.payload())?;

        assert_eq!(
            session.actor_vitals().map(|vitals| vitals.mana),
            Some(expected_mana)
        );
        assert!(matches!(&presentation_event,
            SessionEvent::Presentation(delta) if delta.value.iter().any(|event| matches!(event.kind,
                PresentationEventKind::ValueText { attacker: Some(_), primary, .. }
                    if primary.value == u64::from(expected_damage)))));

        let unchanged = session.actor_vitals().copied();
        let malformed = encode_state_delta(
            1,
            43,
            presentation::STATE_DOMAIN_WORLD_PRESENTATION,
            1,
            2,
            1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&malformed)?.payload())
                .is_err()
        );
        assert_eq!(session.actor_vitals().copied(), unchanged);
        assert!(session.take_events().is_empty());
        Ok::<_, BoxError>(())
    })?
}
