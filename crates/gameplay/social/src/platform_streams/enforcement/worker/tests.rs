use super::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn run(future: impl Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future)
}
fn fixture(name: &str) -> TestResult<Vec<u8>> {
    Ok(std::fs::read(format!(
        "{}/../../../docs/contracts/fixtures/game-enforcement-wire-v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))?)
}
#[derive(Default)]
struct Data {
    cursor: Option<String>,
    reconcile: Option<String>,
    stopped: bool,
    stored: HashMap<String, Vec<u8>>,
    outbox: Vec<Publication>,
    acked: HashSet<String>,
    held: HashSet<String>,
    missing: HashSet<String>,
    alarmed: HashSet<String>,
    alarms: Vec<&'static str>,
    committed: Vec<String>,
    fail_second: bool,
}
#[derive(Default)]
struct Memory(Mutex<Data>);
impl Store for Memory {
    fn alarm(&self, reason: &'static str) {
        if let Ok(mut s) = self.0.lock() {
            s.alarms.push(reason);
        }
    }
    async fn transact(&self, action: Action) -> Result<Reply, ()> {
        let mut s = self.0.lock().map_err(|_| ())?;
        Ok(match action {
            Action::Cursor { reconcile } => Reply::Cursor(
                if reconcile {
                    s.reconcile.clone()
                } else {
                    s.cursor.clone()
                },
                s.stopped,
            ),
            Action::Advance {
                reconcile,
                expected,
                next,
            } => {
                let cursor = if reconcile {
                    &mut s.reconcile
                } else {
                    &mut s.cursor
                };
                if *cursor != expected {
                    return Err(());
                }
                *cursor = next;
                Reply::Done
            }
            Action::Stop { .. } => {
                s.stopped = true;
                Reply::Done
            }
            Action::Next => Reply::Publication(
                s.outbox
                    .iter()
                    .find(|p| {
                        !s.acked.contains(&p.operation)
                            && !s.held.contains(&p.operation)
                            && !s.missing.contains(&p.operation)
                    })
                    .cloned(),
            ),
            Action::Lookup { operation } => Reply::Bytes(s.stored.get(&operation).cloned()),
            Action::Mark {
                operation,
                delivery,
                ..
            } => {
                match delivery {
                    Delivery::Ack => {
                        s.acked.insert(operation);
                    }
                    Delivery::Hold => {
                        s.held.insert(operation);
                    }
                    Delivery::Missing => {
                        s.missing.insert(operation);
                    }
                    Delivery::Republish => {
                        s.acked.remove(&operation);
                    }
                }
                Reply::Done
            }
            Action::Overdue { now_ms } => {
                let due = s
                    .outbox
                    .iter()
                    .filter(|p| {
                        !s.acked.contains(&p.operation) && !s.alarmed.contains(&p.operation)
                    })
                    .filter_map(|p| EnforcementResult::parse(&p.bytes))
                    .filter(|r| now_ms.saturating_sub(r.decided_at_ms) >= 300_000)
                    .map(|r| r.operation_id)
                    .collect::<Vec<_>>();
                let n = due.len();
                for id in due {
                    s.alarmed.insert(id);
                }
                Reply::Alarms(n)
            }
        })
    }
    async fn decide(&self, raw: Vec<u8>) -> Result<Vec<u8>, ()> {
        let command = Command::parse(&raw).ok_or(())?;
        let mut s = self.0.lock().map_err(|_| ())?;
        if s.fail_second && s.committed.len() == 1 {
            s.fail_second = false;
            return Err(());
        }
        if let Some(raw) = s.stored.get(&command.operation_id) {
            return Ok(raw.clone());
        }
        let mut result = EnforcementResult::parse(
            &fixture(if command.intent == Intent::Revoke {
                "result-revoked"
            } else {
                "result-applied"
            })
            .map_err(|_| ())?,
        )
        .ok_or(())?;
        result.operation_id = command.operation_id.clone();
        result.sanction_stream_id = command.sanction_stream_id;
        result.command_digest = digest(&raw);
        let raw = result.encode().ok_or(())?;
        s.committed.push(command.operation_id.clone());
        s.stored.insert(command.operation_id.clone(), raw.clone());
        s.outbox.push(Publication {
            operation: command.operation_id,
            revision: 1,
            bytes: raw.clone(),
        });
        Ok(raw)
    }
}
async fn seeded() -> TestResult<Memory> {
    let store = Memory::default();
    store
        .decide(fixture("command-apply")?)
        .await
        .map_err(|_| "seed")?;
    Ok(store)
}
struct LocalPeer {
    clients: Clients,
    lost: AtomicBool,
}
impl Peer for LocalPeer {
    async fn request(&self, route: Route, raw: Vec<u8>) -> Result<Vec<u8>, Failure> {
        let reply = self.clients.exchange(route, raw).await?;
        // Inject expiry of the caller's deadline after the peer received the body.
        if self.lost.swap(false, Ordering::SeqCst) {
            Err(Failure::Retry(None))
        } else {
            Ok(reply)
        }
    }
}
type Captured = Arc<Mutex<Vec<Vec<u8>>>>;
async fn peer(
    replies: Vec<(u16, Option<u64>, Vec<u8>)>,
    lost: bool,
) -> TestResult<(LocalPeer, Captured)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let captured = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&captured);
    tokio::spawn(async move {
        for (status, retry, body) in replies {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut raw = Vec::new();
            let mut chunk = [0; 4096];
            let _body = loop {
                let Ok(n) = stream.read(&mut chunk).await else {
                    return;
                };
                if n == 0 {
                    return;
                }
                raw.extend_from_slice(&chunk[..n]);
                if let Some(end) = raw.windows(4).position(|v| v == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&raw[..end]);
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + length {
                        break raw[end + 4..end + 4 + length].to_vec();
                    }
                }
            };
            if let Ok(mut records) = output.lock() {
                records.push(raw);
            }
            let retry = retry
                .map(|n| format!("Retry-After: {n}\r\n"))
                .unwrap_or_default();
            let header = format!(
                "HTTP/1.1 {status} Reply\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{retry}\r\n",
                body.len()
            );
            if stream.write_all(header.as_bytes()).await.is_err() {
                return;
            }
            if stream.write_all(&body).await.is_err() {
                return;
            }
        }
    });
    let clients = Clients::local(format!("http://{address}"))?;
    Ok((
        LocalPeer {
            clients,
            lost: AtomicBool::new(lost),
        },
        captured,
    ))
}
#[test]
fn enforcement_pull_processes_page_and_advances_cursor_after_commit() -> TestResult {
    run(async {
        let store = Memory::default();
        store.0.lock().map_err(|_| "mutex")?.fail_second = true;
        let page = fixture("stream-page")?;
        let (peer, requests) =
            peer(vec![(200, None, page.clone()), (200, None, page)], false).await?;
        assert!(pull(&store, &peer).await.is_err());
        assert!(store.0.lock().map_err(|_| "mutex")?.cursor.is_none());
        assert_eq!(pull(&store, &peer).await, Ok(Step::More));
        let s = store.0.lock().map_err(|_| "mutex")?;
        assert_eq!(s.committed.len(), 2);
        assert_eq!(s.cursor.as_deref(), Some("v1:2"));
        assert_eq!(requests.lock().map_err(|_| "mutex")?.len(), 2);
        Ok(())
    })
}
#[test]
fn enforcement_pull_backoff_on_429_retry_after_and_503() -> TestResult {
    run(async {
        let store = Memory::default();
        let (peer, _) = peer(vec![(429, Some(90), vec![]), (503, None, vec![])], false).await?;
        assert_eq!(
            pull(&store, &peer).await,
            Err(Failure::Retry(Some(Duration::from_secs(90))))
        );
        assert_eq!(
            backoff(0, Some(Duration::from_secs(90))),
            Duration::from_secs(90)
        );
        assert_eq!(pull(&store, &peer).await, Err(Failure::Retry(None)));
        assert_eq!(backoff(1, None), Duration::from_secs(2));
        assert_eq!(backoff(30, None), Duration::from_secs(60));
        Ok(())
    })
}
#[test]
fn enforcement_pull_400_is_not_retried_unchanged() -> TestResult {
    run(async {
        let store = Memory::default();
        let (peer, requests) = peer(vec![(400, None, vec![])], false).await?;
        assert_eq!(pull(&store, &peer).await, Ok(Step::Stopped));
        assert_eq!(pull(&store, &peer).await, Ok(Step::Stopped));
        assert_eq!(requests.lock().map_err(|_| "mutex")?.len(), 1);
        assert_eq!(store.0.lock().map_err(|_| "mutex")?.alarms, vec!["defect"]);
        Ok(())
    })
}
#[test]
fn enforcement_results_republish_identical_bytes_after_timeout() -> TestResult {
    run(async {
        let store = seeded().await?;
        let ack = fixture("result-ack")?;
        let (peer, requests) = peer(vec![(200, None, ack.clone()), (200, None, ack)], true).await?;
        assert_eq!(publish(&store, &peer, 1).await, Err(Failure::Retry(None)));
        assert!(store.0.lock().map_err(|_| "mutex")?.acked.is_empty());
        assert_eq!(publish(&store, &peer, 1).await, Ok(Step::More));
        let requests = requests.lock().map_err(|_| "mutex")?;
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], requests[1]);
        Ok(())
    })
}
async fn refusal(status: u16, reason: &str) -> TestResult {
    let store = seeded().await?;
    let (peer, requests) = peer(vec![(status, None, vec![])], false).await?;
    assert_eq!(publish(&store, &peer, 1).await, Ok(Step::More));
    assert_eq!(publish(&store, &peer, 1).await, Ok(Step::Idle));
    let s = store.0.lock().map_err(|_| "mutex")?;
    assert_eq!(s.alarms, vec![reason]);
    assert!(s.acked.is_empty());
    assert_eq!(s.held.len() + s.missing.len(), 1);
    assert_eq!(requests.lock().map_err(|_| "mutex")?.len(), 1);
    Ok(())
}
#[test]
fn enforcement_results_409_holds_and_alarms() -> TestResult {
    run(refusal(409, "conflict"))
}
#[test]
fn enforcement_results_404_stops_and_alarms() -> TestResult {
    run(refusal(404, "missing"))
}
#[test]
fn enforcement_reconcile_republishes_missing_and_alarms_on_digest_mismatch() -> TestResult {
    run(async {
        let store = seeded().await?;
        store
            .decide(fixture("command-revoke")?)
            .await
            .map_err(|_| "seed")?;
        {
            let mut s = store.0.lock().map_err(|_| "mutex")?;
            s.acked = s.stored.keys().cloned().collect();
        }
        let mut page = ReconcilePage::parse(&fixture("reconcile-page")?).ok_or("page")?;
        page.items[0].result_digest = Some("0".repeat(64));
        let (peer, _) = peer(
            vec![(
                200,
                None,
                Message::Reconcile(page).encode().ok_or("encode")?,
            )],
            false,
        )
        .await?;
        assert_eq!(reconcile(&store, &peer).await, Ok(Step::More));
        let s = store.0.lock().map_err(|_| "mutex")?;
        assert_eq!(s.acked.len(), 1);
        assert_eq!(s.alarms, vec!["drift"]);
        Ok(())
    })
}
#[test]
fn enforcement_overdue_alarm_after_5_minutes_once() -> TestResult {
    run(async {
        let store = seeded().await?;
        let result = EnforcementResult::parse(&fixture("result-applied")?).ok_or("result")?;
        let (peer, _) = peer(
            vec![
                (503, None, vec![]),
                (503, None, vec![]),
                (503, None, vec![]),
            ],
            false,
        )
        .await?;
        for now in [
            result.decided_at_ms + 299_999,
            result.decided_at_ms + 300_000,
            result.decided_at_ms + 300_001,
        ] {
            assert!(publish(&store, &peer, now).await.is_err());
        }
        assert_eq!(store.0.lock().map_err(|_| "mutex")?.alarms, vec!["overdue"]);
        Ok(())
    })
}

