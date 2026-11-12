use super::*;
use sha2::Digest;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn t94_watermark_bounds_and_no_clock_inference() -> TestResult {
    let mut watermark = Watermark {
        contract_version: 1,
        operation: "PublishLeagueWatermarkV1".into(),
        source_authority: "oteryn:game:primary".into(),
        family: "world_event_outcome".into(),
        projection_epoch: "3".into(),
        complete_through: "100".into(),
        observed_at: "100".into(),
    };
    assert!(watermark.encode(100).is_ok());
    assert!(watermark.fresh(129));
    assert!(!watermark.fresh(130));
    watermark.complete_through = "101".into();
    assert!(watermark.encode(100).is_err());
    watermark.observed_at = "102".into();
    assert!(watermark.encode(100).is_err());
    Ok(())
}

#[test]
fn t94_transport_routes_and_bounds_are_separate() {
    use crate::descriptor::Operation;
    assert_eq!(
        Operation::PublishWorldEventOutcomeV1.request_bytes_max(),
        16384
    );
    assert_eq!(Operation::PublishGuildSeasonV1.request_bytes_max(), 262144);
    assert_eq!(Operation::PublishLeagueWatermarkV1.request_bytes_max(), 512);
    assert_eq!(
        Operation::PublishLeagueWatermarkV1.path(),
        "/internal/v1/game-public/league-watermark"
    );
    assert!(!is_operation(Operation::PublishPublicProjectionWatermarkV1));
}

fn fixture(name: &str) -> std::result::Result<Snapshot, Box<dyn std::error::Error>> {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/world-event-guild-season/snapshots.json"
    ))?;
    let vector = fixtures["vectors"]
        .as_array()
        .ok_or("vectors")?
        .iter()
        .find(|v| v["name"] == name)
        .ok_or("fixture")?;
    Ok(Snapshot::decode(vector["wire"].as_str().ok_or("wire")?)?)
}

#[test]
fn t94_lock_equality_late_schedule_and_revision_gap() -> TestResult {
    let schedule = fixture("advance_schedule")?;
    assert!(schedule.follows(None, 1790000088).is_ok());
    assert!(schedule.follows(None, 1790000089).is_ok());
    assert!(schedule.follows(None, 1790000090).is_err());
    let settled = fixture("settlement")?;
    // Documentation omits LOCKED: never weaken the production transition to
    // make a disconnected fixture sequence an admissible source history.
    assert!(settled.follows(Some(&schedule), 1790000200).is_err());
    let mut locked = schedule.clone();
    if let Snapshot::Event(s) = &mut locked {
        s.projection_revision = "2".into();
        s.source_observed_at = "1790000090".into();
        s.event[0].state = "LOCKED".into();
    }
    assert!(locked.follows(Some(&schedule), 1790000089).is_err());
    assert!(locked.follows(Some(&schedule), 1790000090).is_ok());
    assert!(settled.follows(Some(&locked), 1790000200).is_err());
    let mut first_terminal = settled.clone();
    if let Snapshot::Event(s) = &mut first_terminal {
        s.projection_revision = "3".into();
    }
    assert!(first_terminal.follows(Some(&locked), 1790000200).is_ok());
    assert!(schedule.follows(Some(&schedule), 1790000200).is_ok());
    let mut gap = locked.clone();
    if let Snapshot::Event(s) = &mut gap {
        s.projection_revision = "4".into();
    }
    assert!(gap.follows(Some(&schedule), 1790000090).is_err());
    Ok(())
}

