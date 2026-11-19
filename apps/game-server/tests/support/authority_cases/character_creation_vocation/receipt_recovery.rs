use super::*;
use native_admission_source::SourceError;
use native_admission_source::character_commands::{
    CommandSource, Decider, Failure, RetainedCreateReceipt, run_retained_create_publications,
};
use native_admission_source::descriptor::Operation;
use native_admission_source::runtime_status::SystemClock;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
const SOURCE: &str = "oteryn:character-authority:primary";

struct Peer {
    calls: Arc<Mutex<Vec<(Operation, String)>>>,
    publications: AtomicUsize,
    accepted: Arc<tokio::sync::Notify>,
}
impl CommandSource for Peer {
    async fn exchange(&mut self, operation: Operation, body: &str) -> Result<Vec<u8>, Failure> {
        self.calls
            .lock()
            .map_err(|_| Failure::Unavailable)?
            .push((operation, body.to_owned()));
        match operation {
            Operation::ReadCharacterCommandIntentV1 => Err(Failure::NotFound),
            Operation::PublishCharacterCommandReceiptV1 => {
                if self.publications.fetch_add(1, Ordering::SeqCst) < 2 {
                    return Err(Failure::Unavailable);
                }
                self.accepted.notify_one();
                Ok(br#"{"contract_version":1,"result":"accepted"}"#.to_vec())
            }
            _ => Err(Failure::InvalidResponse),
        }
    }
}
struct Durable<'a, 'r, 'c> {
    root: &'a DurabilityRoot,
    authority: &'a durability::character_authority::ReconciledCharacterAuthority<'r, 'c>,
    node: &'a NodeIncarnationProof,
    operation_id: &'a str,
    exact_retry_reads: &'a AtomicUsize,
}
impl Decider for Durable<'_, '_, '_> {
    async fn next_retained_create_receipt(
        &mut self,
        after: Option<&str>,
    ) -> Result<Option<RetainedCreateReceipt>, SourceError> {
        let row = self.root
            .next_retained_create_receipt(self.authority, self.node, SOURCE, after)
            .await
            .map_err(|_| SourceError::Unavailable)?;
        if after.is_some_and(|cursor| cursor < self.operation_id) {
            self.exact_retry_reads.fetch_add(1, Ordering::Relaxed);
        }
        Ok(row)
    }
    async fn decide(&mut self, command: &str, operation: &str, input: Input<'_>) -> Decision {
        self.root
            .decide_character_command(self.authority, self.node, SOURCE, command, operation, input)
            .await
    }
}
pub(super) async fn exercise(
    root: &DurabilityRoot,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    receipt: &[u8],
) -> TestResult {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let accepted = Arc::new(tokio::sync::Notify::new());
    let mut peer = Peer {
        calls: calls.clone(),
        publications: AtomicUsize::new(0),
        accepted: accepted.clone(),
    };
    let body = std::str::from_utf8(receipt)?;
    assert!(matches!(
        peer.exchange(Operation::PublishCharacterCommandReceiptV1, body)
            .await,
        Err(Failure::Unavailable)
    ));
    let decoded: serde_json::Value = serde_json::from_slice(receipt)?;
    let operation_id = decoded["operation_id"].as_str().ok_or("operation")?;
    let read =
        native_admission_source::character_commands::encode_read(SOURCE, CREATE_COMMAND, operation_id)?;
    assert!(matches!(
        peer.exchange(Operation::ReadCharacterCommandIntentV1, &read)
            .await,
        Err(Failure::NotFound)
    ));
    let exact_retry_reads = AtomicUsize::new(0);
    let publisher = run_retained_create_publications(
        SystemClock::default(),
        peer,
        Durable {
            root,
            authority,
            node,
            operation_id,
            exact_retry_reads: &exact_retry_reads,
        },
        |_| {},
    );
    tokio::select! {
        () = accepted.notified() => {},
        () = publisher => return Err("receipt publisher stopped".into()),
        () = tokio::time::sleep(std::time::Duration::from_secs(20)) => return Err("retained receipt was not recovered".into()),
    }
    {
        let calls = calls.lock().map_err(|_| "publication calls poisoned")?;
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0].1.as_bytes(), receipt);
        for (operation, bytes) in &calls[2..] {
            assert_eq!(*operation, Operation::PublishCharacterCommandReceiptV1);
            assert_eq!(bytes.as_bytes(), receipt);
        }
    }
    assert!(exact_retry_reads.load(Ordering::Relaxed) > 0);
    root.stop_retained_create_receipt(authority, node, operation_id).await?;
    root.stop_retained_create_receipt(authority, node, operation_id).await?;
    assert!(
        root.next_retained_create_receipt(authority, node, SOURCE, None)
            .await?
            .is_none(),
        "a fresh scan excludes the durably stopped receipt"
    );
    eprintln!("RETAINED_CREATE_RECEIPT_PG_EXECUTED publish_failure=2 read_404=1 recovered=1");
    Ok(())
}
