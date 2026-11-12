use super::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Peer {
    replies: VecDeque<Result<Vec<u8>, Failure>>,
    sent: Vec<(Operation, Vec<u8>)>,
}
impl JournalPeer for Peer {
    async fn exchange(&mut self, op: Operation, b: &[u8]) -> Result<Vec<u8>, Failure> {
        self.sent.push((op, b.to_vec()));
        self.replies.pop_front().ok_or(Failure::Quarantine)?
    }
}
struct Store {
    after_calls: Cell<usize>,
    record: Vec<u8>,
    acks: RefCell<Vec<String>>,
    live: Cell<bool>,
    lose_after_cursor: Cell<bool>,
}
impl JournalOutbox for Store {
    async fn current(&self, _: &ReadGameJournalCursorV1) -> Result<bool, SourceError> {
        Ok(self.live.get())
    }
    async fn first_for_replay(
        &self,
        _: &ReadGameJournalCursorV1,
    ) -> Result<Option<StoredPublication>, SourceError> {
        let record = PublishGameJournalRecordV1::decode(&self.record)?;
        if record.source_revision != "1" {
            return Err(SourceError::InvalidInput);
        }
        Ok(Some(StoredPublication {
            bytes: self.record.clone(),
            sha256: Sha256::digest(&self.record).into(),
        }))
    }
    async fn after_cursor(
        &self,
        _: &ReadGameJournalCursorV1,
        c: &ReadGameJournalCursorV1Response,
    ) -> Result<Option<StoredPublication>, SourceError> {
        self.after_calls.set(self.after_calls.get() + 1);
        if self.lose_after_cursor.get() {
            self.live.set(false);
        }
        let record = PublishGameJournalRecordV1::decode(&self.record)?;
        if c.after_source_revision == "0" {
            return Ok(Some(StoredPublication {
                bytes: self.record.clone(),
                sha256: Sha256::digest(&self.record).into(),
            }));
        }
        if c.after_source_revision != record.source_revision
            || c.last_record_id.as_deref() != Some(record.record_id.as_str())
        {
            return Err(SourceError::InvalidInput);
        }
        self.acks.borrow_mut().push(record.record_id);
        Ok(None)
    }
    async fn acknowledge(
        &self,
        _: &ReadGameJournalCursorV1,
        r: &PublishGameJournalRecordV1,
    ) -> Result<(), SourceError> {
        self.acks.borrow_mut().push(r.record_id.clone());
        Ok(())
    }
}
fn fixture() -> Result<(ReadGameJournalCursorV1, Store), Box<dyn std::error::Error>> {
    let text = include_str!("../publisher_fixture.json");
    let r = PublishGameJournalRecordV1::decode(text.as_bytes())?;
    let s = ReadGameJournalCursorV1 {
        contract_version: 1,
        operation: "ReadGameJournalCursorV1".into(),
        source_authority: r.source_authority,
        account_id: r.account_id,
        world_id: r.world_id,
        character_id: r.character_id,
        character_revision: r.character_revision,
        from_projection_revision: r.from_projection_revision,
        to_projection_revision: r.to_projection_revision,
        projection_epoch: r.projection_epoch,
        family: r.family,
        recovery_generation: r.recovery_generation,
    };
    Ok((
        s,
        Store {
            after_calls: Cell::new(0),
            record: text.as_bytes().to_vec(),
            acks: RefCell::new(Vec::new()),
            live: Cell::new(true),
            lose_after_cursor: Cell::new(false),
        },
    ))
}
#[test]
fn journal_publisher_ack_gap_replay_lost_ack_and_default_off() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (stream,store)=fixture()?;
        let zero=br#"{"contract_version":1,"after_source_revision":"0","last_record_id":null}"#.to_vec();
        let accepted=PublishGameJournalRecordV1Response::BYTES.to_vec();
        let peer=Peer{replies:VecDeque::from([Ok(zero.clone()),Err(Failure::Conflict),Ok(zero.clone()),Ok(accepted)]),sent:Vec::new()};
        let mut p=Publisher::new(JournalExportConfig::default(),peer,stream.source_authority.clone())?;
        assert_eq!(p.step(&stream,&store).await,Step::Disabled);assert!(p.peer.sent.is_empty());
        p.config.game_journal_items_export_enabled=true;
        assert!(matches!(p.step(&stream,&store).await,Step::RetryAfter(d) if (1..=10).contains(&d.as_secs())));
        assert!(store.acks.borrow().is_empty());
        assert_eq!(p.step(&stream,&store).await,Step::Accepted);
        assert_eq!(p.peer.sent[1].1,p.peer.sent[3].1);
        assert_eq!(store.acks.borrow().len(),1);
        let missing:JournalExportConfig=serde_json::from_str("{}")?;
        assert!(!missing.game_journal_items_export_enabled);
        assert!(!missing.journal_progress_publish_enabled);
        let id=PublishGameJournalRecordV1::decode(&store.record)?.record_id;
        let one=ReadGameJournalCursorV1Response{contract_version:1,after_source_revision:"1".into(),last_record_id:Some(id)}.encode()?;
        p.peer.replies=VecDeque::from([Ok(zero),Err(Failure::Retry(Some(Duration::from_secs(999)))),Ok(one)]);
        store.acks.borrow_mut().clear();
        assert_eq!(p.step(&stream,&store).await,Step::RetryAfter(Duration::from_secs(999)));
        assert!(store.acks.borrow().is_empty());
        assert_eq!(p.step(&stream,&store).await,Step::Idle);
        assert_eq!(store.acks.borrow().len(),1);
        Ok(())
    })
}

