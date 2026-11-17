//! C12 client replay. The spell-equipment augment is resolved by the server and uses the
//! existing spell result and actor-vitals payloads. The owning client seam is
//! `crates/session/src/session_exchange.rs::Session::exchange_cast`, which decodes the result,
//! reads the claimed domain-3 delta, and routes it through
//! `crates/session/src/session_exchange.rs::Session::read_delta`; the visible value is exposed
//! by `crates/session/src/session_access.rs::Session::actor_vitals`.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{
    self, ActorVitals, SpellCastDisposition, SpellCastIntent, SpellTarget,
};
use std::num::NonZeroU32;

struct Fixture {
    result: Vec<u8>,
    vitals: Vec<u8>,
    expected: ActorVitals,
}

fn fixture() -> Result<Fixture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("c12-expected.txt"))?;
        let values = expected
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        let [health, max_health, mana, max_mana, soul, harmony, serene] = values.as_slice() else {
            return Err("invalid C12 expected actor-vitals values".into());
        };
        return Ok(Fixture {
            result: std::fs::read(root.join("c12-cast-result.bin"))?,
            vitals: std::fs::read(root.join("c12-actor-vitals.bin"))?,
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
        health: 145,
        max_health: 150,
        mana: 31,
        max_mana: 55,
        soul: 99,
        harmony: 0,
        serene: false,
    };
    Ok(Fixture {
        result: actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
        vitals: actor_spell::encode_actor_vitals(&expected)
            .map_err(|error| format!("C12 inline vitals: {error:?}"))?,
        expected,
    })
}

async fn replay_server(mut stream: DuplexStream, fixture: Fixture) -> Result<(), BoxError> {
    send_vitals_join(&mut stream).await?;
    let command = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&command)?.client_command(1)?;
    assert_eq!(
        actor_spell::decode_spell_cast_intent(command.payload),
        Ok(SpellCastIntent {
            spell: NonZeroU32::new(2).ok_or("C12 spell id is zero")?,
            target: SpellTarget::None,
            aim_at_target: false,
        })
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
    Ok(())
}

#[test]
fn e2e_c12_replays_augmented_cast_payload_and_refuses_invalid_payload() -> Result<(), BoxError> {
    block_on(async {
        let fixture = fixture()?;
        let expected = fixture.expected;
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(replay_server(server, fixture));
        let mut session = Session::admit(client, admission()?).await?;
        let outcome = session
            .cast_spell(
                NonZeroU32::new(2).ok_or("C12 spell id is zero")?,
                SpellTarget::None,
                false,
            )
            .await?;
        server.await??;
        assert_eq!(outcome.disposition, SpellCastDisposition::Cast);
        assert_eq!(session.actor_vitals(), Some(&expected));

        let unchanged = *session.actor_vitals().ok_or("C12 actor vitals absent")?;
        assert!(actor_spell::decode_actor_vitals(&[0x38, 0x02]).is_err());
        assert_eq!(session.actor_vitals(), Some(&unchanged));
        Ok(())
    })?
}
