//! PARITY-REPORT-1: admitted connection bytes, composed owner and the fenced report queue.
use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture as characters;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::player_report::PlayerReportOutcome;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::player_report::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

/// report_no, kind, reported id hex, reported name, comment and reporter x/y/floor.
type ReportRow = (i64, i16, Option<String>, Option<String>, String, i32, i32, i16);

#[test]
fn player_report_postgres_real_wire_queue_replay_refusals_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "player report E2E panicked")??;
    Ok(())
}

fn bug(comment: &str) -> PlayerReportRequest {
    PlayerReportRequest {
        kind: KIND_BUG,
        category: BUG_CATEGORY_MAP,
        reason: 0,
        action: 0,
        target: String::new(),
        comment: comment.into(),
        map_x: 32_000,
        map_y: 31_000,
        map_floor: 7,
    }
}
fn violation(target: &str, reason: u8, action: u8) -> PlayerReportRequest {
    PlayerReportRequest {
        kind: KIND_RULE_VIOLATION,
        category: 0,
        reason,
        action,
        target: target.into(),
        comment: "spam".into(),
        map_x: 0,
        map_y: 0,
        map_floor: 0,
    }
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
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
                    command_type: COMMAND_TYPE_PLAYER_REPORT,
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
async fn report(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    request: &PlayerReportRequest,
    expected: CommandStatus,
) -> TestResult<PlayerReportReply> {
    let bytes = encode_player_report_request(request).map_err(debug)?;
    let (status, reply) = exchange(owner, admitted, id, &bytes).await?;
    assert_eq!(status, expected, "command {id}");
    decode_player_report_reply(&reply).map_err(debug)
}
async fn refill(h: &Harness) -> TestResult {
    sqlx::query("UPDATE game_player_report_rate_buckets SET tokens=3")
        .execute(&h.pool)
        .await?;
    Ok(())
}
async fn direct(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    request: &PlayerReportRequest,
) -> TestResult<PlayerReportOutcome> {
    let bytes = encode_player_report_request(request).map_err(debug)?;
    Ok(owner
        .player_report_command(admitted, id, COMMAND_TYPE_PLAYER_REPORT, &bytes)
        .await)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "player_report_wire", true, false)
            .await
            .map_err(|e| format!("harness: {e:?}"))?;
        prepare_current_session(&h)
            .await
            .map_err(|e| format!("prepare session: {e}"))?;
        let alice = CharacterId::from_bytes(characters::sid(90, 1)).map_err(debug)?;
        characters::historical_character(
            &h.pool,
            90,
            alice,
            characters::sid(90, 2),
            id(42),
            "Alice",
        )
        .await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        admitted.item_fence = item_fence_of(current);
        let unselected = admitted;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let spot = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .borrow_movement_position()
                .read(deps.actor)
                .map_err(debug)?
                .position()
        };
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_PLAYER_REPORT_V1])
                .ok_or("selection")?;

        // A bug report and two rule-violation reports reach the queue through the wire.
        let before_first = admitted;
        let first = report(&owner, &mut admitted, 1, &bug("hole"), CommandStatus::Accepted).await?;
        assert_eq!(first.code, PlayerReportCode::Accepted);
        assert_ne!(first.report, 0);
        let resolved = report(
            &owner,
            &mut admitted,
            2,
            &violation("alice", 1, 0),
            CommandStatus::Accepted,
        )
        .await?;
        assert_eq!(resolved.code, PlayerReportCode::Accepted);
        let unknown = report(
            &owner,
            &mut admitted,
            3,
            &violation("Nobody Here", 2, 1),
            CommandStatus::Accepted,
        )
        .await?;
        assert_eq!(unknown.code, PlayerReportCode::Accepted);
        assert!(first.report < resolved.report && resolved.report < unknown.report);

        // REPORT1-RL-03: the fourth report inside the burst is refused with a typed wait.
        let limited = report(&owner, &mut admitted, 4, &bug("again"), CommandStatus::Accepted).await?;
        assert_eq!(limited.code, PlayerReportCode::RateLimited);
        assert_eq!(limited.report, 0);
        assert!((1..=60).contains(&limited.wait_seconds));
        // SESSION-INGRESS-1 replays the stream's retained result.
        assert_eq!(
            report(&owner, &mut admitted, 4, &bug("again"), CommandStatus::DuplicateReplay).await?,
            limited
        );
        // The native receipt answers an earlier command without a token or a second row, and
        // refuses changed input under the committed id.
        let PlayerReportOutcome::Payload(receipt) =
            direct(&owner, before_first, 1, &bug("hole")).await?
        else {
            return Err("first receipt".into());
        };
        assert_eq!(decode_player_report_reply(&receipt).map_err(debug)?, first);
        let PlayerReportOutcome::Payload(changed) =
            direct(&owner, before_first, 1, &bug("other")).await?
        else {
            return Err("changed receipt".into());
        };
        assert_eq!(
            decode_player_report_reply(&changed).map_err(debug)?.code,
            PlayerReportCode::Invalid
        );

        refill(&h).await?;
        let refused = [
            (5, violation("fixture HERO", 1, 0), PlayerReportCode::SelfReport),
            (6, violation("ALICE", 1, 0), PlayerReportCode::AlreadyPending),
            (
                7,
                PlayerReportRequest {
                    kind: 3,
                    ..bug("unknown kind")
                },
                PlayerReportCode::Invalid,
            ),
        ];
        for (command, request, code) in refused {
            let reply = report(&owner, &mut admitted, command, &request, CommandStatus::Accepted)
                .await?;
            assert_eq!((reply.code, reply.report, reply.wait_seconds), (code, 0, 0));
        }
        refill(&h).await?;
        // Another action against the same target is a separate pending report.
        let other_action = report(
            &owner,
            &mut admitted,
            8,
            &violation("Alice", 1, 2),
            CommandStatus::Accepted,
        )
        .await?;
        assert_eq!(other_action.code, PlayerReportCode::Accepted);

        // A stale lease fence reaches the owner and writes nothing.
        let mut stale = admitted;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .character_lease_generation += 1;
        assert!(matches!(
            direct(&owner, stale, 9, &bug("stale")).await?,
            PlayerReportOutcome::Rejected
        ));
        // Without the selected capability the wire refuses before the owner.
        let mut session = unselected;
        session.continuity = admitted.continuity;
        session.continuity.selected_capabilities = unselected.continuity.selected_capabilities;
        let next = session.continuity.next_command_id;
        let bytes = encode_player_report_request(&bug("no capability")).map_err(debug)?;
        assert_eq!(
            exchange(&owner, &mut session, next, &bytes).await?.0,
            CommandStatus::Rejected
        );

        // The queue holds exactly the accepted reports with server-owned facts only.
        let rows: Vec<ReportRow> =
            sqlx::query_as(
                "SELECT report_no,kind,encode(uuid_send(reported_character_id),'hex'),reported_name,comment,reporter_x,reporter_y,reporter_floor FROM game_player_reports ORDER BY report_no",
            )
            .fetch_all(&h.pool)
            .await?;
        let numbers: Vec<u64> = rows.iter().map(|row| row.0 as u64).collect();
        assert_eq!(
            numbers,
            [first.report, resolved.report, unknown.report, other_action.report]
        );
        let alice_hex: String = alice.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!((rows[0].1, rows[0].2.as_deref(), rows[0].3.as_deref()), (1, None, None));
        assert_eq!(rows[0].4, "hole");
        assert_eq!((rows[1].1, rows[1].2.as_deref(), rows[1].3.as_deref()), (2, Some(alice_hex.as_str()), None));
        assert_eq!(
            (rows[2].2.as_deref(), rows[2].3.as_deref()),
            (None, Some("Nobody Here"))
        );
        for row in &rows {
            assert_eq!(
                (row.5, row.6, row.7),
                (spot.x, spot.y, spot.floor)
            );
        }
        let columns: Vec<String> = sqlx::query_scalar(
            "SELECT column_name::text FROM information_schema.columns WHERE table_name='game_player_reports' ORDER BY ordinal_position",
        )
        .fetch_all(&h.pool)
        .await?;
        assert!(
            columns
                .iter()
                .all(|c| !["email", "ip", "address", "chat", "statement", "translation"]
                    .iter()
                    .any(|banned| c.contains(banned))),
            "{columns:?}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_report_receipts")
                .fetch_one(&h.pool)
                .await?,
            8
        );
        // Append-only evidence and no direct runtime write path.
        assert!(
            sqlx::query("UPDATE game_player_reports SET comment='x'")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM game_player_report_receipts")
                .execute(&h.pool)
                .await
                .is_err()
        );
        let runtime_insert: bool = sqlx::query_scalar(
            "SELECT has_table_privilege('oteryn_game_runtime','game_player_reports','INSERT') OR has_table_privilege('oteryn_game_runtime','game_player_report_rate_buckets','UPDATE')",
        )
        .fetch_one(&h.pool)
        .await?;
        assert!(!runtime_insert);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
