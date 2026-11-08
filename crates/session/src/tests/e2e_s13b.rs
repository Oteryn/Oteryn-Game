//! S13b client replay: the party spell's mana cost. The server-observed vitals before and after
//! the in-party cast (`s13b-expected.txt`, two lines `health max_health mana max_mana soul harmony
//! serene`) are sent through the production `encode_actor_vitals` (the session keeps decoded
//! vitals, not raw bytes). `Session::exchange_cast` (`session_exchange.rs`) must claim the
//! `ACTOR_VITALS` delta of the `Cast` result and `Session::actor_vitals` must show the spent mana;
//! a delta with a stale base revision is refused and leaves the vitals unchanged.
use super::e2e_s13a::{intent, join_with_vitals};
use super::*;
use oteryn_protocol_oteryn::actor_spell::{self, ActorVitals, SpellCastDisposition, SpellTarget};
use std::num::NonZeroU32;

const FIXTURE: &str = "150 150 55 55 100 0 0\n150 150 15 55 100 0 0\n";

fn parse(text: &str) -> Result<Vec<ActorVitals>, BoxError> {
    text.lines()
        .map(|line| {
            let values = line
                .split_whitespace()
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()?;
            let [health, max_health, mana, max_mana, soul, harmony, serene] = values.as_slice()
            else {
                return Err("invalid S13b vitals line".into());
            };
            Ok(ActorVitals {
                health: *health,
                max_health: *max_health,
                mana: *mana,
                max_mana: *max_mana,
                soul: *soul,
                harmony: *harmony,
                serene: *serene != 0,
            })
        })
        .collect()
}

#[test]
fn e2e_s13b_party_spell_mana_cost_replay() -> Result<(), BoxError> {
    let text = match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        Some(root) => {
            std::fs::read_to_string(std::path::Path::new(&root).join("s13b-expected.txt"))?
        }
        None => FIXTURE.to_owned(),
    };
    let vitals = parse(&text)?;
    let [before, after] = vitals.as_slice() else {
        return Err("S13b needs the before and after vitals".into());
    };
    let (before, after) = (*before, *after);
    assert!(after.mana < before.mana);
    let delta = actor_spell::encode_actor_vitals(&after)
        .map_err(|error| format!("S13b vitals: {error:?}"))?;
    block_on(async {
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(async move {
            join_with_vitals(&mut peer, &before).await?;
            let command = read_frame(&mut peer).await?;
            let command = decode_wire_envelope(&command)?.client_command(1)?;
            assert_eq!(
                actor_spell::decode_spell_cast_intent(command.payload),
                Ok(intent()?)
            );
            write_frame(
                &mut peer,
                &encode_command_result(
                    1,
                    41,
                    7,
                    CommandStatus::Accepted,
                    &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
                )?,
            )
            .await?;
            write_frame(
                &mut peer,
                &encode_state_delta(
                    1,
                    42,
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    3,
                    4,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    &delta,
                )?,
            )
            .await?;
            read_frame(&mut peer).await?;
            write_frame(
                &mut peer,
                &encode_command_result(
                    1,
                    43,
                    8,
                    CommandStatus::Accepted,
                    &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
                )?,
            )
            .await?;
            write_frame(
                &mut peer,
                &encode_state_delta(
                    1,
                    44,
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    3,
                    4,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    &delta,
                )?,
            )
            .await?;
            Ok::<_, BoxError>(peer)
        });
        let mut session = Session::admit(client, admission()?).await?;
        assert_eq!(session.actor_vitals(), Some(&before));
        let outcome = session
            .cast_spell(
                NonZeroU32::new(2).ok_or("spell id is zero")?,
                SpellTarget::None,
                false,
            )
            .await?;
        assert_eq!(outcome.disposition, SpellCastDisposition::Cast);
        assert_eq!(outcome.status, CommandStatus::Accepted);
        assert_eq!(session.actor_vitals(), Some(&after));
        // A repeated delta against the already advanced revision is stale and is refused.
        let retained = *session.actor_vitals().ok_or("vitals absent")?;
        let stale = session
            .cast_spell(
                NonZeroU32::new(2).ok_or("spell id is zero")?,
                SpellTarget::None,
                false,
            )
            .await;
        assert!(stale.is_err());
        assert_eq!(session.actor_vitals(), Some(&retained));
        server.await??;
        Ok::<_, BoxError>(())
    })?
}
