//! C18b replay coverage: a crosshair (position) cast driven through the production client path.
//!
//! `Session::cast_spell` in `session_commands.rs` encodes the position-target intent and calls
//! `Session::exchange_cast` in `session_exchange.rs`, which decodes the spell result and reads
//! the claimed actor-vitals delta that `domain_dispatch.rs` installs; `Session::actor_vitals`
//! (`session_access.rs`) exposes the client-visible result. The scripted server checks the
//! exact intent bytes the client sent and answers with the captured server payloads.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{
    self, ActorVitals, SpellCastDisposition, SpellCastIntent, SpellTarget, SpellTargetPosition,
};
use std::num::NonZeroU32;

struct Fixture {
    spell: NonZeroU32,
    position: SpellTargetPosition,
    result: Vec<u8>,
    vitals: Vec<u8>,
    expected: ActorVitals,
}

fn fixture() -> Result<Fixture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let text = std::fs::read_to_string(root.join("c18b-expected.txt"))?;
        let mut lines = text.lines();
        let intent = lines
            .next()
            .ok_or("C18b expected intent line absent")?
            .split_whitespace()
            .map(str::parse::<i64>)
            .collect::<Result<Vec<_>, _>>()?;
        let vitals = lines
            .next()
            .ok_or("C18b expected vitals line absent")?
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        let [spell, x, y, floor] = intent.as_slice() else {
            return Err("invalid C18b expected intent".into());
        };
        let [
            disposition,
            health,
            max_health,
            mana,
            max_mana,
            soul,
            harmony,
            serene,
        ] = vitals.as_slice()
        else {
            return Err("invalid C18b expected vitals".into());
        };
        assert_eq!(*disposition, SpellCastDisposition::Cast as u32);
        return Ok(Fixture {
            spell: NonZeroU32::new(u32::try_from(*spell)?).ok_or("C18b spell index is zero")?,
            position: SpellTargetPosition {
                x: i32::try_from(*x)?,
                y: i32::try_from(*y)?,
                floor: i16::try_from(*floor)?,
            },
            result: std::fs::read(root.join("c18b-result.bin"))?,
            vitals: std::fs::read(root.join("c18b-vitals.bin"))?,
            expected: ActorVitals {
                health: *health,
                max_health: *max_health,
                mana: *mana,
                max_mana: *max_mana,
                soul: *soul,
                harmony: *harmony,
                serene: *serene != 0,
            },
        });
    }
    let expected = ActorVitals {
        health: 150,
        max_health: 150,
        mana: 30,
        max_mana: 55,
        soul: 99,
        harmony: 2,
        serene: false,
    };
    Ok(Fixture {
        spell: NonZeroU32::new(4).ok_or("C18b spell index is zero")?,
        position: SpellTargetPosition {
            x: 32_370,
            y: 32_241,
            floor: 7,
        },
        result: actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
        vitals: actor_spell::encode_actor_vitals(&expected)
            .map_err(|error| format!("C18b inline vitals: {error:?}"))?,
        expected,
    })
}

async fn replay_server(
    mut stream: DuplexStream,
    fixture: Fixture,
    wire_intent: SpellCastIntent,
) -> Result<(), BoxError> {
    send_vitals_join(&mut stream).await?;
    // 1. The position cast: the intent on the wire is exactly the crosshair target.
    let command = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&command)?.client_command(1)?;
    assert_eq!(
        actor_spell::decode_spell_cast_intent(command.payload),
        Ok(wire_intent)
    );
    write_frame(
        &mut stream,
        &encode_command_result(1, 41, 7, CommandStatus::Accepted, &fixture.result)?,
    )
    .await?;
    write_frame(
        &mut stream,
        &encode_state_delta(
            1,
            42,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &fixture.vitals,
        )?,
    )
    .await?;
    // 2. A second cast is refused by the server: no vitals delta follows.
    read_frame(&mut stream).await?;
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            43,
            8,
            CommandStatus::Rejected,
            &actor_spell::encode_spell_cast_result(SpellCastDisposition::Rejected),
        )?,
    )
    .await?;
    // 3. A cast whose vitals delta is corrupt: the client errors and keeps its vitals.
    read_frame(&mut stream).await?;
    write_frame(
        &mut stream,
        &encode_command_result(1, 44, 9, CommandStatus::Accepted, &fixture.result)?,
    )
    .await?;
    write_frame(
        &mut stream,
        &encode_state_delta(
            1,
            45,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            4,
            5,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &[0x38, 0x02],
        )?,
    )
    .await?;
    Ok(())
}

#[test]
fn e2e_c18b_position_cast_runs_through_session_cast_spell() -> Result<(), BoxError> {
    block_on(async {
        let fixture = fixture()?;
        let expected = fixture.expected;
        let (spell, position) = (fixture.spell, fixture.position);
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(replay_server(
            server,
            fixture,
            SpellCastIntent {
                spell,
                target: SpellTarget::Position(position),
                aim_at_target: false,
            },
        ));
        let mut session = Session::admit(client, admission()?).await?;
        let initial = *session.actor_vitals().ok_or("C18b initial vitals absent")?;
        assert_ne!(initial, expected);

        let cast = session
            .cast_spell(spell, SpellTarget::Position(position), false)
            .await?;
        assert_eq!(cast.disposition, SpellCastDisposition::Cast);
        assert_eq!(cast.status, CommandStatus::Accepted);
        assert_eq!(
            cast.actor_vitals_delta.as_ref().map(|delta| delta.value),
            Some(expected)
        );
        assert_eq!(session.actor_vitals(), Some(&expected));

        // A refused cast changes no vitals; a malformed vitals payload is rejected by the codec.
        let refused = session
            .cast_spell(spell, SpellTarget::Position(position), false)
            .await?;
        assert_eq!(refused.disposition, SpellCastDisposition::Rejected);
        assert!(refused.actor_vitals_delta.is_none());
        assert_eq!(session.actor_vitals(), Some(&expected));
        assert!(
            session
                .cast_spell(spell, SpellTarget::Position(position), false)
                .await
                .is_err()
        );
        server.await??;
        assert_eq!(session.actor_vitals(), Some(&expected));
        Ok(())
    })?
}
