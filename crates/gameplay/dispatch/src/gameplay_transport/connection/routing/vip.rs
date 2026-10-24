//! Vip command body; predicate and connection control flow stay in `connection.rs`.

use super::super::super as transport;
use super::super::{AdmittedSession, FreshAdmissionAuthority};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    Dispatch::Vip(
        transport::vip::dispatch(
            authority,
            *admitted,
            command.command_id,
            command.command_type,
            command.payload,
        )
        .await,
    )
}
