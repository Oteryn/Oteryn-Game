use super::*;
use std::sync::{Arc, Mutex};

const AUTHORITY: &str = "oteryn:character-authority:primary";
const ACCOUNT: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const MAX_TX: Duration = Duration::from_secs(3);

fn character(index: usize, name: &str) -> CharacterSummary {
    CharacterSummary {
        character_id: format!("01934f10-7c04-7001-805b-3b11223344{index:02x}"),
        world_id: WORLD.into(),
        name: name.into(),
        availability: Availability::Available,
    }
}

fn snapshot(characters: Vec<CharacterSummary>) -> AccountSnapshot {
    AccountSnapshot {
        account_id: ACCOUNT.into(),
        projection_epoch: 1,
        projection_revision: 42,
        source_observed_at: 1_790_000_000,
        characters,
    }
}

#[test]
fn snapshot_matches_the_contract_fixture() {
    let body = encode_snapshot(AUTHORITY, &snapshot(vec![character(1, "Aldric")])).unwrap();
    assert_eq!(
        body,
        concat!(
            r#"{"contract_version":1,"operation":"PublishAccountCharactersV1","#,
            r#""source_authority":"oteryn:character-authority:primary","#,
            r#""account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","#,
            r#""projection_revision":"42","source_observed_at":"1790000000","characters":[{"#,
            r#""character_id":"01934f10-7c04-7001-805b-3b1122334401","#,
            r#""world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","#,
            r#""availability":"AVAILABLE"}]}"#
        )
    );
    let mut locked = character(2, "Bera");
    locked.availability = Availability::Unavailable;
    let body = encode_snapshot(AUTHORITY, &snapshot(vec![locked])).unwrap();
    assert!(body.contains(r#""availability":"UNAVAILABLE""#));
    // An empty list is a valid snapshot.
    assert!(
        encode_snapshot(AUTHORITY, &snapshot(vec![]))
            .unwrap()
            .ends_with(r#""characters":[]}"#)
    );
}

#[test]
fn worst_case_snapshot_fits_and_the_wire_bound_is_64() {
    let name = "N".repeat(64);
    let full: Vec<_> = (0..MAX_CHARACTERS)
        .map(|i| {
            let mut c = character(i, &name);
            c.availability = Availability::Unavailable;
            c
        })
        .collect();
    let body = encode_snapshot(AUTHORITY, &snapshot(full.clone())).unwrap();
    assert!(body.len() <= SNAPSHOT_BYTES, "{}", body.len());
    let mut over = full;
    over.push(character(MAX_CHARACTERS, "Extra"));
    assert!(encode_snapshot(AUTHORITY, &snapshot(over)).is_err());
}

#[test]
fn names_outside_the_contract_are_refused() {
    for name in [
        "",
        &"N".repeat(65),
        "Al\"dric",
        "Al\\dric",
        "Al\ndric",
        "Al\u{7f}dric",
        "Al\u{202e}dric",
        "Al\u{2066}dric",
        "Al\u{200b}dric",
        "Al\u{feff}dric",
        "Ald\u{0301}ric",
    ] {
        assert!(!name_valid(name), "{name:?}");
        assert!(encode_snapshot(AUTHORITY, &snapshot(vec![character(1, name)])).is_err());
    }
    assert!(name_valid("Al Dric"));
}

#[test]
fn snapshot_refuses_unsorted_duplicate_or_malformed_members() {
    let unsorted = vec![character(2, "Bera"), character(1, "Aldric")];
    assert!(encode_snapshot(AUTHORITY, &snapshot(unsorted)).is_err());
    let duplicate = vec![character(1, "Aldric"), character(1, "Aldric")];
    assert!(encode_snapshot(AUTHORITY, &snapshot(duplicate)).is_err());
    let mut upper = character(1, "Aldric");
    upper.world_id = WORLD.to_uppercase();
    assert!(encode_snapshot(AUTHORITY, &snapshot(vec![upper])).is_err());
    let mut zero = snapshot(vec![]);
    zero.projection_revision = 0;
    assert!(encode_snapshot(AUTHORITY, &zero).is_err());
    let mut v4 = snapshot(vec![]);
    v4.account_id = "0190f2a1-3b4c-4d5e-8f60-718293a4b5c6".into();
    assert!(encode_snapshot(AUTHORITY, &v4).is_err());
    assert!(encode_snapshot("bad authority", &snapshot(vec![])).is_err());
}

fn facts(oldest: Option<i64>, now_ms: i64) -> WatermarkFacts {
    WatermarkFacts {
        projection_epoch: 1,
        oldest_undelivered_ms: oldest,
        now_ms,
    }
}

#[test]
fn watermark_never_passes_an_undelivered_change() {
    // Nothing queued: now less the maximum transaction duration.
    assert_eq!(
        complete_through(&facts(None, 1_790_000_010_500), MAX_TX),
        Some(1_790_000_007)
    );
    // A queued change at second 5.000 or 5.001: T stays strictly before it.
    for (oldest, through) in [
        (1_790_000_005_000, 1_790_000_004),
        (1_790_000_005_001, 1_790_000_005),
    ] {
        let t = complete_through(&facts(Some(oldest), 1_790_000_010_500), MAX_TX).unwrap();
        assert_eq!(t, through);
        assert!(t * 1000 < oldest);
    }
    // A queued resync row holds the watermark below the resync start however
    // late it is observed.
    let start = 1_790_000_000_000;
    for later in [start, start + 60_000, start + 3_600_000] {
        let t = complete_through(&facts(Some(start), later), MAX_TX).unwrap();
        assert!(t * 1000 < start);
    }
    assert_eq!(complete_through(&facts(None, 1_000), MAX_TX), None);
    let body = encode_watermark(AUTHORITY, &facts(None, 1_790_000_003_000), MAX_TX).unwrap();
    assert_eq!(
        body,
        concat!(
            r#"{"contract_version":1,"operation":"PublishProjectionWatermarkV1","#,
            r#""source_authority":"oteryn:character-authority:primary","projection_epoch":"1","#,
            r#""complete_through":"1790000000","observed_at":"1790000003"}"#
        )
    );
    assert!(body.len() <= WATERMARK_BYTES);
}

#[test]
fn response_is_exact() {
    assert_eq!(
        decode_response(br#"{"contract_version":1,"result":"accepted"}"#).unwrap(),
        Delivery::Accepted
    );
    assert_eq!(
        decode_response(br#"{"contract_version":1,"result":"superseded"}"#).unwrap(),
        Delivery::Superseded
    );
    for raw in [
        &br#"{"contract_version":1,"result":"refreshed"}"#[..],
        br#"{"contract_version":2,"result":"accepted"}"#,
        br#"{"contract_version":1,"result":"accepted","x":1}"#,
        br#"{"contract_version":1,"result":null}"#,
        br#"{"contract_version":1}"#,
    ] {
        assert!(decode_response(raw).is_err());
    }
    assert!(decode_response(&[b' '; RESPONSE_BYTES + 1]).is_err());
}

#[test]
fn operations_have_compiled_paths_and_bounds() {
    assert_eq!(
        Operation::PublishAccountCharactersV1.path(),
        "/internal/v1/game-auth/native-account-characters"
    );
    assert_eq!(
        Operation::PublishProjectionWatermarkV1.path(),
        "/internal/v1/game-auth/native-account-characters/watermark"
    );
    assert_eq!(
        Operation::PublishAccountCharactersV1.request_bytes_max(),
        16_384
    );
    assert_eq!(
        Operation::PublishProjectionWatermarkV1.request_bytes_max(),
        512
    );
}

#[derive(Clone, Default)]
struct Clock(Arc<Mutex<Duration>>);

impl ReportClock for Clock {
    fn elapsed(&self) -> Duration {
        *self.0.lock().unwrap()
    }
    fn unix_now(&self) -> i64 {
        0
    }
    /// Virtual time: the clock jumps when the sleep is first polled.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        let clock = self.0.clone();
        async move { *clock.lock().unwrap() += duration }
    }
}

#[derive(Default)]
struct Store {
    queued: Vec<AccountSnapshot>,
    cleared: Vec<(String, u64, u64)>,
    /// An unreconciled or unreachable store: every read fails.
    failing: bool,
    /// The watermark epoch when not 1 (a restored store reads a lower one).
    epoch: Option<u64>,
}

impl ProjectionStore for Arc<Mutex<Store>> {
    async fn next_snapshot(&mut self) -> Result<Option<AccountSnapshot>, ()> {
        let store = self.lock().unwrap();
        if store.failing {
            return Err(());
        }
        Ok(store.queued.first().cloned())
    }
    async fn clear(&mut self, account: &str, epoch: u64, revision: u64) -> Result<(), ()> {
        let mut store = self.lock().unwrap();
        store.queued.retain(|s| {
            !(s.account_id == account
                && s.projection_epoch <= epoch
                && s.projection_revision <= revision)
        });
        store.cleared.push((account.into(), epoch, revision));
        Ok(())
    }
    async fn watermark_facts(&mut self) -> Result<WatermarkFacts, ()> {
        let store = self.lock().unwrap();
        if store.failing {
            return Err(());
        }
        let mut facts = facts(None, 1_790_000_003_000);
        facts.projection_epoch = store.epoch.unwrap_or(1);
        Ok(facts)
    }
}

/// Every fence and sink event in order, shared by the fake fence and sink.
type Events = Arc<Mutex<Vec<String>>>;

struct Sink(Vec<Result<Delivery, NotDelivered>>, Vec<Operation>, Events);

impl ProjectionSink for Sink {
    async fn send(&mut self, operation: Operation, _body: &str) -> Result<Delivery, NotDelivered> {
        self.1.push(operation);
        self.2.lock().unwrap().push(format!("send {operation:?}"));
        if self.0.is_empty() {
            Err(NotDelivered::Unavailable)
        } else {
            self.0.remove(0)
        }
    }
}

/// F in memory: `None` is a missing or malformed fence; `writable` false
/// refuses every persist.
struct Fence {
    value: Option<u64>,
    writable: bool,
    events: Events,
}

impl EpochFence for Fence {
    fn read(&mut self) -> Result<u64, FenceUnusable> {
        self.value.ok_or(FenceUnusable)
    }
    fn persist(&mut self, epoch: u64) -> Result<(), FenceUnusable> {
        if !self.writable {
            return Err(FenceUnusable);
        }
        self.events.lock().unwrap().push(format!("persist {epoch}"));
        self.value = Some(epoch);
        Ok(())
    }
}

fn fence(value: Option<u64>, events: &Events) -> Fence {
    Fence {
        value,
        writable: true,
        events: events.clone(),
    }
}

fn fenced_publisher(
    store: &Arc<Mutex<Store>>,
    replies: Vec<Result<Delivery, NotDelivered>>,
    fence: Fence,
) -> Publisher<Clock, Arc<Mutex<Store>>, Sink, Fence> {
    let events = fence.events.clone();
    Publisher::new(
        Clock::default(),
        store.clone(),
        Sink(replies, vec![], events),
        fence,
        AUTHORITY.into(),
        MAX_TX,
    )
}

fn publisher_with(
    store: &Arc<Mutex<Store>>,
    replies: Vec<Result<Delivery, NotDelivered>>,
) -> Publisher<Clock, Arc<Mutex<Store>>, Sink, Fence> {
    fenced_publisher(store, replies, fence(Some(1), &Events::default()))
}

fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn outbox_clears_only_on_acknowledgement_up_to_the_sent_revision() {
    let store = Arc::new(Mutex::new(Store::default()));
    store
        .lock()
        .unwrap()
        .queued
        .push(snapshot(vec![character(1, "Aldric")]));
    let refusals = [
        Err(NotDelivered::Conflict),
        Err(NotDelivered::RateLimited),
        Err(NotDelivered::Unavailable),
        Err(NotDelivered::InvalidResponse),
    ];
    let mut replies = vec![Ok(Delivery::Accepted)];
    for refusal in refusals {
        replies.insert(replies.len() - 1, Ok(Delivery::Accepted));
        replies.insert(replies.len() - 1, refusal);
    }
    // Watermark first (accepted), then the refused snapshot; repeat.
    let mut publisher = publisher_with(&store, replies);
    for round in 0..4 {
        if round > 0 {
            *publisher.clock.0.lock().unwrap() += WATERMARK_PERIOD;
        }
        assert!(!block_on(publisher.step()));
        assert!(store.lock().unwrap().cleared.is_empty());
    }
    assert!(block_on(publisher.step()));
    assert_eq!(store.lock().unwrap().cleared, vec![(ACCOUNT.into(), 1, 42)]);
    assert!(store.lock().unwrap().queued.is_empty());

    // Superseded also clears; an invalid snapshot is never sent and stays queued.
    let store = Arc::new(Mutex::new(Store::default()));
    let mut bad = snapshot(vec![character(1, "Al\"dric")]);
    bad.projection_revision = 7;
    store.lock().unwrap().queued.push(bad);
    let mut publisher = publisher_with(&store, vec![Ok(Delivery::Superseded)]);
    assert!(!block_on(publisher.step()));
    assert_eq!(
        publisher.sink.1,
        vec![Operation::PublishProjectionWatermarkV1]
    );
    assert_eq!(store.lock().unwrap().queued.len(), 1);
}

#[test]
fn watermark_is_sent_on_its_period_even_while_snapshots_fail() {
    let store = Arc::new(Mutex::new(Store::default()));
    store.lock().unwrap().queued.push(snapshot(vec![]));
    let mut publisher = publisher_with(&store, vec![]);
    for _ in 0..3 {
        block_on(publisher.step());
        *publisher.clock.0.lock().unwrap() += WATERMARK_PERIOD;
    }
    let watermarks = publisher
        .sink
        .1
        .iter()
        .filter(|op| **op == Operation::PublishProjectionWatermarkV1)
        .count();
    assert_eq!(watermarks, 3);
    // Not due yet: no second watermark within the period.
    let before = publisher.sink.1.len();
    block_on(publisher.step());
    *publisher.clock.0.lock().unwrap() += Duration::from_millis(1);
    block_on(publisher.step());
    assert!(
        publisher.sink.1[before..]
            .iter()
            .filter(|op| **op == Operation::PublishProjectionWatermarkV1)
            .count()
            <= 1
    );
}

/// Each store call takes `cost` on the clock.
struct SlowStore {
    clock: Clock,
    cost: Duration,
    inner: Arc<Mutex<Store>>,
}

impl ProjectionStore for SlowStore {
    async fn next_snapshot(&mut self) -> Result<Option<AccountSnapshot>, ()> {
        self.clock.sleep(self.cost).await;
        self.inner.next_snapshot().await
    }
    async fn clear(&mut self, account: &str, epoch: u64, revision: u64) -> Result<(), ()> {
        self.clock.sleep(self.cost).await;
        self.inner.clear(account, epoch, revision).await
    }
    async fn watermark_facts(&mut self) -> Result<WatermarkFacts, ()> {
        self.clock.sleep(self.cost).await;
        self.inner.watermark_facts().await
    }
}

/// Watermark exchanges are refused after `watermark_cost`, or never complete;
/// snapshot exchanges never complete.
struct HangingSink {
    clock: Clock,
    watermark_cost: Option<Duration>,
    watermarks: Vec<Duration>,
    snapshots: Vec<String>,
}

impl ProjectionSink for HangingSink {
    async fn send(&mut self, operation: Operation, body: &str) -> Result<Delivery, NotDelivered> {
        match (operation, self.watermark_cost) {
            (Operation::PublishProjectionWatermarkV1, Some(cost)) => {
                self.watermarks.push(self.clock.elapsed());
                self.clock.sleep(cost).await;
            }
            (Operation::PublishProjectionWatermarkV1, None) => {
                self.watermarks.push(self.clock.elapsed());
                std::future::pending::<()>().await;
            }
            _ => {
                self.snapshots.push(body.into());
                std::future::pending::<()>().await;
            }
        }
        Err(NotDelivered::Unavailable)
    }
}

#[test]
fn watermarks_stay_within_the_gap_bound_in_the_worst_case() {
    // Every store call takes the maximum transaction duration; exchanges take
    // their full compiled bound (1 + 2 + 3 s) or never complete.
    for watermark_cost in [Some(Duration::ZERO), Some(Duration::from_secs(6)), None] {
        let clock = Clock::default();
        let store = Arc::new(Mutex::new(Store::default()));
        store.lock().unwrap().queued.push(snapshot(vec![]));
        let mut publisher = Publisher::new(
            clock.clone(),
            SlowStore {
                clock: clock.clone(),
                cost: MAX_TX,
                inner: store.clone(),
            },
            HangingSink {
                clock: clock.clone(),
                watermark_cost,
                watermarks: vec![],
                snapshots: vec![],
            },
            fence(Some(1), &Events::default()),
            AUTHORITY.into(),
            MAX_TX,
        );
        // The `run` loop, on virtual time.
        while clock.elapsed() < Duration::from_secs(120) {
            if !block_on(publisher.step()) {
                let wait = publisher
                    .watermark_due
                    .saturating_sub(clock.elapsed())
                    .min(IDLE);
                block_on(clock.sleep(wait));
            }
        }
        let sink = &publisher.sink;
        assert!(sink.watermarks.len() >= 12, "{watermark_cost:?}");
        for pair in sink.watermarks.windows(2) {
            assert!(
                pair[1] - pair[0] <= MAX_WATERMARK_GAP,
                "{watermark_cost:?}: {pair:?}"
            );
        }
        // A cut exchange is not delivered: nothing is cleared, and every retry
        // of the same pair carries the same body.
        assert!(store.lock().unwrap().cleared.is_empty());
        if watermark_cost == Some(Duration::ZERO) {
            assert!(!sink.snapshots.is_empty());
        }
        assert!(sink.snapshots.windows(2).all(|pair| pair[0] == pair[1]));
    }
}

fn queued_store(epoch: u64) -> Arc<Mutex<Store>> {
    let store = Arc::new(Mutex::new(Store::default()));
    let mut queued = snapshot(vec![character(1, "Aldric")]);
    queued.projection_epoch = epoch;
    store.lock().unwrap().queued.push(queued);
    store.lock().unwrap().epoch = Some(epoch);
    store
}

#[test]
fn accepted_superseded_conflict_and_unavailable_paths() {
    for (reply, cleared) in [
        (Ok(Delivery::Accepted), true),
        (Ok(Delivery::Superseded), true),
        (Err(NotDelivered::Conflict), false),
        (Err(NotDelivered::Unavailable), false),
    ] {
        let store = queued_store(1);
        let mut publisher = publisher_with(&store, vec![Ok(Delivery::Accepted), reply]);
        assert_eq!(block_on(publisher.step()), cleared);
        assert_eq!(store.lock().unwrap().queued.is_empty(), cleared);
        assert_eq!(
            publisher.sink.1,
            vec![
                Operation::PublishProjectionWatermarkV1,
                Operation::PublishAccountCharactersV1
            ]
        );
    }
    assert_eq!(class(&Ok(Delivery::Superseded)), "superseded");
}

#[test]
fn an_unreconciled_store_publishes_nothing() {
    let store = queued_store(1);
    store.lock().unwrap().failing = true;
    let mut publisher = publisher_with(&store, vec![Ok(Delivery::Accepted); 4]);
    for _ in 0..3 {
        assert!(!block_on(publisher.step()));
        *publisher.clock.0.lock().unwrap() += WATERMARK_PERIOD;
    }
    assert!(publisher.sink.1.is_empty());
    assert!(store.lock().unwrap().cleared.is_empty());
}

#[test]
fn an_unusable_or_higher_fence_refuses_every_publication() {
    // Missing or malformed F, an unwritable F for a higher epoch, and a
    // restored store whose epoch is below F.
    let events = Events::default();
    let unwritable = Fence {
        value: Some(1),
        writable: false,
        events: events.clone(),
    };
    for (epoch, fence) in [
        (1, fence(None, &events)),
        (2, unwritable),
        (4, fence(Some(5), &events)),
    ] {
        let store = queued_store(epoch);
        let mut publisher = fenced_publisher(&store, vec![Ok(Delivery::Accepted); 4], fence);
        for _ in 0..3 {
            assert!(!block_on(publisher.step()));
            *publisher.clock.0.lock().unwrap() += WATERMARK_PERIOD;
        }
        assert!(publisher.sink.1.is_empty(), "{epoch}");
        assert_eq!(store.lock().unwrap().queued.len(), 1);
        assert!(store.lock().unwrap().cleared.is_empty());
    }
    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn a_higher_epoch_persists_the_fence_before_it_is_sent() {
    let events = Events::default();
    let store = queued_store(7);
    let mut publisher = fenced_publisher(
        &store,
        vec![Ok(Delivery::Accepted), Ok(Delivery::Accepted)],
        fence(Some(5), &events),
    );
    assert!(block_on(publisher.step()));
    assert_eq!(publisher.fence.value, Some(7));
    assert_eq!(
        *events.lock().unwrap(),
        vec![
            "persist 7".to_owned(),
            "send PublishProjectionWatermarkV1".into(),
            "send PublishAccountCharactersV1".into(),
        ]
    );
}

#[test]
fn the_fence_file_round_trips_and_refuses_anything_else() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!(
        "oteryn-epoch-fence-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("epoch.fence");
    // Missing: refused, and never created by a write.
    assert!(read_fence(&path).is_err());
    assert!(write_fence(&path, 3).is_err());
    std::fs::write(&path, b"0\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(read_fence(&path), Ok(0));
    write_fence(&path, 1_790_000_000).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"1790000000\n");
    assert_eq!(EpochFenceFile::new(path.clone()).read(), Ok(1_790_000_000));
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    // No temporary file is left behind.
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    // Group- or world-writable, a symlink, or garbage: refused.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o620)).unwrap();
    assert!(read_fence(&path).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let link = directory.join("link.fence");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(read_fence(&link).is_err());
    std::fs::write(&path, b"12x").unwrap();
    assert!(read_fence(&path).is_err());
    std::fs::write(&path, vec![b'1'; FENCE_BYTES + 1]).unwrap();
    assert!(read_fence(&path).is_err());
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_rename_without_a_directory_sync_leaves_the_fence_unusable_until_synced() {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicBool, Ordering};
    static SYNC_FAILS: AtomicBool = AtomicBool::new(false);
    fn sync(directory: &std::os::fd::OwnedFd) -> rustix::io::Result<()> {
        if SYNC_FAILS.load(Ordering::SeqCst) {
            Err(rustix::io::Errno::IO)
        } else {
            rustix::fs::fsync(directory)
        }
    }
    let directory = std::env::temp_dir().join(format!(
        "oteryn-epoch-fence-unsynced-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("epoch.fence");
    std::fs::write(&path, b"5\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut fence = EpochFenceFile::with_sync(path.clone(), sync);
    assert_eq!(fence.read(), Ok(5));
    SYNC_FAILS.store(true, Ordering::SeqCst);
    // The rename lands but the directory sync fails: the write is refused,
    // and F stays unusable at that same epoch until a sync succeeds.
    assert_eq!(fence.persist(9), Err(FenceUnusable));
    assert_eq!(std::fs::read(&path).unwrap(), b"9\n");
    let mut publisher = Publisher::new(
        Clock::default(),
        Arc::new(Mutex::new(Store::default())),
        Sink(vec![], vec![], Events::default()),
        fence,
        AUTHORITY.into(),
        MAX_TX,
    );
    assert_eq!(publisher.admit(9), Err("fence_invalid"));
    assert_eq!(publisher.admit(9), Err("fence_invalid"));
    SYNC_FAILS.store(false, Ordering::SeqCst);
    assert_eq!(publisher.admit(9), Ok(()));
    assert_eq!(publisher.fence.read(), Ok(9));
    // A fresh handle syncs before its first read even if never written.
    SYNC_FAILS.store(true, Ordering::SeqCst);
    assert_eq!(
        EpochFenceFile::with_sync(path.clone(), sync).read(),
        Err(FenceUnusable)
    );
    SYNC_FAILS.store(false, Ordering::SeqCst);
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_symbolic_link_anywhere_in_the_fence_directory_path_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!(
        "oteryn-epoch-fence-links-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    let real = directory.join("real").join("inner");
    std::fs::create_dir_all(&real).unwrap();
    let fence = real.join("epoch.fence");
    std::fs::write(&fence, b"7\n").unwrap();
    std::fs::set_permissions(&fence, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(read_fence(&fence), Ok(7));
    // A link in the middle of the path, and as the fence directory itself.
    std::os::unix::fs::symlink(directory.join("real"), directory.join("middle")).unwrap();
    std::os::unix::fs::symlink(&real, directory.join("last")).unwrap();
    for linked in [
        directory.join("middle").join("inner").join("epoch.fence"),
        directory.join("last").join("epoch.fence"),
    ] {
        assert_eq!(read_fence(&linked), Err(FenceUnusable));
        assert_eq!(write_fence(&linked, 8), Err(FenceUnusable));
        assert_eq!(
            EpochFenceFile::new(linked.clone()).read(),
            Err(FenceUnusable)
        );
        assert_eq!(EpochFenceFile::new(linked).persist(8), Err(FenceUnusable));
    }
    assert_eq!(std::fs::read(&fence).unwrap(), b"7\n");
    // `..` is refused rather than resolved.
    assert_eq!(
        read_fence(&real.join("..").join("inner").join("epoch.fence")),
        Err(FenceUnusable)
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn the_fence_grammar_is_exact() {
    assert_eq!(parse_fence(b"0"), Some(0));
    assert_eq!(parse_fence(b"0\n"), Some(0));
    assert_eq!(parse_fence(b"42\n"), Some(42));
    assert_eq!(parse_fence(b"18446744073709551615"), Some(u64::MAX));
    for raw in [
        &b""[..],
        b"\n",
        b"01",
        b"1\n\n",
        b" 1",
        b"1 ",
        b"-1",
        b"+1",
        b"1\r\n",
        b"18446744073709551616",
    ] {
        assert_eq!(parse_fence(raw), None, "{raw:?}");
    }
}

#[test]
fn a_raise_is_the_migration_expression_and_strictly_above_the_fence() {
    // greatest(epoch + 1, transaction Unix ms), as migration 0028 computes it.
    assert_eq!(
        raised_epoch(1, 1_790_000_000_999, 0),
        Some(1_790_000_000_999)
    );
    assert_eq!(
        raised_epoch(1_790_000_000_005, 1_790_000_000_000, 0),
        Some(1_790_000_000_006)
    );
    // Equal to or below F: refused.
    assert_eq!(raised_epoch(1, 1_790_000_000_000, 1_790_000_000_000), None);
    assert_eq!(raised_epoch(1, 1_790_000_000_000, 1_790_000_000_001), None);
    assert_eq!(
        raised_epoch(1, 1_790_000_000_000, 1_789_999_999_999),
        Some(1_790_000_000_000)
    );
    assert_eq!(raised_epoch(i64::MAX, 0, 0), None);
    assert_eq!(raised_epoch(-5, -1_000_000, 0), None);
}
