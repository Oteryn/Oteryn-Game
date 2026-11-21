//! Fixed-clock publisher scenarios; the peer rejects overlapping exchanges.
use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Default)]
struct Clock(Arc<AtomicU64>);
impl ReportClock for Clock {
    fn elapsed(&self) -> Duration {
        Duration::from_secs(self.0.load(Ordering::SeqCst))
    }
    fn unix_now(&self) -> i64 {
        1790000003
    }
    async fn sleep(&self, _duration: Duration) {
        std::future::pending::<()>().await;
    }
}
struct MemoryStore {
    snapshot: Option<Snapshot>,
    acks: usize,
    quarantined: bool,
    feed_quarantined: bool,
}
impl Store for MemoryStore {
    async fn next(&mut self) -> Result<Option<Snapshot>, ()> {
        Ok(if self.quarantined || self.feed_quarantined {
            None
        } else {
            self.snapshot.clone()
        })
    }
    async fn acknowledge(&mut self, snapshot: &Snapshot) -> Result<(), ()> {
        if self
            .snapshot
            .as_ref()
            .is_some_and(|s| s.epoch == snapshot.epoch && s.revision == snapshot.revision)
        {
            self.snapshot = None;
            self.acks += 1;
        }
        Ok(())
    }
    async fn quarantine(&mut self, _snapshot: &Snapshot) -> Result<(), ()> {
        self.quarantined = true;
        Ok(())
    }
    async fn quarantine_feed(&mut self, _epoch: u64) -> Result<(), ()> {
        self.feed_quarantined = true;
        Ok(())
    }
    async fn facts(&mut self) -> Result<WatermarkFacts, ()> {
        if self.feed_quarantined {
            return Err(());
        }
        Ok(WatermarkFacts {
            projection_epoch: 1,
            oldest_undelivered_ms: self.snapshot.as_ref().map(|_| 1790000002000),
            now_ms: 1790000003000,
        })
    }
}
struct Fence {
    value: u64,
    writes: usize,
}
impl EpochFence for Fence {
    fn read(&mut self) -> Result<u64, crate::account_characters::FenceUnusable> {
        Ok(self.value)
    }
    fn persist(&mut self, epoch: u64) -> Result<(), crate::account_characters::FenceUnusable> {
        self.writes += 1;
        self.value = epoch;
        Ok(())
    }
}
#[derive(Default)]
struct Seen {
    requests: Vec<(Operation, String)>,
    active: usize,
    maximum: usize,
    drop_state: bool,
    quarantine_state: bool,
    embargo: bool,
}
struct FakePeer(Arc<Mutex<Seen>>);
impl Peer for FakePeer {
    async fn send(&mut self, operation: Operation, body: &str) -> Result<(), Failure> {
        {
            let mut seen = self.0.lock().map_err(|_| Failure::Retry(None))?;
            seen.active += 1;
            seen.maximum = seen.maximum.max(seen.active);
            assert_eq!(seen.active, 1);
            seen.requests.push((operation, body.to_owned()));
        }
        tokio::task::yield_now().await;
        let mut seen = self.0.lock().map_err(|_| Failure::Retry(None))?;
        seen.active -= 1;
        if seen.embargo {
            seen.embargo = false;
            return Err(Failure::Retry(Some(Duration::from_secs(999))));
        }
        if operation == Operation::PublishCharacterServiceStateV2 {
            if seen.quarantine_state {
                return Err(Failure::Quarantine);
            }
            if seen.drop_state {
                seen.drop_state = false;
                return Err(Failure::Retry(None));
            }
        }
        Ok(())
    }
}
type Subject = Publisher<Clock, MemoryStore, FakePeer, Fence>;
type Fixture = (Subject, Arc<Mutex<Seen>>);
fn publisher(seen: Seen, floor: u64) -> Result<Fixture, Box<dyn std::error::Error>> {
    let facts: Value = serde_json::from_slice(include_bytes!(
        "../../../../docs/contracts/fixtures/character-services-v2/state.json"
    ))?;
    let snapshot = Snapshot {
        account_id: facts["account_id"].as_str().ok_or("account")?.into(),
        character_id: facts["character_id"].as_str().ok_or("character")?.into(),
        epoch: 1,
        revision: 43,
        facts,
    };
    let seen = Arc::new(Mutex::new(seen));
    Ok((
        Publisher::new(
            Clock::default(),
            MemoryStore {
                snapshot: Some(snapshot),
                acks: 0,
                quarantined: false,
                feed_quarantined: false,
            },
            FakePeer(seen.clone()),
            Fence {
                value: floor,
                writes: 0,
            },
            "oteryn:character-services:primary".into(),
            Duration::from_secs(3),
        ),
        seen,
    ))
}
fn runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}
#[test]
fn character_services_projection_golden_snapshot_ack_and_one_inflight()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let (mut p, seen) = publisher(Seen::default(), 1)?;
        assert!(p.step().await);
        assert_eq!(p.store.acks, 1);
        assert!(p.store.snapshot.is_none());
        assert_eq!(p.fence.writes, 0);
        let seen = seen.lock().map_err(|_| "peer lock")?;
        assert_eq!(seen.maximum, 1);
        let body = &seen
            .requests
            .iter()
            .find(|(op, _)| *op == Operation::PublishCharacterServiceStateV2)
            .ok_or("snapshot request")?
            .1;
        assert_eq!(
            body.as_bytes(),
            include_bytes!("../../../../docs/contracts/fixtures/character-services-v2/state.json")
        );
        Ok(())
    })
}
#[test]
fn character_services_projection_lost_ack_retries_identical_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let (mut p, seen) = publisher(
            Seen {
                drop_state: true,
                ..Seen::default()
            },
            1,
        )?;
        assert!(!p.step().await);
        assert_eq!(p.store.acks, 0);
        assert!(p.store.snapshot.is_some());
        p.clock.0.store(1, Ordering::SeqCst);
        assert!(p.step().await);
        assert_eq!(p.store.acks, 1);
        let seen = seen.lock().map_err(|_| "peer lock")?;
        let bytes: Vec<_> = seen
            .requests
            .iter()
            .filter(|(op, _)| *op == Operation::PublishCharacterServiceStateV2)
            .map(|(_, body)| body)
            .collect();
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes[0], bytes[1]);
        Ok(())
    })
}
#[test]
fn character_services_projection_quarantine_and_restore_floor_never_ack()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let (mut p, seen) = publisher(
            Seen {
                quarantine_state: true,
                ..Seen::default()
            },
            1,
        )?;
        assert!(!p.step().await);
        assert!(p.store.quarantined);
        p.clock.0.store(4, Ordering::SeqCst);
        assert!(!p.step().await);
        assert_eq!(p.store.acks, 0);
        assert_eq!(
            seen.lock()
                .map_err(|_| "peer lock")?
                .requests
                .iter()
                .filter(|(op, _)| *op == Operation::PublishCharacterServiceStateV2)
                .count(),
            1
        );
        let (mut p, seen) = publisher(Seen::default(), 2)?;
        assert!(!p.step().await);
        assert_eq!(p.store.acks, 0);
        assert!(seen.lock().map_err(|_| "peer lock")?.requests.is_empty());
        assert_eq!(p.fence.writes, 0);
        Ok(())
    })
}
#[test]
fn character_services_projection_retry_after_is_not_shortened()
-> Result<(), Box<dyn std::error::Error>> {
    runtime()?.block_on(async {
        let (mut p, seen) = publisher(
            Seen {
                embargo: true,
                ..Seen::default()
            },
            1,
        )?;
        assert!(!p.step().await);
        p.clock.0.store(998, Ordering::SeqCst);
        assert!(!p.step().await);
        assert_eq!(seen.lock().map_err(|_| "peer lock")?.requests.len(), 1);
        p.clock.0.store(999, Ordering::SeqCst);
        assert!(p.step().await);
        assert_eq!(p.store.acks, 1);
        Ok(())
    })
}
#[test]
fn character_services_projection_rejects_reused_purpose_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let cert = CertificateDer::from(
        include_bytes!(
            "../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der"
        )
        .to_vec(),
    );
    let key = PrivateKeyDer::try_from(
        include_bytes!(
            "../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der"
        )
        .to_vec(),
    )?;
    let root = CertificateDer::from(
        include_bytes!("../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der")
            .to_vec(),
    );
    assert!(
        Descriptor::new(
            ("127.0.0.1".into(), 443),
            "source.test".into(),
            vec![root],
            vec![cert.clone()],
            key,
            &[&[cert]]
        )
        .is_err()
    );
    Ok(())
}
#[test]
fn character_services_projection_locked_lifecycle_is_never_active()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut publisher, _) = publisher(Seen::default(), 1)?;
    let snapshot = publisher.store.snapshot.as_mut().ok_or("snapshot")?;
    snapshot.facts["lifecycle"] = json!("UNSUPPORTED_LIFECYCLE");
    assert!(
        snapshot
            .encode("oteryn:character-services:primary")
            .is_err()
    );
    Ok(())
}
