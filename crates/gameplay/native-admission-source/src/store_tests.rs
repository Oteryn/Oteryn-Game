#![allow(clippy::unwrap_used)]
use super::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

type Calls = Arc<Mutex<Vec<(Operation, String)>>>;
type Receipts = Arc<Mutex<Vec<Vec<u8>>>>;

const OP: &str = "01890f3e-7b1c-7abc-8def-0123456789ab";
const ORDER: &str = "01890f3e-7b1d-7abc-8def-0123456789ab";
const RECEIPT: &[u8] =
    br#"{"operation_id":"01890f3e-7b1c-7abc-8def-0123456789ab","result":"COMMITTED"}"#;

struct Fake {
    replies: VecDeque<Result<Vec<u8>, TransportFailure>>,
    calls: Calls,
}
impl StoreTransport for Fake {
    fn exchange<'a>(
        &'a mut self,
        op: Operation,
        body: &'a str,
    ) -> BoxFuture<'a, Result<Vec<u8>, TransportFailure>> {
        self.calls.lock().unwrap().push((op, body.to_owned()));
        let value = self.replies.pop_front().unwrap();
        Box::pin(async move { value })
    }
}
fn client(replies: Vec<Result<Vec<u8>, TransportFailure>>) -> (StoreClient, Calls) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    (
        StoreClient::new(Box::new(Fake {
            replies: replies.into(),
            calls: calls.clone(),
        })),
        calls,
    )
}
fn list() -> Vec<u8> {
    format!(r#"{{"contract_version":1,"entries":[{{"operation_id":"{OP}"}}],"next_cursor":null}}"#)
        .into_bytes()
}
fn intent() -> Vec<u8> {
    format!(r#"{{"contract_version":1,"intent":{{"operation_id":"{OP}","account_id":"acct-1","character_id":"char-1","world_id":"world-1","fulfilment_unit_id":"unit-1","catalog_revision":"rev-1","offer_key":"offer-1"}}}}"#).into_bytes()
}
struct Apply {
    seen: Arc<Mutex<usize>>,
}
struct Publisher {
    replies: VecDeque<Result<(), ProtocolError>>,
    receipts: Receipts,
}
impl PublishReceipt for Publisher {
    fn publish<'a>(&'a mut self, receipt: &'a [u8]) -> BoxFuture<'a, Result<(), ProtocolError>> {
        self.receipts.lock().unwrap().push(receipt.to_vec());
        let reply = self.replies.pop_front().unwrap();
        Box::pin(async move { reply })
    }
}
fn publisher(replies: Vec<Result<(), ProtocolError>>) -> (Box<dyn PublishReceipt>, Receipts) {
    let receipts = Arc::new(Mutex::new(Vec::new()));
    (
        Box::new(Publisher {
            replies: replies.into(),
            receipts: receipts.clone(),
        }),
        receipts,
    )
}
impl ApplyDelivery for Apply {
    fn apply<'a>(&'a mut self, _: &'a DeliveryIntent) -> BoxFuture<'a, Result<Vec<u8>, ()>> {
        *self.seen.lock().unwrap() += 1;
        Box::pin(async { Ok(RECEIPT.to_vec()) })
    }
}
#[derive(Default)]
struct Events {
    unknown: Arc<Mutex<Vec<String>>>,
}
impl ReceiptEvents for Events {
    fn unknown_operation(&mut self, id: &str) {
        self.unknown.lock().unwrap().push(id.to_owned());
    }
}

#[test]
fn replay_of_same_intent_reapplies_and_republishes_stored_receipt() {
    block_on(async {
        let (c, calls) = client(vec![Ok(list()), Ok(intent()), Ok(list()), Ok(intent())]);
        let (publisher, receipts) = publisher(vec![Ok(()), Ok(())]);
        let seen = Arc::new(Mutex::new(0));
        let mut p = StorePoller::new(
            c,
            Box::new(Apply { seen: seen.clone() }),
            publisher,
            Box::new(Events::default()),
        );
        p.poll_once().await;
        p.poll_once().await;
        assert_eq!(*seen.lock().unwrap(), 2);
        assert_eq!(*receipts.lock().unwrap(), vec![RECEIPT.to_vec(); 2]);
        assert_eq!(calls.lock().unwrap().len(), 4);
    });
}

#[test]
fn lost_publish_response_retries_without_changing_operation() {
    block_on(async {
        let (c, calls) = client(vec![Ok(list()), Ok(intent()), Ok(list()), Ok(intent())]);
        let (publisher, _) = publisher(vec![Err(ProtocolError::Transport), Ok(())]);
        let mut p = StorePoller::new(
            c,
            Box::new(Apply {
                seen: Arc::new(Mutex::new(0)),
            }),
            publisher,
            Box::new(Events::default()),
        );
        assert_eq!(p.poll_once().await, POLL);
        assert_eq!(p.poll_once().await, POLL);
        let reads: Vec<_> = calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(op, _)| *op == Operation::ReadStoreDeliveryIntentV1)
            .map(|(_, b)| b.clone())
            .collect();
        assert_eq!(reads.len(), 2);
        assert!(reads.iter().all(|b| b.contains(OP)));
    });
}

#[test]
fn receipt_404_stops_that_publication_and_raises_alarm() {
    block_on(async {
        let (c, _) = client(vec![Ok(list()), Ok(intent())]);
        let (publisher, _) = publisher(vec![Err(ProtocolError::NotFound)]);
        let events = Events::default();
        let unknown = events.unknown.clone();
        let mut p = StorePoller::new(
            c,
            Box::new(Apply {
                seen: Arc::new(Mutex::new(0)),
            }),
            publisher,
            Box::new(events),
        );
        assert_eq!(p.poll_once().await, POLL);
        assert_eq!(&*unknown.lock().unwrap(), &[OP.to_owned()]);
    });
}

fn request() -> PurchaseStoreOffer {
    PurchaseStoreOffer {
        purchase_request_id: OP.into(),
        account_id: "acct-1".into(),
        character_id: "char-1".into(),
        offer_key: "offer-1".into(),
        catalog_revision: "rev-1".into(),
        price: 50,
        currency: "TIBIA_COIN".into(),
    }
}

#[test]
fn purchase_closed_failures_are_typed() {
    block_on(async {
        for (code, want) in [
            ("STALE_QUOTE", PurchaseResult::StaleQuote),
            ("INSUFFICIENT_BALANCE", PurchaseResult::InsufficientBalance),
        ] {
            let raw = format!(
                r#"{{"contract_version":1,"result":"{code}","order_id":null,"operation_ids":null}}"#
            )
            .into_bytes();
            let (mut c, _) = client(vec![Ok(raw)]);
            assert_eq!(c.purchase(&request()).await.unwrap(), want);
        }
    });
}

#[test]
fn accepted_purchase_decodes_bound_ids() {
    block_on(async {
        let raw=format!(r#"{{"contract_version":1,"result":"ACCEPTED","order_id":"{ORDER}","operation_ids":["{OP}"]}}"#).into_bytes();
        let (mut c, _) = client(vec![Ok(raw)]);
        assert!(matches!(
            c.purchase(&request()).await.unwrap(),
            PurchaseResult::Accepted { .. }
        ));
    });
}

#[test]
fn malformed_and_unknown_results_fail_closed() {
    block_on(async {
        for raw in [
        br#"{"contract_version":1,"result":"NEW_SUCCESS","order_id":null,"operation_ids":null}"#
            .to_vec(),
        br#"{"contract_version":1,"entries":[],"next_cursor":null,"extra":1}"#.to_vec(),
    ] {
        let (mut c, _) = client(vec![Ok(raw)]);
        assert!(c.purchase(&request()).await.is_err() || c.list_pending(None).await.is_err());
    }
    });
}
