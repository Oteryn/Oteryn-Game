use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::*;

use crate::foundation::{MessageType, decode_wire_envelope};

use crate::gameplay_transport::ComposedFreshAdmission;

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS,
    serve_admitted,
};

fn frame(command_id: u64, command_type: u32, payload: &[u8]) -> TestResult<Vec<u8>> {
    let bytes = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    Ok([u32::try_from(bytes.len())?.to_be_bytes().as_slice(), &bytes].concat())
}

/// SESSION-INGRESS-1: each connection continues this GameSession's command stream, so the
/// commands are numbered from `first` and the ended continuity carries into the next one.
pub(super) async fn serve_from(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    first: u64,
    commands: &[(u32, &[u8])],
) -> TestResult<(String, Vec<u8>)> {
    let mut input = Vec::new();
    for (offset, (kind, payload)) in (0..).zip(commands) {
        input.extend(frame(first + offset, *kind, payload)?);
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&input).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    Ok((format!("{end:?}"), output))
}

pub(super) async fn serve(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u32, &[u8])],
) -> TestResult<(String, Vec<u8>)> {
    let first = admitted.continuity.next_command_id;
    serve_from(owner, admitted, first, commands).await
}

pub(super) fn results(
    output: &[u8],
) -> TestResult<Vec<(u64, crate::foundation::CommandStatus, Vec<u8>)>> {
    let mut cursor = 0;
    let mut values = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        cursor += length + 4;
        if envelope.message_type() == MessageType::CommandResult {
            let value =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            values.push((value.command_id, value.status, value.payload.to_vec()));
        }
    }
    Ok(values)
}

pub(super) async fn epoch(h: &Harness, value: u64) -> TestResult {
    // World owner fixture: not a daily runtime clock or a client-supplied epoch.
    if value == 1 {
        sqlx::query("INSERT INTO game_world_reset_epochs VALUES(encode($1,'hex')::uuid,0,$2,'daily-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-10-01 08:00:00Z','2026-10')")
            .bind(id(42).as_slice()).bind([8_u8;32].as_slice()).execute(&h.pool).await?;
        let epoch:String=sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_history WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(epoch, "1");
        return Ok(());
    }
    sqlx::query("INSERT INTO game_world_reset_history SELECT encode($1,'hex')::uuid,$2::text::numeric,((DATE '2026-10-01'+($2::text::integer-1))-DATE '1970-01-01')::bigint,extract(epoch FROM ((DATE '2026-10-01'+($2::text::integer-1))::timestamp + interval '6 hours') AT TIME ZONE 'Europe/Berlin')::bigint*1000,extract(epoch FROM ((DATE '2026-10-01'+($2::text::integer-1))::timestamp + interval '6 hours') AT TIME ZONE 'Europe/Berlin')::bigint*1000")
        .bind(id(42).as_slice()).bind(value.to_string()).execute(&h.pool).await?;
    Ok(())
}
