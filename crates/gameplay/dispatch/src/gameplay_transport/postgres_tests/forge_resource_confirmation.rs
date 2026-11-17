//! Additive Resource quote variants exercise actual6116 dispatch against the existing fenced fixture.
use super::{
    TestResult,
    item_forge::support::{self, Case, send_command},
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use oteryn_protocol_oteryn::{CommandStatus, forge_preview as wire, item_forge::Operation};
#[test]
fn forge_resource_confirmation_postgres_quotes_are_real_and_readonly() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("forge-resource-confirmation", || {
        support::scenario(Case::Preview)
    })
}
pub(super) async fn read_quotes(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command: &mut u64,
) -> TestResult {
    for (nonce, operation) in [
        (11, Operation::DustToSlivers),
        (12, Operation::DustLimit),
        (13, Operation::SliversToCores),
    ] {
        let request = wire::Request {
            nonce,
            selection: wire::Selection::Resource(operation),
        }
        .encode()
        .ok_or("resource quote")?;
        let (status, bytes) = send_command(
            client,
            generation,
            *command,
            wire::COMMAND_TYPE_FORGE_PREVIEW,
            &request,
        )
        .await?;
        *command = command.checked_add(1).ok_or("resource command overflow")?;
        assert_eq!(status, CommandStatus::Accepted);
        let reply = wire::Reply::decode(&bytes).ok_or("resource reply")?;
        assert_eq!(reply.nonce, nonce);
        if operation == Operation::SliversToCores {
            assert_eq!(
                reply.payload,
                wire::Payload::Refused,
                "17 slivers do not satisfy the exact50-sliver conversion"
            );
            continue;
        }
        let wire::Payload::Resource(q) = reply.payload else {
            return Err("resource terms".into());
        };
        assert_eq!(q.operation, operation);
        assert_eq!((q.before.dust, q.before.dust_limit), (1000, 1000));
        if q.full_resources {
            assert_eq!((q.before.slivers, q.before.cores), (17, 2));
        } else {
            assert_eq!(operation, Operation::DustLimit);
        }
        if operation == Operation::DustToSlivers {
            assert_eq!(
                (q.after_dust, q.after_limit, q.after_slivers, q.after_cores),
                (940, 1000, 20, 2)
            );
        } else {
            assert_eq!(
                (q.after_dust, q.after_limit, q.after_slivers, q.after_cores),
                (930, 1020, 0, 0)
            );
        }
    }
    eprintln!(
        "FORGE_RESOURCE_CONFIRMATION POSTGRES_QUOTE_PROOF exact_conversions_and_next_limit_readonly"
    );
    Ok(())
}

/// A limit review does not require a SessionItemView; run after the live stream is drained.
pub(super) async fn read_without_handles(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let without_handles = owner
        .forge_preview(
            *admitted,
            admitted.continuity.next_command_id,
            wire::Request {
                nonce: 14,
                selection: wire::Selection::Resource(Operation::DustLimit),
            },
            None,
        )
        .await;
    let crate::gameplay_transport::forge_preview::Outcome::Payload(bytes) = without_handles else {
        return Err("limit review must not require inventory handles".into());
    };
    assert!(matches!(wire::Reply::decode(&bytes), Some(wire::Reply {
        nonce: 14, payload: wire::Payload::Resource(q),
    }) if !q.full_resources && q.after_limit == 1020));
    Ok(())
}
