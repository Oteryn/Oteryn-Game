use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn run(f: impl Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(f)
}
fn fixture(name: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(format!(
        "{}/../../../docs/contracts/fixtures/profile-b-authority-v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
}
fn sample() -> Result<(Request, Answer), Box<dyn std::error::Error>> {
    let Some(Message::Request(r)) = Message::parse(&fixture("authority-request")?) else {
        return Err("request".into());
    };
    let Some(Message::Answer(a)) = Message::parse(&fixture("authority")?) else {
        return Err("answer".into());
    };
    Ok((r, a))
}
#[derive(Default)]
struct Data {
    view: View,
    cursor: Option<String>,
    stopped: bool,
    accepts: usize,
}
#[derive(Default)]
struct Memory(Mutex<Data>);
impl Store for Memory {
    async fn load(&self, _: &str) -> Result<View, ()> {
        Ok(self.0.lock().map_err(|_| ())?.view.clone())
    }
    async fn accept(&self, r: &Request, p: &[(String, u64)], raw: &[u8]) -> Result<Acceptance, ()> {
        let Some(Message::Answer(a)) = Message::parse(raw) else {
            return Err(());
        };
        if !a.bound_to(r, p) {
            return Err(());
        }
        let mut d = self.0.lock().map_err(|_| ())?;
        d.accepts += 1;
        if d.view
            .answer
            .as_ref()
            .is_some_and(|old| a.authority_revision <= old.authority_revision)
        {
            return Ok(Acceptance::Ignored(d.view.clone()));
        }
        if a.entitlements.iter().any(|e| {
            d.view.hints.iter().any(|h| {
                h.product_id == e.product_id
                    && h.product_version == e.product_version
                    && h.lifecycle_revision > e.lifecycle_revision
            })
        }) {
            return Ok(Acceptance::Conflict(d.view.clone()));
        }
        d.view.answer = Some(a);
        Ok(Acceptance::Accepted(d.view.clone()))
    }
    async fn cursor(&self) -> Result<(Option<String>, bool), ()> {
        let d = self.0.lock().map_err(|_| ())?;
        Ok((d.cursor.clone(), d.stopped))
    }
    async fn apply(&self, expected: Option<&str>, raw: &[u8]) -> Result<bool, ()> {
        let Some(Message::Page(page)) = Message::parse(raw) else {
            return Err(());
        };
        let mut d = self.0.lock().map_err(|_| ())?;
        if d.cursor.as_deref() != expected {
            return Ok(false);
        }
        for h in page.items {
            if let Some(old) =
                d.view.hints.iter_mut().find(|o| {
                    o.product_id == h.product_id && o.product_version == h.product_version
                })
            {
                old.lifecycle_revision = old.lifecycle_revision.max(h.lifecycle_revision);
            } else {
                d.view.hints.push(h);
            }
        }
        if page.next_cursor.is_some() {
            d.cursor = page.next_cursor;
        }
        Ok(true)
    }
    async fn stop(&self) -> Result<(), ()> {
        self.0.lock().map_err(|_| ())?.stopped = true;
        Ok(())
    }
    async fn known_hints(&self, a: &[String], p: &[(String, u64)]) -> Result<Vec<Hint>, ()> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ())?
            .view
            .hints
            .iter()
            .filter(|h| {
                a.contains(&h.account_id) && p.contains(&(h.product_id.clone(), h.product_version))
            })
            .cloned()
            .collect())
    }
}
struct Source {
    reply: Result<Vec<u8>, Failure>,
    stopped: AtomicBool,
    requests: AtomicUsize,
}
impl Source {
    fn new(reply: Result<Vec<u8>, Failure>) -> Self {
        Self {
            reply,
            stopped: AtomicBool::new(false),
            requests: AtomicUsize::new(0),
        }
    }
}
impl Peer for Source {
    async fn authority(&self, _: &Request, _: &[(String, u64)]) -> Result<Vec<u8>, Failure> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(Failure::Configuration);
        }
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.reply.clone()
    }
    async fn changes(&self, _: Option<String>, _: u16) -> Result<Vec<u8>, Failure> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(Failure::Configuration);
        }
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.reply.clone()
    }
    fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }
}
#[test]
fn profile_b_outages_404_and_unsafe_time_never_renew_benefit() -> TestResult {
    run(async {
        let (r, a) = sample()?;
        let products = a
            .entitlements
            .iter()
            .map(|e| (e.product_id.clone(), e.product_version))
            .collect::<Vec<_>>();
        for failure in [
            Failure::Missing,
            Failure::Retry(None),
            Failure::Retry(Some(Duration::from_secs(90))),
        ] {
            let consumer = Consumer::default();
            consumer.merge(
                &r.account_id,
                View {
                    answer: Some(a.clone()),
                    hints: vec![],
                },
                true,
            );
            let source = Source::new(Err(failure));
            let store = Memory::default();
            assert!(
                refresh_once(&store, &consumer, &source, &r, &products)
                    .await
                    .is_err()
            );
            let issued = time(&a.authority_issued_at).ok_or("time")?;
            assert_eq!(
                consumer.class_at(
                    &r.account_id,
                    &products[0].0,
                    1,
                    Some(GameNow {
                        now_us: issued - 1_000_000,
                        uncertainty_us: 0
                    })
                ),
                Evaluation::None,
                "clock before issue cannot recover authority"
            );
            for (seconds, class) in [
                (1800, Evaluation::Active),
                (2400, Evaluation::Stale),
                (3600, Evaluation::Expired),
                (7200, Evaluation::Expired),
            ] {
                assert_eq!(
                    consumer.class_at(
                        &r.account_id,
                        &products[0].0,
                        1,
                        Some(GameNow {
                            now_us: issued + seconds * 1_000_000,
                            uncertainty_us: 0
                        })
                    ),
                    class
                );
            }
            assert_eq!(
                consumer.class_at(&r.account_id, &products[0].0, 1, None),
                Evaluation::None
            );
            assert_eq!(
                consumer.class_at(
                    &r.account_id,
                    &products[0].0,
                    1,
                    Some(GameNow {
                        now_us: issued,
                        uncertainty_us: 5_000_001
                    })
                ),
                Evaluation::None
            );
            assert_eq!(store.0.lock().map_err(|_| "mutex")?.accepts, 0);
        }
        Ok(())
    })
}
#[test]
fn profile_b_hints_only_reduce_then_require_fresh_authority() -> TestResult {
    run(async {
        let (r, mut a) = sample()?;
        let p = a
            .entitlements
            .iter()
            .map(|e| (e.product_id.clone(), e.product_version))
            .collect::<Vec<_>>();
        let store = Memory::default();
        let consumer = Consumer::default();
        let source = Source::new(Ok(Message::Answer(a.clone()).encode().ok_or("encode")?));
        assert!(matches!(
            refresh_once(&store, &consumer, &source, &r, &p).await,
            Ok(Refresh::Accepted(_))
        ));
        let now = Some(GameNow {
            now_us: time(&a.authority_issued_at).ok_or("time")?,
            uncertainty_us: 0,
        });
        let hint = Hint {
            account_id: r.account_id.clone(),
            product_id: p[0].0.clone(),
            product_version: 1,
            lifecycle_revision: 4,
        };
        let page = Source::new(Ok(Message::Page(Page {
            schema: "oteryn.entitlement_change_page.v1".into(),
            items: vec![hint.clone()],
            next_cursor: Some("v1:1".into()),
        })
        .encode()
        .ok_or("page")?));
        assert_eq!(
            changes_once(&store, &page, |h| {
                consumer.hint(h);
            })
            .await,
            Ok(Changes::More)
        );
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, now),
            Evaluation::Stale
        );
        let inactive = Hint {
            product_id: p[1].0.clone(),
            ..hint.clone()
        };
        consumer.hint(&inactive);
        assert_eq!(
            consumer.class_at(&r.account_id, &p[1].0, 1, now),
            Evaluation::None
        );
        let mut offline = hint.clone();
        offline.account_id = "019a0000-0000-7000-8000-000000000003".into();
        assert!(!consumer.hint(&offline));
        assert_eq!(consumer.accounts.lock().map_err(|_| "mutex")?.len(), 1);
        a.authority_revision += 1;
        a.entitlements[0].lifecycle_revision = 4;
        let fresh = Source::new(Ok(Message::Answer(a).encode().ok_or("fresh")?));
        assert!(matches!(
            refresh_once(&store, &consumer, &fresh, &r, &p).await,
            Ok(Refresh::Accepted(_))
        ));
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, now),
            Evaluation::Active
        );
        assert!(
            refresh_once(&store, &consumer, &source, &r, &p)
                .await
                .is_err()
        );
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, now),
            Evaluation::Active,
            "replay changes nothing"
        );
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, None),
            Evaluation::None,
            "fresh answer is no clock recovery anchor"
        );
        Ok(())
    })
}
#[test]
fn profile_b_defects_stop_requests_and_persist_feed_stop() -> TestResult {
    run(async {
        let (r, a) = sample()?;
        let p = a
            .entitlements
            .iter()
            .map(|e| (e.product_id.clone(), e.product_version))
            .collect::<Vec<_>>();
        for error in [Failure::Defect, Failure::Configuration] {
            let store = Memory::default();
            let source = Source::new(Err(error));
            let consumer = Consumer::default();
            assert_eq!(
                refresh_once(&store, &consumer, &source, &r, &p).await,
                Ok(Refresh::Stopped)
            );
            assert!(source.stopped.load(Ordering::Acquire));
            assert_eq!(
                refresh_once(&store, &consumer, &source, &r, &p).await,
                Ok(Refresh::Stopped)
            );
            assert_eq!(
                source.requests.load(Ordering::Relaxed),
                1,
                "global client stop prevents unchanged retries"
            );
            let source = Source::new(Err(error));
            let reduced = AtomicUsize::new(0);
            assert_eq!(
                changes_once(&store, &source, |_| {
                    reduced.fetch_add(1, Ordering::Relaxed);
                })
                .await,
                Ok(Changes::Stopped)
            );
            assert_eq!(
                changes_once(&store, &source, |_| {
                    reduced.fetch_add(1, Ordering::Relaxed);
                })
                .await,
                Ok(Changes::Stopped)
            );
            assert_eq!(reduced.load(Ordering::Relaxed), 0);
            assert_eq!(source.requests.load(Ordering::Relaxed), 1);
            assert!(store.0.lock().map_err(|_| "mutex")?.stopped);
        }
        Ok(())
    })
}
#[test]
fn profile_b_refresh_cadence_empty_list_and_reconnect_wake() -> TestResult {
    run(async {
        let (r, a) = sample()?;
        let issued = time(&a.authority_issued_at).ok_or("time")?;
        assert_eq!(refresh_at(&a, 0), Some(issued + 1_800_000_000));
        assert_eq!(refresh_at(&a, 120), Some(issued + 1_920_000_000));
        let root = DurabilityRoot::connect_test_runtime("postgres://node@127.0.0.1:1/game")?;
        let client = Client::local("http://127.0.0.1:1", true)?;
        let refresher = Refresher::new(root, Some(client), vec![]);
        refresher.admit([0x61; 16]);
        assert_eq!(refresher.active_tasks(), 0);
        let active = Refresher::new(
            refresher.inner.root.clone(),
            refresher.inner.client.clone(),
            a.entitlements
                .iter()
                .map(|e| (e.product_id.clone(), e.product_version))
                .collect(),
        );
        let account = crate::platform_streams::uuid(&r.account_id).ok_or("account")?;
        let wake = Arc::new(Notify::new());
        let task = tokio::spawn(std::future::pending());
        active.schedules.0.lock().map_err(|_| "mutex")?.insert(
            account,
            Schedule {
                wake: wake.clone(),
                task,
            },
        );
        active.admit(account);
        assert_eq!(active.active_tasks(), 1);
        assert!(
            tokio::time::timeout(Duration::ZERO, wake.notified())
                .await
                .is_ok(),
            "reconnect wakes the existing B1 task"
        );
        active.release(account);
        assert_eq!(active.active_tasks(), 0);

        let store = Memory::default();
        let source = Source::new(Ok(fixture("authority")?));
        assert_eq!(
            refresh_once(&store, refresher.consumer(), &source, &r, &[]).await,
            Ok(Refresh::Stopped)
        );
        assert_eq!(source.requests.load(Ordering::Relaxed), 0);
        assert_eq!(CHANGES_POLL, Duration::from_secs(10));
        assert_eq!(retry_delay(0, None), Duration::from_secs(1));
        assert_eq!(retry_delay(20, None), Duration::from_secs(60));
        assert_eq!(
            retry_delay(0, Some(Duration::from_secs(180))),
            Duration::from_secs(60)
        );
        Ok(())
    })
}

