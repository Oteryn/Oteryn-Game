use super::*;

/// Whole connection lifecycle without resource policy; the listener runs the
/// same phases under its budgets.
#[cfg(test)]
pub async fn serve_connection<S, A, I>(
    stream: &mut S,
    authority: &A,
    identifiers: &I,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
{
    let Ok(frame) = read_frame(stream).await else {
        return ConnectionEnd::TransportFailed;
    };
    match admit_frame(stream, &frame, authority, identifiers).await {
        Ok(admitted) => hold_admitted(stream, admitted).await,
        Err(end) => end,
    }
}