#[test]
fn t94_corrections_bind_immediate_record_and_freeze_is_immutable() -> TestResult {
    let settled = fixture("settlement")?;
    let corrected = fixture("correction")?;
    let closed = fixture("final_freeze")?;
    assert!(corrected.follows(Some(&settled), 1790000200).is_ok());
    assert!(closed.follows(Some(&corrected), 1790000200).is_ok());
    assert!(closed.follows(Some(&closed), 1790000300).is_ok());
    let mut next = corrected.clone();
    if let Snapshot::Event(s) = &mut next {
        s.projection_revision = "4".into();
        s.event[0].outcome = vec!["completed".into()];
        s.event[0].evidence_sha256 = vec!["a".repeat(64)];
    }
    assert!(next.follows(Some(&corrected), 1790000200).is_err());
    if let Snapshot::Event(s) = &mut next {
        s.event[0].correction_of = vec![corrected.record_hash()?];
    }
    assert!(next.follows(Some(&corrected), 1790000200).is_ok());
    if let Snapshot::Event(s) = &mut next {
        s.projection_revision = "5".into();
    }
    assert!(next.follows(Some(&closed), 1790000300).is_err());
    Ok(())
}

#[test]
fn t94_unsafe_source_metadata_never_authorizes_outcome() -> TestResult {
    let mut policy = Policy {
        enabled: true,
        ..Policy::default()
    };
    for name in [
        "advance_schedule",
        "settlement",
        "void",
        "tied_guilds_frozen",
    ] {
        assert_eq!(policy.admit(&fixture(name)?), Err(Failure::InvalidSource));
    }
    policy.event_rules.insert((
        "BOSS".into(),
        "fixture:approved-raid".into(),
        "fixture:raid-result-v1".into(),
    ));
    assert_eq!(
        policy.admit(&fixture("settlement")?),
        Err(Failure::InvalidSource)
    );
    Ok(())
}

#[test]
fn t94_rejects_malformed_wire_and_full_snapshot_bounds() -> TestResult {
    let snapshot = fixture("advance_schedule")?;
    let wire = snapshot.encode()?;
    assert!(Snapshot::decode(&" ".repeat(SEASON_BYTES + 1)).is_err());
    for changed in [
        format!(" {wire}"),
        wire.replace(
            "\"contract_version\":1",
            "\"contract_version\":1,\"contract_version\":1",
        ),
        wire.replace("\"3\"", "\"03\""),
        wire.replace("\"RAID\"", "null"),
        wire.replace("\"RAID\"", "\"raid\""),
        wire.replace("\"RAID\"", "\"R\\u0041ID\""),
        wire.replace(
            "\"contract_version\":1",
            "\"unknown\":1,\"contract_version\":1",
        ),
    ] {
        assert!(Snapshot::decode(&changed).is_err());
    }
    let mut season = fixture("tied_guilds_frozen")?;
    if let Snapshot::Season(s) = &mut season {
        s.season[0].standings.swap(0, 1);
    }
    assert!(season.encode().is_err());
    let mut season = fixture("tied_guilds_frozen")?;
    if let Snapshot::Season(s) = &mut season {
        s.season[0].standings[0].scores[0].score = "101".into();
    }
    assert!(season.encode().is_err());
    assert!(decimal("18446744073709551615", true).is_ok());
    assert!(decimal("18446744073709551616", true).is_err());
    assert!(decimal("0", true).is_ok());
    assert!(decimal("0", false).is_err());
    Ok(())
}

#[test]
fn t94_fixture_bytes_and_record_hashes() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/world-event-guild-season/snapshots.json"
    ))?;
    for vector in fixtures["vectors"].as_array().ok_or("vectors")? {
        let wire = vector["wire"].as_str().ok_or("wire")?;
        let snapshot = Snapshot::decode(wire)?;
        assert_eq!(snapshot.encode()?, wire, "{}", vector["name"]);
        assert_eq!(digest(wire.as_bytes()), vector["sha256"]);
        if let Some(hash) = vector["record_sha256"].as_str() {
            assert_eq!(snapshot.record_hash()?, hash);
        }
    }
    Ok(())
}