#[test]
fn enforcement_reconcile_fetches_unknown_command_by_operation() -> TestResult {
    run(async {
        let store = Memory::default();
        let mut page = ReconcilePage::parse(&fixture("reconcile-page")?).ok_or("page")?;
        page.items.truncate(1);
        page.items[0].state = "PENDING".into();
        page.items[0].receipt_revision = None;
        page.items[0].result_digest = None;
        page.next_cursor = Some("v1:1".into());
        let (peer, captured) = peer(
            vec![
                (
                    200,
                    None,
                    Message::Reconcile(page).encode().ok_or("encode")?,
                ),
                (200, None, fixture("command-apply")?),
            ],
            false,
        )
        .await?;
        assert_eq!(reconcile(&store, &peer).await, Ok(Step::More));
        let captured = captured.lock().map_err(|_| "mutex")?;
        assert_eq!(captured.len(), 2);
        assert!(captured[0].starts_with(
            b"POST /internal/v1/support/enforcement-commands/reconcile/read HTTP/1.1\r\n"
        ));
        assert!(captured[1].starts_with(
            b"POST /internal/v1/support/enforcement-commands/commands/read HTTP/1.1\r\n"
        ));
        assert!(captured[1].ends_with(&fixture("read-request")?));
        assert_eq!(store.0.lock().map_err(|_| "mutex")?.committed.len(), 1);
        Ok(())
    })
}
#[test]
fn enforcement_results_unknown_or_oversize_ack_never_clears_outbox() -> TestResult {
    run(async {
        for reply in [vec![b'x'; 129], b"{}".to_vec()] {
            let store = seeded().await?;
            let (peer, _) = peer(vec![(200, None, reply)], false).await?;
            assert_eq!(publish(&store, &peer, 1).await, Err(Failure::Retry(None)));
            assert!(store.0.lock().map_err(|_| "mutex")?.acked.is_empty());
        }
        Ok(())
    })
}
