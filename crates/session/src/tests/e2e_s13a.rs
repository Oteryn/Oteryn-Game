//! S13a client replay: the party gate. A party spell cast while solo and one cast during the
//! cooldown are answered with the server-observed refusals (`s13a-expected.txt`, encoded by the
//! production `encode_spell_cast_result`; `Session::cast_spell` decodes the result and does not
//! retain the raw bytes). `Session::exchange_cast` (`session_exchange.rs`) must report the refusal
//! with no `ACTOR_VITALS` delta, and `Session::actor_vitals` must stay at the joined values. A
//! malformed result is an error that leaves the vitals unchanged.
use super::*;
use oteryn_protocol_oteryn::actor_spell::{
    self, ActorVitals, SpellCastDisposition, SpellCastIntent, SpellTarget,
};
use std::num::NonZeroU32;

type Refusal = (CommandStatus, SpellCastDisposition);

const FIXTURE: &str = "solo\tstatus=Accepted\tdisposition=NotAvailable\nrefusal\tstatus=Accepted\tdisposition=CoolingDown\n";

pub(super) const JOINED: ActorVitals = ActorVitals {
    health: 150,
    max_health: 150,
    mana: 55,
    max_mana: 55,
    soul: 100,
    harmony: 0,
    serene: false,
};

fn status(name: &str) -> Result<CommandStatus, BoxError> {
    match name {
        "Accepted" => Ok(CommandStatus::Accepted),
        "Rejected" => Ok(CommandStatus::Rejected),
        other => Err(format!("unknown S13a status {other}").into()),
    }
}

fn disposition(name: &str) -> Result<SpellCastDisposition, BoxError> {
    match name {
        "Cast" => Ok(SpellCastDisposition::Cast),
        "CoolingDown" => Ok(SpellCastDisposition::CoolingDown),
        "LevelTooLow" => Ok(SpellCastDisposition::LevelTooLow),
        "MagicLevelTooLow" => Ok(SpellCastDisposition::MagicLevelTooLow),
        "NotEnoughMana" => Ok(SpellCastDisposition::NotEnoughMana),
        "NotEnoughSoul" => Ok(SpellCastDisposition::NotEnoughSoul),
        "NotAvailable" => Ok(SpellCastDisposition::NotAvailable),
        "TargetRequired" => Ok(SpellCastDisposition::TargetRequired),
        "TargetIllegal" => Ok(SpellCastDisposition::TargetIllegal),
        "Rejected" => Ok(SpellCastDisposition::Rejected),
        other => Err(format!("unknown S13a disposition {other}").into()),
    }
}

fn parse(text: &str) -> Result<Vec<Refusal>, BoxError> {
    text.lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let [_, st, di] = fields.as_slice() else {
                return Err("S13a expected line needs three fields".into());
            };
            Ok((
                status(st.strip_prefix("status=").ok_or("status= missing")?)?,
                disposition(
                    di.strip_prefix("disposition=")
                        .ok_or("disposition= missing")?,
                )?,
            ))
        })
        .collect()
}

pub(super) async fn join_with_vitals(
    stream: &mut DuplexStream,
    vitals: &ActorVitals,
) -> Result<(), BoxError> {
    read_frame(&mut *stream).await?;
    write_frame(
        &mut *stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let overlay = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let payload =
        actor_spell::encode_actor_vitals(vitals).map_err(|error| format!("vitals: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial(0),
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            DomainSnapshot {
                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 3,
                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &payload,
            },
        ],
    )? {
        write_frame(&mut *stream, &frame).await?;
    }
    Ok(())
}

pub(super) fn intent() -> Result<SpellCastIntent, BoxError> {
    Ok(SpellCastIntent {
        spell: NonZeroU32::new(2).ok_or("spell id is zero")?,
        target: SpellTarget::None,
        aim_at_target: false,
    })
}

#[test]
fn e2e_s13a_party_gate_and_cooldown_refusals_replay() -> Result<(), BoxError> {
    let text = match std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        Some(root) => {
            std::fs::read_to_string(std::path::Path::new(&root).join("s13a-expected.txt"))?
        }
        None => FIXTURE.to_owned(),
    };
    let refusals = parse(&text)?;
    let [solo, cooldown] = refusals.as_slice() else {
        return Err("S13a needs the solo and the cooldown refusal".into());
    };
    assert_ne!(solo.1, SpellCastDisposition::Cast);
    assert_ne!(cooldown.1, SpellCastDisposition::Cast);
    let (solo, cooldown) = (*solo, *cooldown);
    block_on(async {
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(async move {
            join_with_vitals(&mut peer, &JOINED).await?;
            for (offset, (st, di)) in [solo, cooldown].into_iter().enumerate() {
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
                        41 + u64::try_from(offset)?,
                        7 + u64::try_from(offset)?,
                        st,
                        &actor_spell::encode_spell_cast_result(di),
                    )?,
                )
                .await?;
            }
            read_frame(&mut peer).await?;
            write_frame(
                &mut peer,
                &encode_command_result(1, 43, 9, CommandStatus::Accepted, &[0xff])?,
            )
            .await?;
            Ok::<_, BoxError>(peer)
        });
        let mut session = Session::admit(client, admission()?).await?;
        assert_eq!(session.actor_vitals(), Some(&JOINED));
        for (offset, want) in [solo, cooldown].into_iter().enumerate() {
            let outcome = session
                .cast_spell(
                    NonZeroU32::new(2).ok_or("spell id is zero")?,
                    SpellTarget::None,
                    false,
                )
                .await?;
            assert_eq!(outcome.command_id, 7 + u64::try_from(offset)?);
            assert_eq!((outcome.status, outcome.disposition), want);
            assert_eq!(outcome.actor_vitals_delta, None);
            assert_eq!(session.actor_vitals(), Some(&JOINED));
        }
        let malformed = session
            .cast_spell(
                NonZeroU32::new(2).ok_or("spell id is zero")?,
                SpellTarget::None,
                false,
            )
            .await;
        assert!(malformed.is_err());
        assert_eq!(session.actor_vitals(), Some(&JOINED));
        server.await??;
        Ok::<_, BoxError>(())
    })?
}