#[test]
fn t94_default_on_requires_mappings_and_disable_refuses_fixtures()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/world-event-guild-season/snapshots.json"
    ))?;
    for vector in fixtures["vectors"].as_array().ok_or("vectors")? {
        let snapshot = Snapshot::decode(vector["wire"].as_str().ok_or("wire")?)?;
        assert_eq!(
            Policy::default().admit(&snapshot),
            Err(Failure::InvalidSource)
        );
        assert_eq!(
            Policy {
                enabled: false,
                ..Policy::default()
            }
            .admit(&snapshot),
            Err(Failure::Pending)
        );
    }
    Ok(())
}

#[derive(Default)]
struct MemoryFence(u64);
impl crate::account_characters::EpochFence for MemoryFence {
    fn read(&mut self) -> std::result::Result<u64, crate::account_characters::FenceUnusable> {
        Ok(self.0)
    }
    fn persist(
        &mut self,
        epoch: u64,
    ) -> std::result::Result<(), crate::account_characters::FenceUnusable> {
        self.0 = epoch;
        Ok(())
    }
}
struct TestClock;
impl crate::runtime_status::ReportClock for TestClock {
    fn elapsed(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }
    fn unix_now(&self) -> i64 {
        1790000200
    }
    async fn sleep(&self, _: std::time::Duration) {}
}
struct FixtureStore {
    p: Publication,
    acked: std::cell::Cell<bool>,
}
impl publisher::LeagueStore for FixtureStore {
    async fn current(&self, _: &str, _: u64, token: &str) -> std::result::Result<bool, ()> {
        Ok(token == "fixture-lease")
    }
    async fn produce(&self) -> std::result::Result<(), ()> {
        Ok(())
    }
    async fn next(&self, _: &str) -> std::result::Result<Option<Publication>, ()> {
        Ok((!self.acked.get()).then(|| self.p.clone()))
    }
    async fn failed(
        &self,
        _: &Publication,
        _: crate::runtime_status::NotDelivered,
    ) -> std::result::Result<(), ()> {
        Ok(())
    }
    async fn release_watermark(&self, _: &WatermarkDelivery) -> std::result::Result<(), ()> {
        Ok(())
    }
    async fn ack(&self, p: &Publication) -> std::result::Result<(), ()> {
        if p != &self.p {
            return Err(());
        }
        self.acked.set(true);
        Ok(())
    }
    async fn watermark(&self, _: &str) -> std::result::Result<Option<WatermarkDelivery>, ()> {
        Ok(None)
    }
}
struct FixtureSink {
    wire: String,
    attempts: Vec<String>,
    retry: bool,
}
impl publisher::LeagueSink for FixtureSink {
    async fn send(
        &mut self,
        operation: crate::descriptor::Operation,
        body: &str,
    ) -> std::result::Result<crate::account_characters::Delivery, crate::runtime_status::NotDelivered>
    {
        assert!(is_operation(operation));
        assert_eq!(body, self.wire);
        self.attempts.push(body.into());
        if self.retry {
            self.retry = false;
            Err(crate::runtime_status::NotDelivered::RateLimited)
        } else {
            Ok(crate::account_characters::Delivery::Accepted)
        }
    }
}
#[test]
fn t94_every_fixture_publisher_byte_exact_retry_ack_and_epoch_fence() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(async {
            use publisher::Publisher;
            use std::sync::{Arc, Mutex};
            let fixtures: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../docs/contracts/fixtures/world-event-guild-season/snapshots.json"
            ))?;
            for vector in fixtures["vectors"].as_array().ok_or("vectors")? {
                let raw = vector["wire"].as_str().ok_or("wire")?;
                let s = Snapshot::decode(raw)?;
                let (family, epoch, revision) = match &s {
                    Snapshot::Event(s) => (
                        "world_event_outcome",
                        &s.projection_epoch,
                        &s.projection_revision,
                    ),
                    Snapshot::Season(s) => {
                        ("guild_season", &s.projection_epoch, &s.projection_revision)
                    }
                };
                let p = Publication {
                    id: "fixture".into(),
                    delivery_token: Some("fixture-lease".into()),
                    family: family.into(),
                    epoch: decimal(epoch, false)?,
                    revision: decimal(revision, false)?,
                    wire: raw.into(),
                    sha256: sha2::Sha256::digest(raw.as_bytes()).to_vec(),
                    audit: vec![],
                    audit_event_id: String::new(),
                    transaction_id: String::new(),
                };
                let mut producer = Publisher {
                    enabled: true,
                    clock: TestClock,
                    store: FixtureStore {
                        p,
                        acked: std::cell::Cell::new(false),
                    },
                    sink: FixtureSink {
                        wire: raw.into(),
                        attempts: vec![],
                        retry: true,
                    },
                    fence: Arc::new(Mutex::new(MemoryFence(0))),
                    family,
                    source_authority: "oteryn:game:primary".into(),
                };
                assert_eq!(
                    producer.step().await,
                    Err(crate::runtime_status::NotDelivered::RateLimited)
                );
                assert!(!producer.store.acked.get());
                assert_eq!(producer.step().await, Ok(true));
                assert_eq!(producer.step().await, Ok(false));
                assert_eq!(producer.sink.attempts, vec![raw, raw]);
                producer.store.acked.set(false);
                producer.fence.lock().map_err(|_| "fence")?.0 = producer.store.p.epoch + 1;
                assert!(producer.step().await.is_err());
                assert!(!producer.store.acked.get());
                assert_eq!(producer.sink.attempts.len(), 2);
                producer.enabled = false;
                assert!(producer.step().await.is_err());
            }
            Ok(())
        })
}

