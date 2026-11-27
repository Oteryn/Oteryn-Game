//! Real-holder contention and Foundation USE assertions shared by the PostgreSQL shards.
use super::fixture::debug;
use super::TestResult;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::{self as wire, world_object};
use std::{cell::Cell, future::Future, sync::Arc, time::Duration};
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub(super) fn holder_timeout_delay() -> Duration {
    crate::gameplay_transport::COMMAND_HOLDER_WINDOW + Duration::from_millis(200)
}

pub(super) async fn contend_use<T>(
    root: &crate::durability::DurabilityRoot,
    purpose: &'static str,
    delay: Duration,
    serve: impl Future<Output = T>,
) -> TestResult<T> {
    use crate::gameplay_transport::{
        USE_HOLDER_CHECKOUT_PROBE, composed_admission::HolderCheckoutProbe,
    };

    let probe = Arc::new(HolderCheckoutProbe::default());
    let contention = probe.contend_with(root, async move { tokio::time::sleep(delay).await });
    let serving = USE_HOLDER_CHECKOUT_PROBE.scope(
        (purpose, Cell::new(Some(Arc::clone(&probe)))),
        crate::durability::DurabilityRoot::with_holder_owner(serve),
    );
    let (contended, result) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(contention, serving)
    })
    .await?;
    contended.map_err(debug)?;
    Ok(result)
}

pub(super) async fn send_use(
    client: &mut (impl AsyncWrite + Unpin),
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
) -> TestResult {
    let frame = wire::encode_client_command(
        admitted.continuity.connection_generation,
        &wire::ClientCommandValue {
            command_id,
            command_type: world_object::COMMAND_TYPE_USE_INTENT,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

pub(super) fn assert_holder_refusal(
    envelope: &wire::WireEnvelopeView<'_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> TestResult {
    let result = wire::decode_command_result(envelope.payload()).map_err(debug)?;
    assert_eq!(result.command_id, command_id);
    assert_eq!(result.status, wire::CommandStatus::Rejected);
    let error = wire::FoundationProtocolError::CommandRefused;
    assert_eq!(result.error_code, error.code());
    let payloadless = wire::encode_command_error_result(
        admitted.continuity.connection_generation,
        envelope.server_sequence(),
        command_id,
        error,
    )
    .map_err(debug)?;
    let payloadless = crate::foundation::decode_wire_envelope(&payloadless).map_err(debug)?;
    assert_eq!(envelope.payload(), payloadless.payload(), "field 5 must be absent");
    Ok(())
}
