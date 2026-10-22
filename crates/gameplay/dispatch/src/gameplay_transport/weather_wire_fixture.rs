//! Test-only access to the existing production-connection fixture; no second harness.

use super::TestResult;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, IDLE_LIVENESS, serve_admitted};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

pub(crate) async fn serve_input(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    input: &[u8],
) -> TestResult<(String, Vec<u8>)> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    client.write_all(input).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok((format!("{end:?}"), output))
}