#[test]
fn journal_publisher_unknown_cursor_bootstraps_from_retained_first_record() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (stream, store) = fixture()?;
            let peer = Peer {
                replies: VecDeque::from([
                    Err(Failure::Conflict),
                    Ok(PublishGameJournalRecordV1Response::BYTES.to_vec()),
                ]),
                sent: Vec::new(),
            };
            let mut publisher = Publisher::new(
                JournalExportConfig {
                    game_journal_items_export_enabled: true,
                    ..Default::default()
                },
                peer,
                stream.source_authority.clone(),
            )?;
            assert_eq!(publisher.step(&stream, &store).await, Step::Accepted);
            assert_eq!(publisher.peer.sent[1].1, store.record);
            assert_eq!(store.acks.borrow().len(), 1);
            assert_eq!(store.after_calls.get(), 0);
            Ok(())
        })
}

#[test]
fn journal_publisher_ambiguity_authority_backoff_and_diagnostic() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
 let (stream,store)=fixture()?;
 let zero=br#"{"contract_version":1,"after_source_revision":"0","last_record_id":null}"#.to_vec();
 let peer=Peer{replies:VecDeque::from([Ok(b"bad cursor".to_vec()),Ok(zero.clone()),Ok(b"bad ACK".to_vec()),Ok(zero.clone()),Ok(PublishGameJournalRecordV1Response::BYTES.to_vec())]),sent:vec![]};
 let mut p=Publisher::new(JournalExportConfig{game_journal_items_export_enabled:true,..Default::default()},peer,stream.source_authority.clone())?;
 assert!(matches!(p.step(&stream,&store).await,Step::RetryAfter(_)));
 assert!(matches!(p.step(&stream,&store).await,Step::RetryAfter(_)));assert!(store.acks.borrow().is_empty());
 assert_eq!(p.step(&stream,&store).await,Step::Accepted);
 assert_eq!(p.peer.sent.iter().filter(|(op,_)|*op==Operation::ReadGameJournalCursorV1).count(),3);
 store.lose_after_cursor.set(true);store.acks.borrow_mut().clear();
 p.peer.replies=VecDeque::from([Ok(zero)]);let before=p.peer.sent.len();
 assert_eq!(p.step(&stream,&store).await,Step::Quarantined);assert_eq!(p.peer.sent.len(),before+1);assert!(store.acks.borrow().is_empty());
 p.attempt=0;
 for (lo,hi) in [(1,1),(1,2),(2,4),(4,8),(5,10),(5,10)] {
 assert!(matches!(p.retry(b"stream",None),Step::RetryAfter(d) if d>=Duration::from_secs(lo)&&d<=Duration::from_secs(hi)));
 }
 assert!(matches!(p.retry(b"stream",Some(Duration::from_secs(999))),Step::RetryAfter(d) if d==Duration::from_secs(999)));
 let old=tokio::time::Instant::now()-Duration::from_secs(61);p.last_progress=old;p.last_diagnostic=old;
 p.retry(b"stream",None);assert!(p.last_diagnostic>old);let diagnostic=p.last_diagnostic;p.retry(b"stream",None);assert_eq!(p.last_diagnostic,diagnostic);
 Ok(())
 })
}

#[test]
fn journal_publisher_http_ambiguity_classification() {
    for status in [201, 204, 301, 404, 429, 500, 501, 502, 503, 504, 599] {
        assert!(
            matches!(classify_response(status,vec![],Some(Duration::from_secs(999))),Err(Failure::Retry(Some(d))) if d==Duration::from_secs(999))
        );
    }
    for status in [400, 401, 413] {
        assert_eq!(
            classify_response(status, vec![], None),
            Err(Failure::Quarantine)
        );
    }
}
