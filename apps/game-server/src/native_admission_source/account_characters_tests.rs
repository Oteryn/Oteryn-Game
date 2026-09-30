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
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        *self.0.lock().unwrap() += duration;
        std::future::ready(())
    }
}

#[derive(Default)]
struct Store {
    queued: Vec<AccountSnapshot>,
    cleared: Vec<(String, u64, u64)>,
}

impl ProjectionStore for Arc<Mutex<Store>> {
    async fn next_snapshot(&mut self) -> Result<Option<AccountSnapshot>, ()> {
        Ok(self.lock().unwrap().queued.first().cloned())
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
        Ok(facts(None, 1_790_000_003_000))
    }
}

struct Sink(Vec<Result<Delivery, NotDelivered>>, Vec<Operation>);

impl ProjectionSink for Sink {
    async fn send(&mut self, operation: Operation, _body: &str) -> Result<Delivery, NotDelivered> {
        self.1.push(operation);
        if self.0.is_empty() {
            Err(NotDelivered::Unavailable)
        } else {
            self.0.remove(0)
        }
    }
}

fn publisher_with(
    store: &Arc<Mutex<Store>>,
    replies: Vec<Result<Delivery, NotDelivered>>,
) -> Publisher<Clock, Arc<Mutex<Store>>, Sink> {
    Publisher::new(
        Clock::default(),
        store.clone(),
        Sink(replies, vec![]),
        AUTHORITY.into(),
        MAX_TX,
    )
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