#[test]
fn profile_b_healthy_feed_revocation_is_stale_then_revoked() -> TestResult {
    run(async {
        let (r, mut a) = sample()?;
        let p = a
            .entitlements
            .iter()
            .map(|e| (e.product_id.clone(), e.product_version))
            .collect::<Vec<_>>();
        let store = Memory::default();
        let consumer = Consumer::default();
        let source = Source::new(Ok(Message::Answer(a.clone()).encode().ok_or("active")?));
        assert!(matches!(
            refresh_once(&store, &consumer, &source, &r, &p).await,
            Ok(Refresh::Accepted(_))
        ));
        let now = Some(GameNow {
            now_us: time(&a.authority_issued_at).ok_or("time")?,
            uncertainty_us: 0,
        });
        let hint = Hint {
            account_id: r.account_id.clone(),
            product_id: p[0].0.clone(),
            product_version: 1,
            lifecycle_revision: 4,
        };
        consumer.hint(&hint);
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, now),
            Evaluation::Stale
        );
        a.authority_revision += 1;
        a.entitlements[0].lifecycle_revision = 4;
        a.entitlements[0].entitlement_state = CommercialState::Revoked;
        let source = Source::new(Ok(Message::Answer(a).encode().ok_or("revoked")?));
        assert!(matches!(
            refresh_once(&store, &consumer, &source, &r, &p).await,
            Ok(Refresh::Accepted(_))
        ));
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, now),
            Evaluation::None
        );
        assert_eq!(
            consumer
                .accounts
                .lock()
                .map_err(|_| "mutex")?
                .get(&r.account_id)
                .and_then(|v| v.answer.as_ref())
                .ok_or("evidence")?
                .entitlements[0]
                .entitlement_state,
            CommercialState::Revoked
        );
        Ok(())
    })
}

