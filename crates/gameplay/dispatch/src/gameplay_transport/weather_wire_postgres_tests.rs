//! WEATHER-1 (D1025/D1026/D1027/D1448) end to end on the production authority: the real
//! composed owner exposes the shipped weather source, offers capability 26 on the connection's
//! own offered set, and `serve_admitted` (the production serve path) writes the domain 22
//! snapshot of the actor's area to a client that selected 6 and 26. A client that did not select
//! 26 gets no weather frame. The session's actor stands on the owner's runtime cell.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.

use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::foundation::{ExactActorRef, MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities, with_weather_offer,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::world_weather::shipped;
use oteryn_protocol_oteryn::decode_snapshot_chunk;
use oteryn_protocol_oteryn::world_weather::{
    CAPABILITY_WORLD_WEATHER_V1, SNAPSHOT_TYPE_WORLD_WEATHER_V1, STATE_DOMAIN_WORLD_WEATHER,
    WorldWeather, decode_world_weather,
};

/// Every domain 22 snapshot value in the frames the server wrote.
fn weather_snapshots(output: &[u8]) -> TestResult<Vec<WorldWeather>> {
    let mut values = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let frame = &output[cursor + 4..cursor + 4 + length];
        cursor += 4 + length;
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_WORLD_WEATHER {
                    assert_eq!(domain.snapshot_type, SNAPSHOT_TYPE_WORLD_WEATHER_V1);
                    values.push(decode_world_weather(domain.payload).map_err(debug)?);
                }
            }
        }
    }
    Ok(values)
}

/// `admitted` served by the production `serve_admitted` on the composed owner until the empty
/// client closes: the frames written.
async fn serve(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<(String, Vec<u8>)> {
    fixture_access::serve_input(owner, admitted, &[]).await
}

fn now_ms() -> TestResult<u64> {
    crate::world_clock::now_utc_ms().ok_or_else(|| "the UTC clock reads".into())
}

/// The production serve future on the composed owner is large in a debug build: the test runs
/// on a thread with a stack sized for it, as the default 2 MiB test thread overflows.
#[test]
fn the_composed_server_offers_26_and_serves_the_actors_domain_22_snapshot() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| composed_server_serves_weather().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "the weather e2e thread panicked")??;
    Ok(())
}

fn composed_server_serves_weather() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "weather_wire", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        track(&owner, &dependencies)?;
        let actor: ExactActorRef = dependencies.actor;

        // The production authority exposes the shipped source, so the connection's own offered
        // set (what `admit_frame` negotiates against) carries capability 26.
        let source = owner
            .world_weather()
            .ok_or("the shipped source is exposed")?;
        assert!(shipped().is_some());
        let offered = with_weather_offer(owner.offered_capabilities(), true);
        assert!(
            offered
                .iter()
                .any(|capability| capability.id == CAPABILITY_WORLD_WEATHER_V1)
        );
        assert!(
            PRODUCTION_OFFERED_CAPABILITIES
                .iter()
                .any(|capability| capability.id == CAPABILITY_WORLD_WEATHER_V1)
        );

        // A client that selected 6 and 26 gets the domain 22 snapshot of its actor's area.
        let position = owner
            .observe(actor)
            .await
            .ok_or("the actor is observable")?
            .actor_position;
        let mut weather_session = dependencies.admitted;
        weather_session.first_entry = FirstEntryOutcome::Positioned;
        weather_session.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[6, CAPABILITY_WORLD_WEATHER_V1])
                .ok_or("6 and 26 are selectable together")?;
        let before = now_ms()?;
        let (end, output) = serve(&owner, weather_session).await?;
        let after = now_ms()?;
        let snapshots = weather_snapshots(&output)?;
        let [served] = snapshots.as_slice() else {
            return Err(format!(
                "expected one domain 22 snapshot, got {snapshots:?}; end {end}; {} bytes",
                output.len()
            )
            .into());
        };
        // The wall clock may cross a slot boundary while serving: either side's value is right.
        let expected = [
            source.observe(position, before),
            source.observe(position, after),
        ];
        assert!(expected.contains(served), "{served:?} not in {expected:?}");
        assert!(matches!(served, WorldWeather::Active { .. }));

        // A client that did not select 26 never receives weather.
        let mut plain_session = dependencies.admitted;
        plain_session.first_entry = FirstEntryOutcome::Positioned;
        plain_session.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[6]).ok_or("6 is selectable")?;
        let (_, output) = serve(&owner, plain_session).await?;
        assert!(weather_snapshots(&output)?.is_empty());

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[path = "weather_wire_fixture.rs"]
pub(super) mod fixture_access;
