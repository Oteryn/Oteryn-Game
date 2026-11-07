use super::*;
use crate::world_weather::{WEATHER_SLOT_MS, shipped};

use oteryn_protocol_oteryn::world_weather::{
    CAPABILITY_WORLD_WEATHER_V1, DELTA_TYPE_WORLD_WEATHER_V1, SNAPSHOT_TYPE_WORLD_WEATHER_V1,
    STATE_DOMAIN_WORLD_WEATHER, WorldWeather, decode_world_weather,
};
use oteryn_protocol_oteryn::{decode_snapshot_chunk, decode_state_delta};
use std::task::Poll;
use std::time::Duration;

const THAIS: ActorPosition = ActorPosition {
    x: 32369,
    y: 32241,
    floor: 7,
};
const SVARGROND: ActorPosition = ActorPosition {
    x: 32257,
    y: 31159,
    floor: 7,
};

/// The shipped source's observation for a position at the start of an absolute slot.
fn expected(position: ActorPosition, slot: u64) -> Result<WorldWeather, Box<dyn Error>> {
    Ok(shipped()
        .ok_or("the shipped weather data loads")?
        .observe(position, slot * WEATHER_SLOT_MS))
}

fn with_position(position: ActorPosition, slot: u64) -> NegotiatingAuthority {
    let authority = NegotiatingAuthority::new(None);
    authority.position.set(position);
    authority.clock_ms.set(slot * WEATHER_SLOT_MS + 17);
    authority
}

/// Domain 22 in the frames: the snapshot entries `(revision, value)` and the deltas
/// `(base, new, value)`; no other domain may carry weather.
#[allow(clippy::type_complexity)]
fn weather_of(
    frames: &[Vec<u8>],
) -> Result<(Vec<(u64, WorldWeather)>, Vec<(u64, u64, WorldWeather)>), Box<dyn Error>> {
    let (mut snapshots, mut deltas) = (Vec::new(), Vec::new());
    for frame in frames {
        let envelope = decode_wire_envelope(frame)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                let (_, domains) = decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_WORLD_WEATHER {
                        assert_eq!(domain.snapshot_type, SNAPSHOT_TYPE_WORLD_WEATHER_V1);
                        snapshots.push((
                            domain.revision,
                            decode_world_weather(domain.payload)
                                .map_err(|e| format!("{e:?}"))?,
                        ));
                    }
                }
            }
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload())?;
                if delta.domain_id == STATE_DOMAIN_WORLD_WEATHER {
                    assert_eq!(delta.delta_type, DELTA_TYPE_WORLD_WEATHER_V1);
                    deltas.push((
                        delta.base_revision,
                        delta.new_revision,
                        decode_world_weather(delta.payload).map_err(|e| format!("{e:?}"))?,
                    ));
                }
            }
            _ => {}
        }
    }
    Ok((snapshots, deltas))
}

#[test]
fn production_offers_capability_26_with_the_shipped_data_and_fails_closed_without_it()
-> Result<(), Box<dyn Error>> {
    // D1027 signed off: the static sets list 26 (registry `offered: true`); a connection
    // keeps it only while its authority exposes a source.
    for set in [
        PRODUCTION_OFFERED_CAPABILITIES,
        ITEM_VIEW_OFFERED_CAPABILITIES,
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
    ] {
        assert!(
            set.iter()
                .any(|capability| capability.id == CAPABILITY_WORLD_WEATHER_V1)
        );
        assert_eq!(with_weather_offer(set, true), set);
        assert!(
            with_weather_offer(set, false)
                .iter()
                .all(|capability| capability.id != CAPABILITY_WORLD_WEATHER_V1)
        );
        assert_eq!(with_weather_offer(set, true).len(), set.len());
    }
    assert!(shipped().is_some());
    run(async {
        // The production default authority exposes the shipped data, so 26 is offered
        // (requiring 6); a client that does not ask for it never gets it.
        let live = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(
            &live,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                26, // WORLD_WEATHER_V1
            ])?,
        )
        .await?;
        assert!(admitted.is_ok());
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            26, // WORLD_WEATHER_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let (admitted, frames) = admit(&live, &bootstrap(&[6])?).await?;
        assert_eq!(accepted_selection(&frames)?, [6]);
        let (_, frames) = serve(&live, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
        assert!(weather_of(&frames)?.0.is_empty());
        // Missing or invalid data (no source) fails closed: not offered, so 6 alone.
        let closed = NegotiatingAuthority::new(None);
        closed.weather_bound.set(false);
        let (admitted, frames) = admit(
            &closed,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                26, // WORLD_WEATHER_V1
            ])?,
        )
        .await?;
        assert!(admitted.is_ok());
        assert_eq!(accepted_selection(&frames)?, [6]);
        let (_, frames) = serve(&closed, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
        assert!(weather_of(&frames)?.0.is_empty());
        Ok(())
    })
}

