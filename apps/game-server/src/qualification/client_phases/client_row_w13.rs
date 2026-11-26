//! W13 client replay capture: the real DAILY_BOOST_QUERY reply (the day's boosted creature, boss
//! and reset epoch) and a malformed query refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0151-daily-boost/protocol.rs`.
use super::*;
use oteryn_dev_client::client_ui::daily_boost::{self, DailyBoostPanel};
use oteryn_protocol_oteryn::daily_boost as wire;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

type Reply = oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;

#[derive(serde::Serialize, serde::Deserialize)]
struct Capture {
    session: String,
    fresh_session: String,
    expected: (String, String, String),
    replies: Vec<(u64, u64, u32, Vec<u8>)>,
}

impl Capture {
    fn provenance(&self) -> String {
        let sequence = self
            .replies
            .iter()
            .map(|(id, seq, _, _)| format!("{id}:{seq}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "session={} fresh_session={} replies={sequence}",
            self.session, self.fresh_session
        )
    }
}

async fn query(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<Reply> {
    Ok(client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            payload,
        )
        .await?)
}

fn require_reply(reply: &Reply, expected: CommandStatus) -> TestResult {
    if reply.status != expected
        || (expected == CommandStatus::Rejected && !reply.payload.is_empty())
    {
        return Err(format!("daily boost expected {expected:?}, observed {reply:?}").into());
    }
    Ok(())
}

async fn persisted(clients: &SeamClients<'_>) -> TestResult<(String, String, String)> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let selected = sqlx::query_as(
        "SELECT a.reset_epoch::text, b.creature_key, b.boss_key
         FROM game_world_reset_active a JOIN game_world_daily_boosts b USING (world_id, reset_epoch)
         WHERE a.world_id = encode($1, 'hex')::uuid",
    )
    .bind(clients.world.as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(selected)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let mut client = connect_world(clients, 0xfa).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Err("admission did not offer daily-boost capability 151".into());
    }
    let session = client.join_snapshot().game_session_id;
    let reply = query(&mut client, &[]).await?;
    require_reply(&reply, CommandStatus::Accepted)?;
    let expected = persisted(clients).await?;
    let refused = query(&mut client, &[0]).await?;
    require_reply(&refused, CommandStatus::Rejected)?;
    if persisted(clients).await? != expected {
        return Err("malformed query changed the persisted daily boost".into());
    }
    if client.session_mut().logout().await?.result != Some(LogoutResult::Accepted) {
        return Err("daily boost logout refused".into());
    }
    drop(client);

    let mut fresh = connect_world(clients, 0xfb).await?;
    let fresh_session = fresh.join_snapshot().game_session_id;
    if session == fresh_session || persisted(clients).await? != expected {
        return Err("daily boost readmission failed to retain the persisted selection".into());
    }
    let readmitted = query(&mut fresh, &[]).await?;
    require_reply(&readmitted, CommandStatus::Accepted)?;
    if fresh.session_mut().logout().await?.result != Some(LogoutResult::Accepted) {
        return Err("daily boost readmission logout refused".into());
    }
    let observed = Capture {
        session: uuid_text(session.as_bytes()),
        fresh_session: uuid_text(fresh_session.as_bytes()),
        expected,
        replies: [reply, refused, readmitted]
            .into_iter()
            .map(|r| (r.command_id, r.server_sequence, r.status as u32, r.payload))
            .collect(),
    };
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory missing")?;
    capture(
        Path::new(&root),
        "w13-daily-boost.json",
        &serde_json::to_vec(&observed)?,
    )?;
    Ok(observed.provenance())
}

#[test]
#[ignore = "requires a fresh W13 live capture"]
fn e2e_w13_replays_daily_boost_selection_and_refusal() -> TestResult {
    use daily_boost::{Intent, apply_result, panel_text, request, spotlight_cards};

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory missing")?;
    let observed: Capture = serde_json::from_slice(&std::fs::read(
        Path::new(&root).join("w13-daily-boost.json"),
    )?)?;
    assert_eq!(observed.provenance(), required("W13_CAPTURE_PROVENANCE")?);
    assert_ne!(observed.session, observed.fresh_session);
    let [accepted, refused, readmitted] = observed.replies.as_slice() else {
        return Err("expected three live Daily Boost replies".into());
    };
    assert_eq!(accepted.0.checked_add(1), Some(refused.0));
    assert!(accepted.1 < refused.1);
    let (epoch, creature, boss) = &observed.expected;
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    assert!(panel.boost.is_none());
    for (record, status) in [
        (accepted, CommandStatus::Accepted),
        (refused, CommandStatus::Rejected),
        (readmitted, CommandStatus::Accepted),
    ] {
        assert_ne!(record.0, 0);
        assert_ne!(record.1, 0);
        assert_eq!(record.2, status as u32);
        let before = panel.boost.clone();
        request(
            &mut panel,
            Intent {
                command: wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
                payload: Vec::new(),
            },
        )
        .ok_or("query request")?;
        assert_eq!(panel_text(&panel), "Loading daily boost...");
        apply_result(
            &mut panel,
            &Reply {
                command_id: record.0,
                server_sequence: record.1,
                status,
                payload: record.3.clone(),
            },
        );
        assert!(!panel.pending);
        if status == CommandStatus::Rejected {
            assert!(record.3.is_empty());
            assert_eq!(panel.boost, before);
            assert_eq!(panel.feedback, Some("The server refused the action"));
            panel = DailyBoostPanel {
                selected: true,
                ..Default::default()
            };
            continue;
        }
        let view = panel.boost.as_ref().ok_or("accepted reply has no boost")?;
        assert_eq!(
            (
                &view.reset_epoch.to_string(),
                &view.creature_key,
                &view.boss_key
            ),
            (epoch, creature, boss)
        );
        assert_eq!(panel.feedback, None);
        let cards = spotlight_cards(&panel);
        assert_eq!(cards.len(), 2);
        assert_eq!(
            (cards[0].key.as_str(), cards[1].key.as_str()),
            (creature.as_str(), boss.as_str())
        );
        assert_eq!(
            (cards[0].kind, cards[1].kind),
            ("Boosted creature", "Boosted boss")
        );
        assert_eq!(
            panel_text(&panel),
            format!(
                "Daily spotlight\nReset epoch: {epoch}\n{}: {}\n{creature}\n{}: {}\n{boss}\n",
                cards[0].kind, cards[0].name, cards[1].kind, cards[1].name,
            )
        );
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_capture scenario=W13.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W13.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W13.client status=FAIL evidence=timeout"),
    }
}
