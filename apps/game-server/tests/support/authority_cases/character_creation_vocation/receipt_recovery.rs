use super::*;
use durability::character_authority::ReconciledCharacterAuthority;
use durability::character_command::RetainedCreatePublisherStore;
use native_admission_source::SourceError;
use native_admission_source::character_commands::{
    CommandSource, Consumer, CreateSendability, Decider, Failure, RetainedCreatePage,
    RetainedCreateScan, classify, run_retained_create_publications,
};
use native_admission_source::descriptor::Operation;
use native_admission_source::runtime_status::{ReportClock, SystemClock};
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::sync::Notify;

const SOURCE: &str = "oteryn:character-authority:primary";
const ACK: &[u8] = br#"{"contract_version":1,"result":"accepted"}"#;
type Calls = Arc<Mutex<Vec<(Operation, String)>>>;

struct Peer {
    calls: Calls,
    replies: VecDeque<Result<Vec<u8>, Failure>>,
}
impl CommandSource for Peer {
    async fn exchange(&mut self, operation: Operation, body: &str) -> Result<Vec<u8>, Failure> {
        self.calls
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .push((operation, body.to_owned()));
        self.replies
            .pop_front()
            .unwrap_or(Err(Failure::InvalidResponse))
    }
}

/// Signal only after a whole poll, including all database passes, returns.
struct PollBoundary {
    time: SystemClock,
    remaining: AtomicUsize,
    reached: Arc<Notify>,
}

impl ReportClock for PollBoundary {
    fn elapsed(&self) -> Duration {
        self.time.elapsed()
    }
    fn unix_now(&self) -> i64 {
        self.time.unix_now()
    }

    async fn sleep(&self, _: Duration) {
        if self.remaining.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.reached.notify_one();
            std::future::pending::<()>().await;
        }
        tokio::task::yield_now().await;
    }
}

async fn drive<F: std::future::Future<Output = ()>>(
    polls: usize,
    run: impl FnOnce(PollBoundary) -> F,
) -> TestResult {
    let reached = Arc::new(Notify::new());
    let future = run(PollBoundary {
        time: SystemClock::default(),
        remaining: AtomicUsize::new(polls),
        reached: reached.clone(),
    });
    tokio::select! {
        () = reached.notified() => Ok(()),
        () = future => Err("publication loop stopped".into()),
        () = tokio::time::sleep(Duration::from_secs(20)) => Err("publication loop missed its poll boundary".into()),
    }
}

/// Observe exact reads while using the production store for all decisions.
struct ObserveReads<'a, 'f, 's> {
    store: RetainedCreatePublisherStore<'a, 'f, 's>,
    exact_reads: &'a AtomicUsize,
}
impl RetainedCreateScan for ObserveReads<'_, '_, '_> {
    async fn scan_retained_creates(
        &mut self,
        after: i64,
        through: Option<i64>,
    ) -> Result<RetainedCreatePage, SourceError> {
        self.store.scan_retained_creates(after, through).await
    }
    async fn sendable_create(&mut self, operation: &str) -> Result<CreateSendability, SourceError> {
        self.exact_reads.fetch_add(1, Ordering::Relaxed);
        self.store.sendable_create(operation).await
    }
    async fn stop_retained_create_receipt(&mut self, operation: &str) -> Result<(), SourceError> {
        self.store.stop_retained_create_receipt(operation).await
    }
}

pub(super) async fn exercise(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    receipt: &[u8],
) -> TestResult {
    let calls = Calls::default();
    let mut peer = Peer {
        calls: calls.clone(),
        replies: VecDeque::from([
            Err(Failure::Unavailable),
            Err(Failure::NotFound),
            Err(Failure::Unavailable),
            Ok(ACK.to_vec()),
        ]),
    };
    let body = std::str::from_utf8(receipt)?;
    assert!(matches!(
        peer.exchange(Operation::PublishCharacterCommandReceiptV1, body)
            .await,
        Err(Failure::Unavailable)
    ));
    let decoded: serde_json::Value = serde_json::from_slice(receipt)?;
    let operation_id = decoded["operation_id"].as_str().ok_or("operation")?;
    let read = native_admission_source::character_commands::encode_read(
        SOURCE,
        CREATE_COMMAND,
        operation_id,
    )?;
    assert!(matches!(
        peer.exchange(Operation::ReadCharacterCommandIntentV1, &read)
            .await,
        Err(Failure::NotFound)
    ));
    let exact_reads = AtomicUsize::new(0);
    let store = ObserveReads {
        store: RetainedCreatePublisherStore::new(root, authority, node, SOURCE)?,
        exact_reads: &exact_reads,
    };
    drive(2, |clock| {
        run_retained_create_publications(clock, peer, store, |_| {})
    })
    .await?;
    {
        let calls = calls.lock().map_err(|_| "publication calls poisoned")?;
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].1.as_bytes(), receipt);
        for (operation, bytes) in &calls[2..] {
            assert_eq!(*operation, Operation::PublishCharacterCommandReceiptV1);
            assert_eq!(bytes.as_bytes(), receipt);
        }
    }
    assert_eq!(exact_reads.load(Ordering::Relaxed), 2);
    let stopped = Calls::default();
    let store = RetainedCreatePublisherStore::new(root, authority, node, SOURCE)?;
    drive(1, |clock| {
        run_retained_create_publications(
            clock,
            Peer {
                calls: stopped.clone(),
                replies: VecDeque::from([Err(classify(409, b""))]),
            },
            store,
            |_| {},
        )
    })
    .await?;
    assert_eq!(
        *stopped.lock().map_err(|_| "publication calls poisoned")?,
        [(Operation::PublishCharacterCommandReceiptV1, body.to_owned())]
    );
    let mut restarted = RetainedCreatePublisherStore::new(root, authority, node, SOURCE)?;
    assert!(
        restarted
            .scan_retained_creates(0, None)
            .await
            .map_err(|_| "fresh scan")?
            .keys
            .is_empty()
    );
    assert!(matches!(
        restarted.sendable_create(operation_id).await,
        Ok(CreateSendability::StoppedOrMissing)
    ));
    root.stop_retained_create_receipt(authority, node, operation_id)
        .await?;
    assert!(
        root.stop_retained_create_receipt(authority, node, &uuid(id(99)))
            .await
            .is_err()
    );
    eprintln!("RETAINED_CREATE_RECEIPT_PG_EXECUTED publish_failure=2 read_404=1 recovered=1");
    Ok(())
}

#[path = "publication_invariants.rs"]
mod publication_invariants;
