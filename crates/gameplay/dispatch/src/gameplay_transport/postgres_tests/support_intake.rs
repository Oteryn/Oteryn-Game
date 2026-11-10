//! PARITY-REPORT-1: admitted connection bytes, composed owner and the fenced report queue.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::support_intake::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                assert!(r.payload.len() <= MAX_REPLY_BYTES);
                break (r.status, r.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("report command {id} ended {ended:?}: {error}").into())
}

#[test]
fn support_intake_postgres_real_dispatch_consent_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "support_intake", true, false).await.map_err(debug)?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(admitted.game_session_id).await?.0;
        admitted.item_fence = item_fence_of(current);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let payload = encode_submission("Reference: 1\nBuild: test\nSource: client.rs:42", true).map_err(debug)?;
        // No selected capability: real dispatch refuses and leaves no evidence.
        let (status, _) = exchange(&owner, &mut admitted, 1, CRASH, &payload).await?;
        assert_eq!(status, CommandStatus::Rejected);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_SUPPORT_INTAKE_V1]).ok_or("selection")?;
        for (id, kind, text) in [(2, CRASH, "Reference: 1\nBuild: test\nSource: client.rs:42"), (4, SURVEY, "Reference: 2\nAnswer: Easy")] {
            let payload = encode_submission(text, true).map_err(debug)?;
            let (status, reply) = exchange(&owner, &mut admitted, id, kind, &payload).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(reply, [1]);
            let (status, replay) = exchange(&owner, &mut admitted, id, kind, &payload).await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            assert_eq!(replay, [1]);
            // Cross-session/command retry uses the displayed reference and exact bytes.
            assert_eq!(exchange(&owner, &mut admitted, id+1, kind, &payload).await?.1, [1]);
        }
        assert!(encode_submission("declined", false).is_err());
        let (status, _) = exchange(&owner, &mut admitted, 6, CRASH, b"\0declined").await?;
        assert_eq!(status, CommandStatus::Rejected);
        let mut stale = admitted;
        if let Some(ref mut fence) = stale.item_fence { fence.character_lease_generation += 1; }
        let stale_payload = encode_submission("Reference: 3", true).map_err(debug)?;
        assert!(matches!(owner.support_intake_command(stale, 7, SURVEY, &stale_payload).await,
            crate::gameplay_transport::support_intake::SupportIntakeOutcome::Rejected));
        // Transport also fails closed: its result writer refuses a stale lease.
        assert!(exchange(&owner, &mut stale, 7, SURVEY, &stale_payload).await.is_err());
        let rows: Vec<(i32,String)> = sqlx::query_as("SELECT kind, preview FROM game_support_intake ORDER BY kind").fetch_all(&h.pool).await?;
        assert_eq!(rows, vec![(CRASH as i32, "Reference: 1\nBuild: test\nSource: client.rs:42".into()),(SURVEY as i32,"Reference: 2\nAnswer: Easy".into())]);
        let columns: Vec<String> = sqlx::query_scalar("SELECT column_name::text FROM information_schema.columns WHERE table_name='game_support_intake' ORDER BY column_name").fetch_all(&h.pool).await?;
        assert_eq!(columns, ["kind", "preview", "receipt"]);
        let can_modify: bool = sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime','game_support_intake','UPDATE') OR has_table_privilege('oteryn_game_runtime','game_support_intake','DELETE')").fetch_one(&h.pool).await?;
        assert!(!can_modify);
        drop(owner); drop(authority); drop(seal);
        h.cleanup().await
    }).map_err(|e|e.to_string()))?.join().map_err(|_|"support intake panicked")??;
    Ok(())
}
