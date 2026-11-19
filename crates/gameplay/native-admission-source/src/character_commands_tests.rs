use super::*;
use std::{
    collections::VecDeque,
    pin::pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

const AUTHORITY: &str = "oteryn:character-authority:primary";
const OP1: &str = "0192a000-0000-7000-8000-000000000001";

fn op(index: u32) -> String {
    format!("0192a000-0000-7000-8000-{index:012x}")
}

fn key(revision: u64, index: u32) -> Key {
    Key {
        source_revision: revision,
        operation_id: op(index),
    }
}

fn entry(revision: u64, index: u32) -> PendingEntry {
    PendingEntry {
        command: CREATE_COMMAND.into(),
        key: key(revision, index),
    }
}

fn page(entries: &[(&str, &str, &str)]) -> Vec<u8> {
    let items: Vec<String> = entries
        .iter()
        .map(|(c, o, r)| {
            format!(r#"{{"command":"{c}","operation_id":"{o}","source_revision":"{r}"}}"#)
        })
        .collect();
    format!(
        r#"{{"contract_version":1,"pending":[{}]}}"#,
        items.join(",")
    )
    .into_bytes()
}

#[test]
fn requests_match_the_contract_fixtures() {
    assert_eq!(
        encode_list(AUTHORITY, None).unwrap(),
        concat!(
            r#"{"contract_version":1,"operation":"ListPendingCharacterCommandsV1","#,
            r#""source_authority":"oteryn:character-authority:primary","#,
            r#""after_source_revision":"0","#,
            r#""after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":32}"#
        )
    );
    assert!(
        encode_list(AUTHORITY, Some(&key(17, 1)))
            .unwrap()
            .contains(r#""after_source_revision":"17","after_operation_id":"0192a000-0000-7000-8000-000000000001""#)
    );
    assert_eq!(
        encode_read(AUTHORITY, CREATE_COMMAND, OP1).unwrap(),
        concat!(
            r#"{"contract_version":1,"operation":"ReadCharacterCommandIntentV1","#,
            r#""source_authority":"oteryn:character-authority:primary","#,
            r#""issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","#,
            r#""command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001"}"#
        )
    );
}

#[test]
fn worst_case_requests_fit_their_bounds() {
    let authority = "a".repeat(128);
    let read = encode_read(&authority, &"C".repeat(64), OP1).unwrap();
    assert_eq!(read.len(), 496 - 128 + ISSUER_AUTHORITY.len());
    assert!(read.len() <= READ_REQUEST_BYTES);
    let list = encode_list(&authority, Some(&key(u64::MAX, 1))).unwrap();
    assert!(list.len() <= LIST_REQUEST_BYTES);
    assert!(encode_list("", None).is_err());
    assert!(encode_list("bad authority", None).is_err());
    assert!(encode_list(AUTHORITY, Some(&key(0, 1))).is_err());
    assert!(encode_read(AUTHORITY, "Create-Character", OP1).is_err());
    assert!(
        encode_read(
            AUTHORITY,
            CREATE_COMMAND,
            "00000000-0000-0000-0000-000000000000"
        )
        .is_err()
    );
}

#[test]
fn full_page_fixture_decodes_and_fits() {
    let rev = u64::MAX.to_string();
    let command = "C".repeat(64);
    let ops: Vec<String> = (1..=32).map(op).collect();
    let entries: Vec<(&str, &str, &str)> = ops
        .iter()
        .map(|o| (command.as_str(), o.as_str(), rev.as_str()))
        .collect();
    let raw = page(&entries);
    assert_eq!(raw.len(), 5602);
    assert_eq!(decode_pending(&raw, None).unwrap().len(), 32);
    let fixture = page(&[(CREATE_COMMAND, OP1, "17")]);
    assert_eq!(
        decode_pending(&fixture, None).unwrap(),
        vec![PendingEntry {
            command: CREATE_COMMAND.into(),
            key: key(17, 1)
        }]
    );
    assert!(
        decode_pending(br#"{"contract_version":1,"pending":[]}"#, None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn malformed_pages_are_refused() {
    let o1 = op(1);
    let o2 = op(2);
    let refused = |raw: &[u8], after: Option<&Key>| assert!(decode_pending(raw, after).is_err());
    // 33 entries.
    let ops: Vec<String> = (1..=33).map(op).collect();
    let entries: Vec<(&str, &str, &str)> = ops
        .iter()
        .map(|o| (CREATE_COMMAND, o.as_str(), "1"))
        .collect();
    refused(&page(&entries), None);
    // Order, duplicates and the cursor.
    refused(
        &page(&[(CREATE_COMMAND, &o2, "5"), (CREATE_COMMAND, &o1, "5")]),
        None,
    );
    refused(
        &page(&[(CREATE_COMMAND, &o1, "6"), (CREATE_COMMAND, &o1, "7")]),
        None,
    );
    refused(&page(&[(CREATE_COMMAND, &o1, "5")]), Some(&key(5, 1)));
    refused(&page(&[(CREATE_COMMAND, &o1, "4")]), Some(&key(5, 1)));
    assert!(decode_pending(&page(&[(CREATE_COMMAND, &o2, "5")]), Some(&key(5, 1))).is_ok());
    // Counters, identifiers and commands.
    for rev in ["0", "017", "18446744073709551616", "-1", ""] {
        refused(&page(&[(CREATE_COMMAND, &o1, rev)]), None);
    }
    refused(
        &page(&[(CREATE_COMMAND, "00000000-0000-0000-0000-000000000000", "1")]),
        None,
    );
    refused(
        &page(&[(CREATE_COMMAND, "0192A000-0000-7000-8000-000000000001", "1")]),
        None,
    );
    refused(
        &page(&[(CREATE_COMMAND, "0192a000-0000-4000-8000-000000000001", "1")]),
        None,
    );
    refused(&page(&[("", &o1, "1")]), None);
    refused(&page(&[(&"C".repeat(65), &o1, "1")]), None);
    refused(&page(&[("Create.Character", &o1, "1")]), None);
    // Grammar: whitespace, escapes, unknown, duplicate, missing and null members.
    refused(br#"{"contract_version":1, "pending":[]}"#, None);
    refused(
        br#"{"contract_version":1,"pending":[]}
"#,
        None,
    );
    refused(br#"{"contract_version":1,"pending":[{"command":"Create\u0043haracter","operation_id":"0192a000-0000-7000-8000-000000000001","source_revision":"1"}]}"#, None);
    refused(br#"{"contract_version":1,"pending":[],"extra":1}"#, None);
    refused(
        br#"{"contract_version":1,"contract_version":1,"pending":[]}"#,
        None,
    );
    refused(br#"{"contract_version":1}"#, None);
    refused(br#"{"contract_version":1,"pending":null}"#, None);
    refused(br#"{"contract_version":2,"pending":[]}"#, None);
    refused(&vec![b' '; LIST_RESPONSE_BYTES + 1], None);
}

#[test]
fn receipt_responses_and_statuses() {
    assert!(decode_receipt_response(br#"{"contract_version":1,"result":"accepted"}"#).is_ok());
    for raw in [
        &br#"{"contract_version":1,"result":"superseded"}"#[..],
        br#"{"contract_version":2,"result":"accepted"}"#,
        br#"{"contract_version":1,"result":"accepted","x":1}"#,
        br#"{"contract_version":1, "result":"accepted"}"#,
        b"",
    ] {
        assert!(decode_receipt_response(raw).is_err());
    }
    assert_eq!(classify(404, b""), Failure::NotFound);
    assert_eq!(classify(409, b""), Failure::Conflict);
    assert_eq!(classify(409, b"x"), Failure::InvalidResponse);
    assert_eq!(classify(429, b""), Failure::RateLimited);
    assert_eq!(classify(503, b""), Failure::Unavailable);
    assert_eq!(classify(500, b"x"), Failure::Unavailable);
    assert!(Failure::RateLimited.backs_off() && Failure::Unavailable.backs_off());
    assert!(!Failure::NotFound.backs_off());
    assert_eq!(
        response_bytes(Operation::ListPendingCharacterCommandsV1),
        Some(LIST_RESPONSE_BYTES)
    );
    assert_eq!(response_bytes(Operation::ReadAccountSecurityV1), None);
}

#[test]
fn cursor_pages_forward_and_restarts_after_a_short_page() {
    let t = Duration::from_secs(100);
    let mut cursor = Cursor::new(t);
    assert_eq!(cursor.next_request(t), None);
    let full: Vec<_> = (1..=32).map(|i| entry(u64::from(i), i)).collect();
    cursor.advance(&full);
    assert_eq!(cursor.next_request(t), Some(key(32, 32)));
    cursor.advance(&[entry(40, 40)]);
    assert_eq!(cursor.next_request(t), None);
}

#[test]
fn rescan_after_eight_full_pages_returns_to_the_reached_key() {
    let t = Duration::from_secs(100);
    let mut cursor = Cursor::new(t);
    cursor.next_request(t);
    let mut base = 0;
    for _ in 0..RESCAN_FULL_PAGES {
        let full: Vec<_> = (1..=32).map(|i| entry(base + u64::from(i), i)).collect();
        cursor.advance(&full);
        base += 32;
        cursor.next_request(t);
    }
    // The ninth request is a rescan from the start.
    assert!(cursor.rescanning());
    // A key committed behind the cursor is unseen; known keys are not.
    for i in 1..=20 {
        cursor.settle(key(u64::from(i), i), Settle::Done, t);
    }
    let behind: Vec<_> = (1..=32).map(|i| entry(u64::from(i), i)).collect();
    assert!(!cursor.observe(&behind));
    cursor.advance(&behind);
    assert!(cursor.rescanning());
    assert_eq!(cursor.next_request(t), Some(key(32, 32)));
    // Passing the reached key ends the rescan.
    let reached = base;
    let past: Vec<_> = (0u32..32)
        .map(|i| entry(reached + u64::from(i), 100 + i))
        .collect();
    cursor.advance(&past);
    assert!(!cursor.rescanning());
}

#[test]
fn rescan_after_thirty_seconds_and_alarm_on_unseen_keys() {
    let t = Duration::from_secs(100);
    let mut cursor = Cursor::new(t);
    cursor.next_request(t);
    let full: Vec<_> = (1..=32).map(|i| entry(1000 + u64::from(i), i)).collect();
    cursor.advance(&full);
    assert_eq!(
        cursor.next_request(t + Duration::from_secs(29)),
        Some(key(1032, 32))
    );
    assert!(!cursor.rescanning());
    assert_eq!(cursor.next_request(t + RESCAN_PERIOD), None);
    assert!(cursor.rescanning());
    let unseen: Vec<_> = (1..=17).map(|i| entry(u64::from(i), 500 + i)).collect();
    assert!(cursor.observe(&unseen));
    // The alarm is raised once per rescan.
    assert!(!cursor.observe(&unseen));
}

#[test]
fn rescan_alarms_past_192_listed_entries() {
    let t = Duration::from_secs(100);
    let mut cursor = Cursor::new(t);
    cursor.next_request(t);
    cursor.advance(
        &(1..=32)
            .map(|i| entry(10_000 + u64::from(i), i))
            .collect::<Vec<_>>(),
    );
    cursor.next_request(t + RESCAN_PERIOD);
    let mut alarms = 0;
    for p in 0..7_u32 {
        let entries: Vec<_> = (0..32)
            .map(|i| entry(u64::from(p * 32 + i + 1), p * 32 + i + 1))
            .collect();
        for e in &entries {
            cursor.settle(e.key.clone(), Settle::Done, t);
        }
        alarms += usize::from(cursor.observe(&entries));
        cursor.advance(&entries);
    }
    assert_eq!(alarms, 1);
}

#[test]
fn known_keys_back_off_and_leave_once_unlisted() {
    let t = Duration::from_secs(100);
    let mut cursor = Cursor::new(t);
    cursor.next_request(t);
    let k = key(1, 1);
    assert!(cursor.due(&k, t));
    cursor.settle(k.clone(), Settle::Retry, t);
    assert!(!cursor.due(&k, t));
    assert!(cursor.due(&k, t + Duration::from_secs(2)));
    cursor.settle(k.clone(), Settle::Retry, t);
    assert!(!cursor.due(&k, t + Duration::from_secs(2)));
    for _ in 0..10 {
        cursor.settle(k.clone(), Settle::Retry, t);
    }
    assert!(cursor.due(&k, t + MAX_BACKOFF));
    cursor.settle(k.clone(), Settle::Done, t);
    assert!(!cursor.due(&k, t + MAX_BACKOFF));
    assert!(cursor.due(&k, t + SETTLED_RECHECK));
    // A pass that no longer lists the key forgets it.
    cursor.advance(&[]);
    cursor.next_request(t);
    cursor.advance(&[]);
    assert!(cursor.due(&k, t));
    // A refused receipt stays stopped while listed, and is forgotten once a pass omits it.
    cursor.settle(k.clone(), Settle::Stopped, t);
    assert!(!cursor.due(&k, t + SETTLED_RECHECK * 1000));
    let listed = PendingEntry {
        command: CREATE_COMMAND.into(),
        key: k.clone(),
    };
    cursor.next_request(t);
    cursor.advance(std::slice::from_ref(&listed));
    assert!(!cursor.due(&k, t + SETTLED_RECHECK * 1000));
    cursor.next_request(t);
    cursor.advance(&[]);
    assert!(cursor.due(&k, t));
}

#[test]
fn the_known_key_bound_keeps_stopped_keys() {
    let t = Duration::from_secs(100);
    let mut cursor = Cursor::new(t);
    let stopped = key(1, 0);
    cursor.settle(stopped.clone(), Settle::Stopped, t);
    for index in 1..=u32::try_from(KNOWN_KEYS).unwrap() {
        cursor.settle(key(2, index), Settle::Done, t);
    }
    assert!(!cursor.due(&stopped, t + SETTLED_RECHECK * 1000));
    // The lowest settled key gave way instead.
    assert!(cursor.due(&key(2, 1), t));
    assert!(!cursor.due(&key(2, 2), t));
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

type Script = Arc<Mutex<VecDeque<Result<Vec<u8>, Failure>>>>;

struct Source {
    script: Script,
    sent: Arc<Mutex<Vec<(Operation, String)>>>,
}

impl CommandSource for Source {
    fn exchange(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = Result<Vec<u8>, Failure>> + Send {
        self.sent.lock().unwrap().push((operation, body.into()));
        let next = self.script.lock().unwrap().pop_front();
        async move {
            match next {
                Some(answer) => answer,
                None => std::future::pending().await,
            }
        }
    }
}

struct Authority {
    decisions: VecDeque<Decision>,
    seen: Arc<Mutex<Vec<(String, String)>>>,
    checked_create: Option<(String, Vec<u8>)>,
}

impl Decider for Authority {
    fn decide(
        &mut self,
        command: &str,
        operation_id: &str,
        input: Input<'_>,
    ) -> impl Future<Output = Decision> + Send {
        let kind = match input {
            Input::Intent(raw) => String::from_utf8_lossy(raw).into_owned(),
            Input::ReadConflict => "read_conflict".into(),
            Input::Unsupported => "unsupported".into(),
            Input::Oversized => "oversized".into(),
        };
        self.seen.lock().unwrap().push((command.into(), kind));
        let decision = self.decisions.pop_front().unwrap_or(Decision::Transient);
        if command == CREATE_COMMAND
            && let Decision::Publish {
                receipt,
                result_code,
            } = &decision
            && result_code == "CHAR_CMD_OK"
        {
            self.checked_create = Some((operation_id.into(), receipt.clone()));
        }
        std::future::ready(decision)
    }

    async fn sendable_create(&mut self, operation: &str) -> Result<CreateSendability, SourceError> {
        let (id, bytes) = self.checked_create.take().unwrap();
        assert_eq!(id, operation);
        Ok(CreateSendability::Ready(String::from_utf8(bytes).unwrap()))
    }

    async fn stop_retained_create_receipt(&mut self, _: &str) -> Result<(), SourceError> {
        Ok(())
    }
}

fn publish(code: &str) -> Decision {
    Decision::Publish {
        receipt: format!("receipt-{code}").into_bytes(),
        result_code: code.into(),
    }
}

/// Exchanges sent, decisions asked (command, operation) and logged events.
type Driven = (Vec<(Operation, String)>, Vec<(String, String)>, Vec<String>);

/// Drive the consumer until its scripted source runs dry.
fn drive(script: Vec<Result<Vec<u8>, Failure>>, decisions: Vec<Decision>) -> Driven {
    // Tests run in parallel; each polls its consumer on its own thread.
    thread_local!(static LOG: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) });
    LOG.with_borrow_mut(Vec::clear);
    let sent = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let consumer = Consumer::new(
        Clock::default(),
        Source {
            script: Arc::new(Mutex::new(script.into())),
            sent: sent.clone(),
        },
        Authority {
            decisions: decisions.into(),
            seen: seen.clone(),
            checked_create: None,
        },
        AUTHORITY.into(),
        |line| LOG.with_borrow_mut(|log| log.push(line.into())),
    );
    let mut run = pin!(consumer.run());
    assert!(matches!(
        run.as_mut().poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    let log = LOG.with_borrow(Clone::clone);
    let sent = sent.lock().unwrap().clone();
    let seen = seen.lock().unwrap().clone();
    (sent, seen, log)
}

const ACCEPTED: &[u8] = br#"{"contract_version":1,"result":"accepted"}"#;

#[test]
fn account_deletion_receipt_uses_v2_bound_on_the_existing_character_path() {
    assert_eq!(
        Operation::PublishAccountClosureCommandReceiptV2.path(),
        Operation::PublishCharacterCommandReceiptV1.path(),
    );
    assert_eq!(
        Operation::PublishCharacterCommandReceiptV1.request_bytes_max(),
        1024
    );
    assert_eq!(
        Operation::PublishAccountClosureCommandReceiptV2.request_bytes_max(),
        2048
    );
    for command in ["CloseAccount", "ResolveAccountClosure"] {
        let (sent, _, _) = drive(
            vec![
                Ok(page(&[(command, OP1, "17")])),
                Ok(b"immutable-intent".to_vec()),
                Ok(br#"{"contract_version":2,"result":"accepted"}"#.to_vec()),
                Ok(page(&[])),
            ],
            vec![Decision::Publish {
                receipt: vec![b'x'; 2048],
                result_code: "CHAR_CMD_OK".into(),
            }],
        );
        assert_eq!(sent[2].0, Operation::PublishAccountClosureCommandReceiptV2);
        assert_eq!(sent[2].1.len(), 2048);
    }
}

#[test]
fn account_deletion_consumer_reads_intents_and_leaves_pending_without_receipts() {
    for command in ["CloseAccount", "ResolveAccountClosure"] {
        let (sent, seen, _) = drive(
            vec![
                Ok(page(&[(command, OP1, "17")])),
                Ok(b"immutable-intent".to_vec()),
            ],
            vec![Decision::Transient],
        );
        assert_eq!(seen, [(command.into(), "immutable-intent".into())]);
        assert_eq!(sent[1].0, Operation::ReadCharacterCommandIntentV1);
        assert!(sent.iter().all(|(operation, _)| !matches!(
            operation,
            Operation::PublishCharacterCommandReceiptV1
                | Operation::PublishAccountClosureCommandReceiptV2
        )));
    }
}

#[test]
fn consumer_reads_decides_and_publishes_the_stored_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/fixtures/bazaar/decisions.json"
    ))?;
    let vectors = fixture["vectors"]
        .as_array()
        .ok_or("missing Bazaar vectors")?;
    let wire = |name: &str| {
        vectors
            .iter()
            .find(|vector| vector["name"].as_str() == Some(name))
            .and_then(|vector| vector["wire"].as_str())
            .ok_or("missing settlement fixture")
    };
    let transfer_intent = wire("character_settlement_intent")?;
    let transfer_receipt = wire("character_settlement_receipt")?;
    let transfer: serde_json::Value = serde_json::from_str(transfer_intent)?;
    let transfer_operation = transfer["operation_id"]
        .as_str()
        .ok_or("missing transfer operation")?;
    let transfer_revision = transfer["source_revision"]
        .as_str()
        .ok_or("missing transfer revision")?;
    let list = page(&[
        (CREATE_COMMAND, OP1, "17"),
        (
            "TransferCharacterOwnership",
            transfer_operation,
            transfer_revision,
        ),
    ]);
    let (sent, seen, log) = drive(
        vec![
            Ok(list),
            Ok(b"intent".to_vec()),
            Ok(ACCEPTED.to_vec()),
            Ok(transfer_intent.as_bytes().to_vec()),
            Ok(ACCEPTED.to_vec()),
        ],
        vec![
            publish("CHAR_CMD_OK"),
            Decision::Publish {
                receipt: transfer_receipt.as_bytes().to_vec(),
                result_code: "CHAR_CMD_OK".to_owned(),
            },
        ],
    );
    let operations: Vec<_> = sent.iter().map(|(o, _)| *o).collect();
    assert_eq!(
        operations,
        [
            Operation::ListPendingCharacterCommandsV1,
            Operation::ReadCharacterCommandIntentV1,
            Operation::PublishCharacterCommandReceiptV1,
            Operation::ReadCharacterCommandIntentV1,
            Operation::PublishCharacterCommandReceiptV1,
            Operation::ListPendingCharacterCommandsV1,
        ]
    );
    assert_eq!(sent[2].1, "receipt-CHAR_CMD_OK");
    assert_eq!(
        sent[3].1,
        encode_read(AUTHORITY, "TransferCharacterOwnership", transfer_operation)?
    );
    assert_eq!(
        sent[4].1, transfer_receipt,
        "publish the stored v1 receipt byte-for-byte"
    );
    assert_eq!(
        seen,
        [
            (CREATE_COMMAND.to_owned(), "intent".to_owned()),
            (
                "TransferCharacterOwnership".to_owned(),
                transfer_intent.to_owned()
            ),
        ]
    );
    // §8: logs name the operation, command and code, never the identifiers or bytes.
    assert!(
        log.iter()
            .any(|line| line.contains("result=CHAR_CMD_OK receipt=accepted"))
    );
    assert!(log.iter().all(|line| !line.contains(OP1)
        && !line.contains("intent")
        && !line.contains(transfer_operation)
        && !line.contains(transfer_intent)));
    Ok(())
}

#[test]
fn read_conflict_and_missing_intent() {
    let o2 = op(2);
    let list = page(&[(CREATE_COMMAND, OP1, "17"), (CREATE_COMMAND, &o2, "18")]);
    let (sent, seen, log) = drive(
        vec![
            Ok(list),
            Err(Failure::Conflict),
            Ok(ACCEPTED.to_vec()),
            Err(Failure::NotFound),
        ],
        vec![publish("CHAR_CMD_OPERATION_CONFLICT")],
    );
    assert_eq!(
        seen,
        [(CREATE_COMMAND.to_owned(), "read_conflict".to_owned())]
    );
    assert_eq!(sent.len(), 5);
    assert!(log.iter().any(|l| l.contains("alarm=intent_missing")));
    assert!(log.iter().any(|l| {
        l.contains("result=CHAR_CMD_OPERATION_CONFLICT receipt=accepted alarm=operation_conflict")
    }));
}

#[test]
fn oversized_intent_is_decided_not_retried() {
    let (sent, seen, _) = drive(
        vec![
            Ok(page(&[(CREATE_COMMAND, OP1, "17")])),
            Err(Failure::Oversized),
            Ok(ACCEPTED.to_vec()),
        ],
        vec![publish("CHAR_CMD_INTENT_INVALID")],
    );
    assert_eq!(seen, [(CREATE_COMMAND.to_owned(), "oversized".to_owned())]);
    assert_eq!(sent.len(), 4);
    assert!(!Failure::Oversized.backs_off());
}

#[test]
fn receipt_refusal_alarms_and_transient_failures_retry() {
    let list = page(&[(CREATE_COMMAND, OP1, "17")]);
    let (sent, _, log) = drive(
        vec![
            Ok(list.clone()),
            Ok(b"intent".to_vec()),
            Err(Failure::Conflict),
            Ok(list),
        ],
        vec![publish("CHAR_CMD_OK")],
    );
    assert!(log.iter().any(|l| l.contains("alarm=receipt_refused")));
    // The settled key is not read again on the next pass.
    assert_eq!(sent.len(), 5);
    let list = page(&[(CREATE_COMMAND, OP1, "17")]);
    let (_, seen, log) = drive(
        vec![Ok(list), Ok(b"intent".to_vec())],
        vec![Decision::Transient],
    );
    assert_eq!(seen.len(), 1);
    assert!(log.iter().any(|l| l.contains("result=transient")));
    let (_, _, log) = drive(
        vec![
            Ok(page(&[(CREATE_COMMAND, OP1, "17")])),
            Ok(b"intent".to_vec()),
        ],
        vec![Decision::Conflict],
    );
    assert!(log.iter().any(|l| l.contains("alarm=receipt_conflict")));
}

#[test]
fn throttling_backs_off_and_relists_the_same_page() {
    let list = page(&[(CREATE_COMMAND, OP1, "17")]);
    let (sent, _, _) = drive(
        vec![
            Err(Failure::RateLimited),
            Ok(list),
            Err(Failure::Unavailable),
        ],
        vec![],
    );
    let lists: Vec<_> = sent
        .iter()
        .filter(|(o, _)| *o == Operation::ListPendingCharacterCommandsV1)
        .map(|(_, b)| b.clone())
        .collect();
    assert_eq!(lists.len(), 3);
    assert!(
        lists
            .iter()
            .all(|b| b.contains(r#""after_source_revision":"0""#))
    );
}

#[test]
fn deletion_consumer_invalidates_then_observes_independent_lifecycle_before_deciding() {
    struct LifecycleAuthority {
        events: Arc<Mutex<Vec<&'static str>>>,
        observed: bool,
        receipt: Vec<u8>,
    }
    impl Decider for LifecycleAuthority {
        fn lifecycle_recovery_generation(&self) -> Option<u64> {
            Some(3)
        }
        async fn begin_closure_lifecycle_read(
            &mut self,
            _: &super::super::account_closure::LifecycleRequest,
        ) {
            self.observed = false;
            self.events.lock().unwrap().push("invalidate");
        }
        async fn observe_closure_lifecycle(
            &mut self,
            request: &super::super::account_closure::LifecycleRequest,
            raw: &[u8],
        ) {
            self.observed = super::super::account_closure::decode_lifecycle(request, raw).is_ok();
            self.events.lock().unwrap().push("observe");
        }
        async fn decide(&mut self, command: &str, _: &str, _: Input<'_>) -> Decision {
            assert_eq!(command, CLOSE_ACCOUNT_COMMAND);
            self.events.lock().unwrap().push("decide");
            if self.observed {
                Decision::Publish {
                    receipt: self.receipt.clone(),
                    result_code: "CHAR_CMD_OK".into(),
                }
            } else {
                Decision::Transient
            }
        }
        async fn acknowledge_receipt(&mut self, _: &str, _: &str) {
            self.events.lock().unwrap().push("ack");
        }
    }
    let closure: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/fixtures/account-deletion/closure.json"
    ))
    .unwrap();
    let lifecycle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/contracts/fixtures/account-deletion/lifecycle.json"
    ))
    .unwrap();
    let intent = closure["vectors"][0]["wire"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let receipt = closure["vectors"][1]["wire"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let parsed: serde_json::Value = serde_json::from_slice(&intent).unwrap();
    for available in [false, true] {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut script = VecDeque::from([
            Ok(intent.clone()),
            if available {
                Ok(lifecycle["vectors"][1]["wire"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec())
            } else {
                Err(Failure::Unavailable)
            },
        ]);
        if available {
            script.push_back(Ok(br#"{"contract_version":2,"result":"accepted"}"#.to_vec()));
        }
        let mut consumer = Consumer::new(
            Clock::default(),
            Source {
                script: Arc::new(Mutex::new(script)),
                sent: sent.clone(),
            },
            LifecycleAuthority {
                events: events.clone(),
                observed: true,
                receipt: receipt.clone(),
            },
            "oteryn:character-authority:primary".into(),
            |_| {},
        );
        let entry = PendingEntry {
            command: CLOSE_ACCOUNT_COMMAND.into(),
            key: Key {
                source_revision: 30,
                operation_id: parsed["operation_id"].as_str().unwrap().into(),
            },
        };
        let mut run = pin!(consumer.process(&entry));
        assert!(matches!(
            run.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(_)
        ));
        if available {
            assert_eq!(
                *events.lock().unwrap(),
                ["invalidate", "observe", "decide", "ack"]
            );
            assert_eq!(sent.lock().unwrap()[2].1.as_bytes(), receipt);
        } else {
            assert_eq!(*events.lock().unwrap(), ["invalidate", "decide"]);
            assert_eq!(sent.lock().unwrap().len(), 2);
        }
        assert_eq!(
            sent.lock().unwrap()[1].0,
            Operation::ReadAccountClosureLifecycleV1
        );
        assert_eq!(
            sent.lock().unwrap()[1].1,
            lifecycle["vectors"][0]["wire"].as_str().unwrap()
        );
    }
}

#[test]
fn closure_publisher_is_independent_of_pending_list_and_quarantines_conflicts() {
    struct Publishing {
        publication: Option<super::super::account_closure::Publication>,
        events: Arc<Mutex<Vec<&'static str>>>,
    }
    impl Decider for Publishing {
        async fn decide(&mut self, _: &str, _: &str, _: Input<'_>) -> Decision {
            self.events.lock().unwrap().push("unexpected-decision");
            Decision::Transient
        }
        async fn next_closure_publication(
            &mut self,
        ) -> Option<super::super::account_closure::Publication> {
            match self.publication.take() {
                Some(p) => Some(p),
                None => std::future::pending().await,
            }
        }
        async fn acknowledge_closure_publication(
            &mut self,
            _: &super::super::account_closure::Publication,
        ) {
            self.events.lock().unwrap().push("ack");
        }
        async fn quarantine_closure_publication(
            &mut self,
            _: &super::super::account_closure::Publication,
        ) {
            self.events.lock().unwrap().push("quarantine");
        }
    }
    for (operation, refusal) in [
        (Operation::PublishAccountClosureV1, None),
        (Operation::PublishAccountClosureV1, Some(Failure::Conflict)),
        (
            Operation::PublishAccountClosureCommandReceiptV2,
            Some(Failure::NotFound),
        ),
    ] {
        let conflict = refusal.is_some();
        let sent = Arc::new(Mutex::new(Vec::new()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let publication = super::super::account_closure::Publication {
            operation,
            body: "stored-canonical-snapshot".into(),
        };
        let answer = if let Some(failure) = refusal {
            Err(failure)
        } else {
            Ok(br#"{"contract_version":2,"result":"accepted"}"#.to_vec())
        };
        let publisher = run_account_closure_publications(
            Clock::default(),
            Source {
                script: Arc::new(Mutex::new(VecDeque::from([answer]))),
                sent: sent.clone(),
            },
            Publishing {
                publication: Some(publication.clone()),
                events: events.clone(),
            },
            |_| {},
        );
        let mut run = pin!(publisher);
        assert!(matches!(
            run.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        assert_eq!(
            *sent.lock().unwrap(),
            [(publication.operation, publication.body)]
        );
        assert_eq!(
            *events.lock().unwrap(),
            [if conflict { "quarantine" } else { "ack" }]
        );
    }
}

#[derive(Default)]
struct RetainedCreateStore {
    clock: Clock,
    stopped: Vec<String>,
    reads_at: Vec<Duration>,
    stop_attempts_at: Vec<Duration>,
    fail_next_stop: bool,
    continuous: bool,
    incarnation_lost: bool,
    unavailable_operation: Option<String>,
    scan_failures: BTreeSet<i64>,
    late_uuid: Option<(u32, String)>,
    removed: BTreeSet<String>,
    replacements: BTreeMap<String, Vec<u8>>,
    generated_rows: u32,
    invalid_generated: bool,
}

struct RetainedCreates {
    store: Arc<Mutex<RetainedCreateStore>>,
    reads: usize,
    read_limit: usize,
    source_authority: &'static str,
}

impl RetainedCreates {
    async fn count_read(&mut self) {
        self.reads += 1;
        if self.reads > self.read_limit {
            std::future::pending::<()>().await;
        }
        let mut store = self.store.lock().unwrap();
        let now = store.clock.elapsed();
        store.reads_at.push(now);
    }
}

impl RetainedCreateScan for RetainedCreates {
    async fn scan_retained_creates(
        &mut self,
        after: i64,
        through: Option<i64>,
    ) -> Result<RetainedCreatePage, SourceError> {
        let page = {
            let mut store = self.store.lock().unwrap();
            if store.incarnation_lost || store.scan_failures.remove(&after) {
                return Err(SourceError::Unavailable);
            }
            let through = through.unwrap_or_else(|| {
                if store.generated_rows > 0 {
                    i64::from(store.generated_rows)
                } else if store.continuous {
                    i64::from(u32::MAX)
                } else {
                    2
                }
            });
            let keys = ((after + 1)..=through)
                .map(|scan_key| {
                    let index = u32::try_from(scan_key).unwrap();
                    let operation_id = store
                        .late_uuid
                        .as_ref()
                        .filter(|(row, _)| *row == index)
                        .map_or_else(|| op(index), |(_, id)| id.clone());
                    RetainedCreateKey {
                        scan_key,
                        operation_id,
                    }
                })
                .filter(|key| {
                    !store.stopped.contains(&key.operation_id)
                        && !store.removed.contains(&key.operation_id)
                })
                .take(2)
                .collect::<Vec<_>>();
            RetainedCreatePage { through, keys }
        };
        if page.keys.is_empty() {
            self.count_read().await;
        }
        Ok(page)
    }

    async fn sendable_create(&mut self, operation: &str) -> Result<CreateSendability, SourceError> {
        self.count_read().await;
        let mut store = self.store.lock().unwrap();
        if store.incarnation_lost || store.unavailable_operation.as_deref() == Some(operation) {
            store.unavailable_operation = None;
            return Err(SourceError::Unavailable);
        }
        if store.stopped.iter().any(|id| id == operation) || store.removed.contains(operation) {
            return Ok(CreateSendability::StoppedOrMissing);
        }
        let body = if store.generated_rows > 0 {
            if store.invalid_generated && operation != op(store.generated_rows) {
                return Ok(CreateSendability::Invalid);
            }
            operation.to_owned()
        } else if operation == OP1 {
            if self.source_authority != AUTHORITY {
                return Ok(CreateSendability::Invalid);
            }
            "receipt-a".into()
        } else if operation == op(2) {
            "receipt-b".into()
        } else {
            assert!(store.continuous);
            "receipt-new".into()
        };
        Ok(CreateSendability::Ready(
            store
                .replacements
                .get(operation)
                .map_or(body, |value| String::from_utf8(value.clone()).unwrap()),
        ))
    }

    async fn stop_retained_create_receipt(
        &mut self,
        operation_id: &str,
    ) -> Result<(), SourceError> {
        let mut store = self.store.lock().unwrap();
        let now = store.clock.elapsed();
        store.stop_attempts_at.push(now);
        if std::mem::take(&mut store.fail_next_stop) {
            return Err(SourceError::Unavailable);
        }
        store.stopped.push(operation_id.into());
        Ok(())
    }
}

fn poll_retained_create_publisher(
    store: Arc<Mutex<RetainedCreateStore>>,
    read_limit: usize,
    source_authority: &'static str,
    script: VecDeque<Result<Vec<u8>, Failure>>,
    event: fn(&str),
) -> Vec<(Operation, String)> {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let clock = store.lock().unwrap().clock.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let _timer_context = runtime.enter();
    let mut run = pin!(run_retained_create_publications(
        clock,
        Source {
            script: Arc::new(Mutex::new(script)),
            sent: sent.clone(),
        },
        RetainedCreates {
            store,
            reads: 0,
            read_limit,
            source_authority,
        },
        event,
    ));
    assert!(
        run.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    sent.lock().unwrap().clone()
}

const OTHER_AUTHORITY: &str = "oteryn:character-authority:secondary";

fn retained_create_ack() -> Vec<u8> {
    br#"{"contract_version":1,"result":"accepted"}"#.to_vec()
}

#[test]
fn retained_create_refusal_alarms_and_does_not_block_later_receipts_after_restart() {
    thread_local! {
        static LOG: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let event = |event: &str| LOG.with_borrow_mut(|log| log.push(event.into()));
    let operation = Operation::PublishCharacterCommandReceiptV1;
    for (refusal, label) in [
        (classify(409, b""), "conflict"),
        (classify(404, b""), "not_found"),
    ] {
        LOG.with_borrow_mut(Vec::clear);
        let store = Arc::new(Mutex::new(RetainedCreateStore::default()));
        let sent = poll_retained_create_publisher(
            store.clone(),
            3,
            AUTHORITY,
            VecDeque::from([Err(refusal), Ok(retained_create_ack())]),
            event,
        );
        assert_eq!(
            sent,
            [
                (operation, "receipt-a".into()),
                (operation, "receipt-b".into())
            ]
        );
        assert_eq!(store.lock().unwrap().stopped, [OP1]);

        let sent = poll_retained_create_publisher(
            store,
            2,
            AUTHORITY,
            VecDeque::from([Ok(retained_create_ack())]),
            event,
        );
        assert_eq!(sent, [(operation, "receipt-b".into())]);
        LOG.with_borrow(|log| {
            assert_eq!(log.len(), 1);
            assert!(log[0].contains(&format!("receipt={label} alarm=receipt_refused")));
        });
    }
}

#[test]
fn retained_create_authority_mismatch_alarms_without_stopping_and_publishes_after_restart() {
    thread_local! {
        static LOG: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let event = |event: &str| LOG.with_borrow_mut(|log| log.push(event.into()));
    let operation = Operation::PublishCharacterCommandReceiptV1;
    let store = Arc::new(Mutex::new(RetainedCreateStore::default()));
    let sent = poll_retained_create_publisher(
        store.clone(),
        5,
        OTHER_AUTHORITY,
        VecDeque::from([Ok(retained_create_ack()), Ok(retained_create_ack())]),
        event,
    );
    assert_eq!(
        sent,
        [
            (operation, "receipt-b".into()),
            (operation, "receipt-b".into())
        ]
    );
    assert!(store.lock().unwrap().stop_attempts_at.is_empty());
    LOG.with_borrow(|log| {
        assert_eq!(log.len(), 1);
        assert!(log[0].contains("receipt=invalid_stored_state alarm="));
    });

    let sent = poll_retained_create_publisher(
        store,
        1,
        AUTHORITY,
        VecDeque::from([Ok(retained_create_ack())]),
        |_| {},
    );
    assert_eq!(sent, [(operation, "receipt-a".into())]);
}

#[test]
fn retained_create_failed_stop_allows_later_receipt_in_same_pass() {
    thread_local! {
        static LOG: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let event = |event: &str| LOG.with_borrow_mut(|log| log.push(event.into()));
    let store = Arc::new(Mutex::new(RetainedCreateStore {
        fail_next_stop: true,
        ..RetainedCreateStore::default()
    }));
    let operation = Operation::PublishCharacterCommandReceiptV1;
    let sent = poll_retained_create_publisher(
        store.clone(),
        3,
        AUTHORITY,
        VecDeque::from([
            Err(classify(409, b"")),
            Ok(retained_create_ack()),
            Err(classify(409, b"")),
        ]),
        event,
    );
    assert_eq!(
        sent,
        [
            (operation, "receipt-a".into()),
            (operation, "receipt-b".into()),
            (operation, "receipt-a".into()),
        ],
    );
    let store = store.lock().unwrap();
    assert_eq!(store.reads_at, [Duration::ZERO, Duration::ZERO, POLL]);
    assert_eq!(store.stop_attempts_at, [Duration::ZERO, POLL]);
    assert_eq!(store.stopped, [OP1]);
    LOG.with_borrow(|log| {
        assert_eq!(log.len(), 2);
        assert!(log[0].contains("receipt=conflict alarm=receipt_stop_write_failed"));
        assert!(log[1].contains("receipt=conflict alarm=receipt_refused"));
    });
}

#[test]
fn retained_create_503_retries_next_poll_during_continuous_scan() {
    let store = Arc::new(Mutex::new(RetainedCreateStore {
        continuous: true,
        ..RetainedCreateStore::default()
    }));
    let operation = Operation::PublishCharacterCommandReceiptV1;
    let sent = poll_retained_create_publisher(
        store.clone(),
        4,
        AUTHORITY,
        VecDeque::from([
            Err(classify(503, b"")),
            Ok(retained_create_ack()),
            Ok(retained_create_ack()),
            Ok(retained_create_ack()),
        ]),
        |event| assert!(!event.contains("alarm="), "{event}"),
    );
    assert_eq!(
        sent,
        [
            (operation, "receipt-a".into()),
            (operation, "receipt-b".into()),
            (operation, "receipt-a".into()),
            (operation, "receipt-new".into()),
        ],
    );
    let store = store.lock().unwrap();
    assert_eq!(store.reads_at, [Duration::ZERO, Duration::ZERO, POLL, POLL]);
    assert!(store.stopped.is_empty());
}

fn retained_create_retry_allows_later_receipts(first: Result<Vec<u8>, Failure>) {
    let store = Arc::new(Mutex::new(RetainedCreateStore::default()));
    let sent = poll_retained_create_publisher(
        store.clone(),
        3,
        AUTHORITY,
        VecDeque::from([first, Ok(retained_create_ack()), Ok(retained_create_ack())]),
        |event| assert!(!event.contains("alarm="), "{event}"),
    );
    let operation = Operation::PublishCharacterCommandReceiptV1;
    assert_eq!(
        sent,
        [
            (operation, "receipt-a".into()),
            (operation, "receipt-b".into()),
            (operation, "receipt-a".into()),
        ],
    );
    assert!(store.lock().unwrap().stopped.is_empty());
}

#[test]
fn retained_create_rate_limit_retries_after_later_receipt() {
    retained_create_retry_allows_later_receipts(Err(classify(429, b"")));
}

#[test]
fn retained_create_malformed_success_retries_after_later_receipt() {
    retained_create_retry_allows_later_receipts(Ok(b"invalid acknowledgement".to_vec()));
}

#[test]
fn retained_create_empty_bad_request_retries_after_later_receipt() {
    retained_create_retry_allows_later_receipts(Err(classify(400, b"")));
}

#[test]
fn retained_create_transport_failure_retries_after_later_receipt() {
    retained_create_retry_allows_later_receipts(Err(Failure::Unavailable));
}

struct PauseAtPoll(Clock);

impl ReportClock for PauseAtPoll {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn unix_now(&self) -> i64 {
        self.0.unix_now()
    }

    async fn sleep(&self, duration: Duration) {
        let mut resume = false;
        std::future::poll_fn(|_| {
            if std::mem::replace(&mut resume, true) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        self.0.sleep(duration).await;
    }
}

fn retry_after_store_change(
    update_store: impl FnOnce(&mut RetainedCreateStore),
    expected_retries: &[&str],
    polls: usize,
) {
    let store = Arc::new(Mutex::new(RetainedCreateStore::default()));
    let clock = PauseAtPoll(store.lock().unwrap().clock.clone());
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut script = VecDeque::from([Err(Failure::Unavailable), Ok(retained_create_ack())]);
    script.extend(expected_retries.iter().map(|_| Ok(retained_create_ack())));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let _timer_context = runtime.enter();
    let mut publisher = pin!(run_retained_create_publications(
        clock,
        Source {
            script: Arc::new(Mutex::new(script)),
            sent: sent.clone(),
        },
        RetainedCreates {
            store: store.clone(),
            reads: 0,
            read_limit: usize::MAX,
            source_authority: AUTHORITY,
        },
        |_| {},
    ));

    let mut cx = Context::from_waker(Waker::noop());
    assert!(publisher.as_mut().poll(&mut cx).is_pending());
    assert_eq!(sent.lock().unwrap().len(), 2);
    update_store(&mut store.lock().unwrap());
    for _ in 0..polls {
        assert!(publisher.as_mut().poll(&mut cx).is_pending());
    }
    let sent = sent.lock().unwrap();
    let operation = Operation::PublishCharacterCommandReceiptV1;
    let mut bodies = vec!["receipt-a", "receipt-b"];
    bodies.extend_from_slice(expected_retries);
    assert_eq!(
        *sent,
        bodies
            .into_iter()
            .map(|body| (operation, body.to_owned()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn queued_retry_is_not_sent_after_durable_stop_committed_elsewhere() {
    retry_after_store_change(|store| store.stopped.push(OP1.into()), &[], 1);
}

#[test]
fn queued_retry_is_not_sent_after_incarnation_lost() {
    retry_after_store_change(|store| store.incarnation_lost = true, &[], 1);
}

#[test]
fn queued_retry_uses_current_bytes_and_never_substitutes_a_successor() {
    retry_after_store_change(
        |store| {
            store
                .replacements
                .insert(OP1.into(), b"current-bytes".to_vec());
        },
        &["current-bytes"],
        1,
    );
    retry_after_store_change(
        |store| {
            store.removed.insert(OP1.into());
        },
        &[],
        1,
    );
}

#[test]
fn queued_retry_remains_eligible_after_unavailable_fenced_read() {
    retry_after_store_change(
        |store| {
            store.unavailable_operation = Some(OP1.into());
        },
        &["receipt-a", "receipt-b"],
        2,
    );
}

#[test]
fn retained_create_nonempty_refusals_never_stop_the_receipt() {
    for status in [404, 409] {
        retained_create_retry_allows_later_receipts(Err(classify(status, b"unexpected body")));
    }
}

#[test]
fn retained_create_later_retry_gets_a_full_exchange_deadline() {
    struct SlowFirstRetry {
        clock: Clock,
        attempts_a: usize,
        attempts_b: usize,
        accepted: Arc<tokio::sync::Notify>,
    }

    impl CommandSource for SlowFirstRetry {
        async fn exchange(&mut self, _: Operation, body: &str) -> Result<Vec<u8>, Failure> {
            if body == "receipt-a" {
                self.attempts_a += 1;
                if self.attempts_a > 1 {
                    *self.clock.0.lock().unwrap() +=
                        RETAINED_CREATE_RETRY_TIME_PER_POLL - Duration::from_nanos(1);
                }
            } else {
                assert_eq!(body, "receipt-b");
                self.attempts_b += 1;
                if self.attempts_b > 1 {
                    // This timer expires well before the exchange deadline and
                    // after the first retry's leftover nanosecond.
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    self.accepted.notify_one();
                    return Ok(retained_create_ack());
                }
            }
            Err(Failure::Unavailable)
        }
    }

    struct YieldingClock(Clock);

    impl ReportClock for YieldingClock {
        fn elapsed(&self) -> Duration {
            self.0.elapsed()
        }

        fn unix_now(&self) -> i64 {
            self.0.unix_now()
        }

        async fn sleep(&self, duration: Duration) {
            self.0.sleep(duration).await;
            tokio::task::yield_now().await;
        }
    }

    let store = Arc::new(Mutex::new(RetainedCreateStore::default()));
    let clock = store.lock().unwrap().clock.clone();
    let accepted = Arc::new(tokio::sync::Notify::new());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut publisher = pin!(run_retained_create_publications(
            YieldingClock(clock.clone()),
            SlowFirstRetry {
                clock,
                attempts_a: 0,
                attempts_b: 0,
                accepted: accepted.clone(),
            },
            RetainedCreates {
                store,
                reads: 0,
                // Two startup rows, two exact retries and one end-of-scan probe.
                read_limit: 5,
                source_authority: AUTHORITY,
            },
            |_| {},
        ));
        let mut acknowledgement = pin!(accepted.notified());
        tokio::time::timeout(
            Duration::from_secs(2),
            std::future::poll_fn(|cx| {
                assert!(publisher.as_mut().poll(cx).is_pending());
                acknowledgement.as_mut().poll(cx)
            }),
        )
        .await
        .expect("B must get its first full retry while A keeps failing");
    });
}

#[test]
fn retained_create_retry_rotation_and_overflow_recover_during_backlog_growth() {
    struct SlowPeer {
        clock: Clock,
        calls: Arc<Mutex<Vec<String>>>,
        overflow_retried: bool,
    }

    impl CommandSource for SlowPeer {
        async fn exchange(&mut self, operation: Operation, body: &str) -> Result<Vec<u8>, Failure> {
            assert_eq!(operation, Operation::PublishCharacterCommandReceiptV1);
            self.calls.lock().unwrap().push(body.into());
            // Simulate one failed exchange consuming the retry budget, leaving
            // the next queue head with a fresh budget on the next poll.
            *self.clock.0.lock().unwrap() += RETAINED_CREATE_RETRY_TIME_PER_POLL;
            if body == op(1025) && std::mem::replace(&mut self.overflow_retried, true) {
                return Ok(retained_create_ack());
            }
            Err(Failure::Unavailable)
        }
    }

    thread_local! {
        static OVERFLOWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    OVERFLOWS.set(0);
    let store = Arc::new(Mutex::new(RetainedCreateStore {
        generated_rows: 1026,
        ..RetainedCreateStore::default()
    }));
    let clock = store.lock().unwrap().clock.clone();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let _timer_context = runtime.enter();
    let mut publisher = pin!(run_retained_create_publications(
        PauseAtPoll(clock.clone()),
        SlowPeer {
            clock,
            calls: calls.clone(),
            overflow_retried: false
        },
        RetainedCreates {
            store: store.clone(),
            reads: 0,
            read_limit: usize::MAX,
            source_authority: AUTHORITY
        },
        |event| {
            assert!(
                event.ends_with("alarm=retained_create_retry_limit"),
                "{event}"
            );
            OVERFLOWS.set(OVERFLOWS.get() + 1);
        },
    ));
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..3 {
        assert!(publisher.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(*calls.lock().unwrap(), [1, 2, 1, 3, 4, 2, 5, 6].map(op));

    // First sweep through 1026 rows, one wrap poll, and a second sweep
    // through the overflowed row. New rows arrive faster than scan capacity.
    for _ in 3..(513 + 1 + 513) {
        store.lock().unwrap().generated_rows += 3;
        assert!(publisher.as_mut().poll(&mut cx).is_pending());
    }
    let calls = calls.lock().unwrap();
    assert_eq!(calls.iter().filter(|body| **body == op(1025)).count(), 2);
    assert!(OVERFLOWS.get() >= 2);
    assert!(store.lock().unwrap().stopped.is_empty());
}

#[test]
fn retained_create_invalid_alarm_capacity_allows_valid_rows_and_realarms_evicted_ids() {
    thread_local! {
        static ALARMS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    ALARMS.with_borrow_mut(Vec::clear);
    let store = Arc::new(Mutex::new(RetainedCreateStore {
        generated_rows: 1026,
        invalid_generated: true,
        ..RetainedCreateStore::default()
    }));
    let sent = poll_retained_create_publisher(
        store.clone(),
        1028,
        AUTHORITY,
        VecDeque::from([Ok(retained_create_ack())]),
        |event| ALARMS.with_borrow_mut(|alarms| alarms.push(event.into())),
    );
    assert_eq!(
        sent,
        [(Operation::PublishCharacterCommandReceiptV1, op(1026))]
    );
    assert!(store.lock().unwrap().stop_attempts_at.is_empty());
    ALARMS.with_borrow(|alarms| {
        assert_eq!(
            alarms
                .iter()
                .filter(|line| line.ends_with("alarm=invalid_retained_receipt"))
                .count(),
            1026
        );
        assert_eq!(
            alarms
                .iter()
                .filter(|line| line.ends_with("alarm=retained_create_invalid_limit"))
                .count(),
            2
        );
    });
}

struct ClosureFeed(std::array::IntoIter<(Operation, &'static str), 3>);
impl Decider for ClosureFeed {
    async fn decide(&mut self, _: &str, _: &str, _: Input<'_>) -> Decision {
        Decision::Transient
    }

    async fn next_closure_publication(
        &mut self,
    ) -> Option<super::super::account_closure::Publication> {
        let (operation, body) = self.0.next()?;
        Some(super::super::account_closure::Publication {
            operation,
            body: body.into(),
        })
    }
}

#[test]
fn closure_feed_is_served_while_retained_create_platform_exchange_waits() {
    let create_calls = Arc::new(Mutex::new(Vec::new()));
    let closure_calls = Arc::new(Mutex::new(Vec::new()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let _timer_context = runtime.enter();
    let mut creates = pin!(run_retained_create_publications(
        PauseAtPoll(Clock::default()),
        Source {
            sent: create_calls.clone(),
            script: Arc::new(Mutex::new(VecDeque::new())),
        },
        RetainedCreates {
            store: Arc::new(Mutex::new(RetainedCreateStore::default())),
            reads: 0,
            read_limit: usize::MAX,
            source_authority: AUTHORITY,
        },
        |_| {},
    ));
    let publications = [
        (
            Operation::PublishAccountClosureCommandReceiptV2,
            "closure-receipt",
        ),
        (Operation::PublishAccountClosureV1, "closure-projection"),
        (
            Operation::PublishAccountClosureWatermarkV1,
            "freshness-watermark",
        ),
    ];
    let mut closure = pin!(run_account_closure_publications(
        PauseAtPoll(Clock::default()),
        Source {
            sent: closure_calls.clone(),
            script: Arc::new(Mutex::new(VecDeque::from(publications.map(|_| {
                Ok(br#"{"contract_version":2,"result":"accepted"}"#.to_vec())
            })))),
        },
        ClosureFeed(publications.into_iter()),
        |_| {},
    ));
    let mut cx = Context::from_waker(Waker::noop());
    for _ in &publications {
        assert!(creates.as_mut().poll(&mut cx).is_pending());
        assert!(closure.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(
        *create_calls.lock().unwrap(),
        [(
            Operation::PublishCharacterCommandReceiptV1,
            "receipt-a".into()
        )]
    );
    assert_eq!(
        *closure_calls.lock().unwrap(),
        publications.map(|(operation, body)| (operation, body.to_owned()))
    );
}

#[test]
fn retained_create_failed_scan_rotates_and_wrap_reaches_in_range_arrival() {
    let store = Arc::new(Mutex::new(RetainedCreateStore {
        generated_rows: 4,
        scan_failures: BTreeSet::from([2]),
        ..RetainedCreateStore::default()
    }));
    let clock = store.lock().unwrap().clock.clone();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let _timer_context = runtime.enter();
    let mut publisher = pin!(run_retained_create_publications(
        PauseAtPoll(clock),
        Source {
            script: Arc::new(Mutex::new(VecDeque::from(vec![
                Err(Failure::Unavailable);
                32
            ]))),
            sent: sent.clone(),
        },
        RetainedCreates {
            store: store.clone(),
            reads: 0,
            read_limit: usize::MAX,
            source_authority: AUTHORITY,
        },
        |event| assert!(
            event.ends_with("alarm=retained_create_scan_unavailable"),
            "{event}"
        ),
    ));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(publisher.as_mut().poll(&mut cx).is_pending());
    {
        let mut store = store.lock().unwrap();
        store.generated_rows = 5;
        store.late_uuid = Some((5, op(0)));
    }
    for expected in [vec![1, 2, 1, 2], vec![1, 2, 1, 2, 1, 2, 4]] {
        assert!(publisher.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            sent.lock()
                .unwrap()
                .iter()
                .map(|(_, body)| body.clone())
                .collect::<Vec<_>>(),
            expected.into_iter().map(op).collect::<Vec<_>>()
        );
    }
    for _ in 0..4 {
        store.lock().unwrap().generated_rows += 3;
        assert!(publisher.as_mut().poll(&mut cx).is_pending());
    }
    let sent = sent.lock().unwrap();
    assert!(
        sent.iter().any(|(_, body)| *body == op(3)),
        "wrap revisits skipped candidate"
    );
    assert!(
        sent.iter().any(|(_, body)| *body == op(0)),
        "new in-range UUID arrives in the next sweep"
    );
}

#[test]
fn closure_feed_is_served_before_create_retries_when_queue_is_full() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Peer {
        clock: Clock,
        slow: Arc<AtomicBool>,
        sent: Arc<Mutex<Vec<String>>>,
    }
    impl CommandSource for Peer {
        async fn exchange(&mut self, _: Operation, body: &str) -> Result<Vec<u8>, Failure> {
            self.sent.lock().unwrap().push(body.to_owned());
            if self.slow.load(Ordering::Relaxed) {
                *self.clock.0.lock().unwrap() += RETAINED_CREATE_RETRY_TIME_PER_POLL;
            }
            Err(Failure::Unavailable)
        }
    }
    let store = Arc::new(Mutex::new(RetainedCreateStore {
        generated_rows: 1030,
        ..RetainedCreateStore::default()
    }));
    let clock = store.lock().unwrap().clock.clone();
    let create_calls = Arc::new(Mutex::new(Vec::new()));
    let closure_calls = Arc::new(Mutex::new(Vec::new()));
    let slow = Arc::new(AtomicBool::new(true));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let _timer_context = runtime.enter();
    thread_local! {
        static SATURATION: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    SATURATION.set(0);
    let mut creates = pin!(run_retained_create_publications(
        PauseAtPoll(clock.clone()),
        Peer {
            clock,
            slow: slow.clone(),
            sent: create_calls.clone()
        },
        RetainedCreates {
            store,
            reads: 0,
            read_limit: usize::MAX,
            source_authority: AUTHORITY
        },
        |event| {
            assert!(event.ends_with("alarm=retained_create_retry_limit"));
            SATURATION.set(SATURATION.get() + 1);
        },
    ));
    let publications = [
        (Operation::PublishAccountClosureCommandReceiptV2, "receipt"),
        (Operation::PublishAccountClosureV1, "projection"),
        (Operation::PublishAccountClosureWatermarkV1, "heartbeat"),
    ];
    let mut closure = pin!(run_account_closure_publications(
        PauseAtPoll(Clock::default()),
        Source {
            script: Arc::new(Mutex::new(VecDeque::from(vec![
                Err(Failure::Unavailable);
                3
            ]))),
            sent: closure_calls.clone(),
        },
        ClosureFeed(publications.into_iter()),
        |_| {},
    ));
    let mut cx = Context::from_waker(Waker::noop());
    for _ in 0..513 {
        assert!(creates.as_mut().poll(&mut cx).is_pending());
    }
    assert_eq!(SATURATION.get(), 2);
    for (index, expected) in [8, 1].into_iter().enumerate() {
        slow.store(index != 0, Ordering::Relaxed);
        let before = create_calls.lock().unwrap().len();
        assert!(closure.as_mut().poll(&mut cx).is_pending());
        assert_eq!(closure_calls.lock().unwrap().len(), index + 1);
        assert!(creates.as_mut().poll(&mut cx).is_pending());
        assert_eq!(create_calls.lock().unwrap().len() - before, expected + 2);
    }
    assert!(closure.as_mut().poll(&mut cx).is_pending());
    assert_eq!(
        *closure_calls.lock().unwrap(),
        publications.map(|(operation, body)| (operation, body.to_owned()))
    );
}