#[test]
fn admission_sends_the_domain_22_snapshot_of_the_actors_area_and_never_rerolls()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = with_position(THAIS, 2);
        let (admitted, _) = admit(
            &authority,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                26, // WORLD_WEATHER_V1
            ])?,
        )
        .await?;
        let mut admitted = admitted.map_err(|e| format!("{e:?}"))?;
        assert_eq!(admitted.continuity.world_weather_revision, 0);
        admitted.continuity.world_weather_revision = 5;
        let (end, frames) = serve(&authority, admitted, &[]).await?;
        let (snapshots, deltas) = weather_of(&frames)?;
        assert!(deltas.is_empty());
        let first = expected(THAIS, 2)?;
        assert_eq!(snapshots, [(5, first)]);
        assert!(matches!(first, WorldWeather::Active { .. }));
        // A resume or transfer never winds the revision back.
        let ConnectionEnd::AdmittedThenDisconnected(served) = end else {
            return Err(format!("{end:?}").into());
        };
        assert_eq!(served.continuity.world_weather_revision, 5);
        // Reconnect (later in the same slot) shows the same value: no reroll.
        let again = with_position(THAIS, 2);
        again.clock_ms.set(2 * WEATHER_SLOT_MS + 899_000);
        let (admitted, _) = admit(
            &again,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                26, // WORLD_WEATHER_V1
            ])?,
        )
        .await?;
        let (_, frames) = serve(&again, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
        assert_eq!(weather_of(&frames)?.0, [(0, first)]);
        // Each area draws independently: a far cold city differs in some slot.
        let mut differs = None;
        for slot in 0..200 {
            if expected(THAIS, slot)? != expected(SVARGROND, slot)? {
                differs = Some(slot);
                break;
            }
        }
        let slot = differs.ok_or("two areas never differ")?;
        let cold = with_position(SVARGROND, slot);
        let (admitted, _) = admit(
            &cold,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                26, // WORLD_WEATHER_V1
            ])?,
        )
        .await?;
        let (_, frames) = serve(&cold, admitted.map_err(|e| format!("{e:?}"))?, &[]).await?;
        assert_eq!(weather_of(&frames)?.0, [(0, expected(SVARGROND, slot)?)]);
        Ok(())
    })
}

#[test]
fn a_new_period_a_lost_source_a_returned_source_and_an_area_change_each_send_one_full_replacement()
-> Result<(), Box<dyn Error>> {
    let first_slot = 2_u64;
    let first = expected(THAIS, first_slot)?;
    // A later slot inside a different period, then a slot where the two areas differ.
    let mut next_slot = first_slot;
    while expected(THAIS, next_slot)? == first {
        next_slot += 1;
    }
    let mut return_slot = next_slot + 20;
    while expected(THAIS, return_slot)? == expected(SVARGROND, return_slot)? {
        return_slot += 1;
    }
    run(async {
        let authority = with_position(THAIS, first_slot);
        let (admitted, _) = admit(
            &authority,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                26, // WORLD_WEATHER_V1
            ])?,
        )
        .await?;
        let mut admitted = admitted.map_err(|e| format!("{e:?}"))?;
        admitted.continuity.world_weather_revision = 5;
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
        let driving = async {
            let beat = Duration::from_millis(400);
            // Within the same period nothing is sent.
            tokio::time::sleep(beat).await;
            authority.clock_ms.set(next_slot * WEATHER_SLOT_MS + 1);
            tokio::time::sleep(beat).await;
            authority.weather_bound.set(false);
            tokio::time::sleep(beat).await;
            // Still unavailable in a later slot: nothing more (one Unavailable only).
            authority.clock_ms.set(return_slot * WEATHER_SLOT_MS);
            tokio::time::sleep(beat).await;
            authority.weather_bound.set(true);
            tokio::time::sleep(beat).await;
            // The player walks into another area: one more full replacement.
            authority.position.set(SVARGROND);
            tokio::time::sleep(beat).await;
            client.shutdown().await?;
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            Ok::<_, std::io::Error>(output)
        };
        let serving = async {
            let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let (mut serving, mut driving) = (std::pin::pin!(serving), std::pin::pin!(driving));
        let (mut end, mut output) = (None, None);
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if output.is_none()
                && let Poll::Ready(value) = driving.as_mut().poll(context)
            {
                output = Some(value);
            }
            if end.is_some() && output.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        let ConnectionEnd::AdmittedThenDisconnected(served) = end.ok_or("end")? else {
            return Err("the connection did not end as a disconnect".into());
        };
        let frames = split(&output.ok_or("output")??)?;
        let (snapshots, deltas) = weather_of(&frames)?;
        assert_eq!(snapshots, [(5, first)]);
        assert_eq!(
            deltas,
            [
                (5, 6, expected(THAIS, next_slot)?),
                (6, 7, WorldWeather::Unavailable),
                (7, 8, expected(THAIS, return_slot)?),
                (8, 9, expected(SVARGROND, return_slot)?),
            ]
        );
        assert_eq!(served.continuity.world_weather_revision, 9);
        // Weather rides its own domain: exactly the four replacements above, no more.
        let mut weather_deltas = 0;
        for frame in &frames {
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::StateDelta
                && decode_state_delta(envelope.payload())?.domain_id
                    == STATE_DOMAIN_WORLD_WEATHER
            {
                weather_deltas += 1;
            }
        }
        assert_eq!(weather_deltas, 4);
        Ok(())
    })
}