#[test]
fn t94_guild_and_challenge_cardinality_boundaries() -> TestResult {
    let Snapshot::Season(s) = fixture("tied_guilds_frozen")? else {
        return Err("season".into());
    };
    let mut record = s.season[0].clone();
    let standing = record.standings[0].clone();
    record.standings = (1..=1000)
        .map(|i| {
            let mut g = standing.clone();
            g.guild_id = format!("01890f4c-3b2a-7c01-8d11-{i:012x}");
            g.rank = i.to_string();
            g
        })
        .collect();
    assert!(record.validate().is_ok());
    let mut extra = standing;
    extra.guild_id = "01890f4c-3b2a-7c01-8d11-0000000003e9".into();
    extra.rank = "1001".into();
    record.standings.push(extra);
    assert!(record.validate().is_err());
    record.standings.clear();
    let challenge = record.challenges[0].clone();
    record.challenges = (1..=32)
        .map(|i| {
            let mut c = challenge.clone();
            c.challenge_id = format!("01890f4c-3b2a-7c01-8d11-{i:012x}");
            c
        })
        .collect();
    assert!(record.validate().is_ok());
    let mut extra = challenge;
    extra.challenge_id = "01890f4c-3b2a-7c01-8d11-000000000021".into();
    record.challenges.push(extra);
    assert!(record.validate().is_err());
    Ok(())
}

#[test]
fn t94_file_fence_rejects_a_stale_writer_after_another_instance_advances() -> TestResult {
    use crate::account_characters::EpochFence;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("oteryn-t94-fence-{}-{nonce}", std::process::id()));
    std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
    let path = directory.join("epoch");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    {
        use std::io::Write;
        (&file).write_all(b"1\n")?;
    }
    file.sync_all()?;
    drop(file);
    let mut stale = fence::LeagueEpochFenceFile::new(path.clone());
    let mut current = fence::LeagueEpochFenceFile::new(path);
    assert_eq!(stale.read().map_err(|_| "read")?, 1);
    current.persist(5).map_err(|_| "advance")?;
    assert!(
        stale.persist(2).is_err(),
        "a stale process must never lower the nonrollback fence"
    );
    assert_eq!(current.read().map_err(|_| "read current")?, 5);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
