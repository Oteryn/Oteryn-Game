//! P11 replay owns the session consumer: `Session::account_achievements` in
//! `account_achievements.rs` sends the page query, checks the page number and decodes the
//! account-achievements result that the server returned.
use super::*;
use oteryn_protocol_oteryn::account_achievements::{
    self as wire, AccountAchievementRow, AccountAchievementsQuery, AccountAchievementsResult,
};

struct Captured {
    page: Vec<u8>,
    wrong_page: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Captured, BoxError> {
    let row = |slug: &str, grade: u32, points: u32| AccountAchievementRow {
        key: format!("oteryn:achievement/{slug}"),
        name: slug.replace('_', " "),
        description: "Earned by playing.".into(),
        grade,
        points,
        earned_at: 1_700_000_000,
        secret: false,
    };
    let page = AccountAchievementsResult {
        total_points: 15,
        fact_count: 2,
        page: 0,
        has_more: false,
        rows: vec![row("first_steps", 1, 5), row("rat_hunter", 2, 10)],
    };
    let wrong = AccountAchievementsResult {
        page: 5,
        ..page.clone()
    };
    Ok(Captured {
        page: wire::encode_account_achievements_result(&page).map_err(|e| format!("{e:?}"))?,
        wrong_page: wire::encode_account_achievements_result(&wrong)
            .map_err(|e| format!("{e:?}"))?,
        expected: "page=0\ntotal_points=15\nfact_count=2\nhas_more=false\nrows=oteryn:achievement/first_steps:1:5|oteryn:achievement/rat_hunter:2:10\nmalformed_query=local-codec-refusal\nmalformed_wire_submission=false\n".into(),
    })
}

fn captured() -> Result<Captured, BoxError> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    let page = std::fs::read(path.join("p11-page.bin"))?;
    let mut decoded = wire::decode_account_achievements_result(&page)
        .map_err(|error| format!("captured page: {error:?}"))?;
    decoded.page += 5;
    Ok(Captured {
        wrong_page: wire::encode_account_achievements_result(&decoded)
            .map_err(|error| format!("wrong-page payload: {error:?}"))?,
        page,
        expected: std::fs::read_to_string(path.join("p11-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

async fn peer(
    mut stream: DuplexStream,
    replies: Vec<(CommandStatus, Vec<u8>)>,
) -> Result<(), BoxError> {
    read_frame(&mut stream).await?;
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let spatial_payload = spatial(0);
    let overlay = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial_payload,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    for (index, (status, payload)) in (0_u64..).zip(replies) {
        let bytes = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&bytes)?.client_command(1)?;
        assert_eq!((command.command_id, command.command_type), (7 + index, 10));
        assert_eq!(
            wire::decode_account_achievements_query(command.payload),
            Ok(AccountAchievementsQuery { page: 0 })
        );
        write_frame(
            &mut stream,
            &encode_command_result(1, 41 + index, command.command_id, status, &payload)?,
        )
        .await?;
    }
    Ok(())
}

#[test]
fn e2e_p11_replays_achievement_page_refusal_and_wrong_page() -> Result<(), BoxError> {
    let captured = captured()?;
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            block_on(async {
                let e = captured.expected.as_str();
                // Capture facts cover the typed page and a local codec boundary.
                // Server rejection below is a separate synthetic session regression.
                assert_eq!(value(e, "malformed_query")?, "local-codec-refusal");
                assert_eq!(value(e, "malformed_wire_submission")?, "false");
                assert!(wire::decode_account_achievements_query(&[0x08]).is_err());
                let (client, server) = tokio::io::duplex(65536);
                let task = tokio::spawn(peer(
                    server,
                    vec![
                        (CommandStatus::Accepted, captured.page.clone()),
                        (CommandStatus::Rejected, Vec::new()),
                        (CommandStatus::Accepted, captured.wrong_page.clone()),
                    ],
                ));
                let mut session = Session::admit(client, admission()?).await?;

                let account_achievements::Response::Page(page) =
                    session.account_achievements(0).await?
                else {
                    return Err("accepted page was refused".into());
                };
                assert_eq!(page.page.to_string(), value(e, "page")?);
                assert_eq!(page.total_points.to_string(), value(e, "total_points")?);
                assert_eq!(page.fact_count.to_string(), value(e, "fact_count")?);
                assert_eq!(page.has_more.to_string(), value(e, "has_more")?);
                let rows: Vec<String> = page
                    .rows
                    .iter()
                    .map(|row| format!("{}:{}:{}", row.key, row.grade, row.points))
                    .collect();
                assert_eq!(rows.join("|"), value(e, "rows")?);

                // Independent synthetic check: a server-rejected query is refused without poisoning.
                assert_eq!(
                    session.account_achievements(0).await?,
                    account_achievements::Response::Refused
                );
                // A page answering another page number is a protocol fault that poisons it.
                assert!(matches!(
                    session.account_achievements(0).await,
                    Err(SessionError::Protocol(
                        FoundationProtocolError::MalformedFrame
                    ))
                ));
                assert!(matches!(
                    session.account_achievements(0).await,
                    Err(SessionError::SessionUnusable)
                ));
                task.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "p11 session scenario panicked")?
}
