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
}

impl Decider for Authority {
    fn decide(
        &mut self,
        command: &str,
        _operation_id: &str,
        input: Input<'_>,
    ) -> impl Future<Output = Decision> + Send {
        let kind = match input {
            Input::Intent(raw) => String::from_utf8_lossy(raw).into_owned(),
            Input::ReadConflict => "read_conflict".into(),
            Input::Unsupported => "unsupported".into(),
        };
        self.seen.lock().unwrap().push((command.into(), kind));
        std::future::ready(self.decisions.pop_front().unwrap_or(Decision::Transient))
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
fn consumer_reads_decides_and_publishes_the_stored_receipt() {
    let o2 = op(2);
    let list = page(&[
        (CREATE_COMMAND, OP1, "17"),
        ("TransferCharacterOwnership", &o2, "18"),
    ]);
    let (sent, seen, log) = drive(
        vec![
            Ok(list),
            Ok(b"intent".to_vec()),
            Ok(ACCEPTED.to_vec()),
            Ok(ACCEPTED.to_vec()),
        ],
        vec![publish("CHAR_CMD_OK"), publish("CHAR_CMD_UNSUPPORTED")],
    );
    let operations: Vec<_> = sent.iter().map(|(o, _)| *o).collect();
    assert_eq!(
        operations,
        [
            Operation::ListPendingCharacterCommandsV1,
            Operation::ReadCharacterCommandIntentV1,
            Operation::PublishCharacterCommandReceiptV1,
            // The unsupported command is never read.
            Operation::PublishCharacterCommandReceiptV1,
            Operation::ListPendingCharacterCommandsV1,
        ]
    );
    assert_eq!(sent[2].1, "receipt-CHAR_CMD_OK");
    assert_eq!(
        seen,
        [
            (CREATE_COMMAND.to_owned(), "intent".to_owned()),
            (
                "TransferCharacterOwnership".to_owned(),
                "unsupported".to_owned()
            ),
        ]
    );
    // §8: logs name the operation, command and code, never the identifiers.
    assert!(
        log.iter()
            .any(|l| l.contains("result=CHAR_CMD_OK receipt=accepted"))
    );
    assert!(
        log.iter()
            .all(|l| !l.contains(OP1) && !l.contains("intent"))
    );
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