#[test]
fn profile_b_other_node_cursor_cannot_hide_a_captured_hint() -> TestResult {
    run(async {
        let (r, a) = sample()?;
        let p = a
            .entitlements
            .iter()
            .map(|e| (e.product_id.clone(), e.product_version))
            .collect::<Vec<_>>();
        let store = Memory::default();
        let consumer = Consumer::default();
        let source = Source::new(Ok(Message::Answer(a.clone()).encode().ok_or("active")?));
        assert!(matches!(
            refresh_once(&store, &consumer, &source, &r, &p).await,
            Ok(Refresh::Accepted(_))
        ));
        let hint = Hint {
            account_id: r.account_id.clone(),
            product_id: p[0].0.clone(),
            product_version: 1,
            lifecycle_revision: 4,
        };
        let page = Message::Page(Page {
            schema: "oteryn.entitlement_change_page.v1".into(),
            items: vec![hint],
            next_cursor: Some("v1:1".into()),
        })
        .encode()
        .ok_or("page")?;
        assert!(store.apply(None, &page).await.map_err(|_| "apply")?);
        sync_known_hints(&store, &[r.account_id.clone()], &p, |h| {
            consumer.hint(h);
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
        let now = Some(GameNow {
            now_us: time(&a.authority_issued_at).ok_or("time")?,
            uncertainty_us: 0,
        });
        assert_eq!(
            consumer.class_at(&r.account_id, &p[0].0, 1, now),
            Evaluation::Stale
        );
        Ok(())
    })
}
