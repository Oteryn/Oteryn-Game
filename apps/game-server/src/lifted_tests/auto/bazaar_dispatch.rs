//! The actual Platform command consumer dispatches through the production node
//! boundary into the durable authority. HTTP responses are synthetic; effects,
//! recovery/incarnation checks, decision storage and outboxes are real PostgreSQL.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, debug};
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::native_admission_source::SourceError;
use crate::native_admission_source::bazaar::transport::{
    Authority, Consumer, DeliveryKey, Exchange, RemoteState, TransportBatch,
};
use crate::native_admission_source::character_commands::Decision;
use crate::native_admission_source::descriptor::Operation;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const OPERATION: &str = "0192a000-0000-7000-8000-000000000082";
const SOURCE: &str = "oteryn:character-authority:primary";

struct DispatchAuthority<'a, 'r, 's> {
    root: &'a DurabilityRoot,
    authority: &'a ReconciledCharacterAuthority<'r, 's>,
    proof: &'a NodeIncarnationProof,
    decisions: Arc<Mutex<Vec<Decision>>>,
}

impl Authority for DispatchAuthority<'_, '_, '_> {
    async fn decide(&mut self, command: &str, operation: &str, raw: &[u8]) -> Decision {
        let decision = crate::node::serve::dispatch_bazaar_intent(
            self.root,
            self.authority,
            self.proof,
            SOURCE,
            command,
            operation,
            raw,
        )
        .await;
        match self.decisions.lock() {
            Ok(mut decisions) => decisions.push(decision.clone()),
            Err(_) => return Decision::Transient,
        }
        decision
    }

    async fn snapshot(&mut self) -> Result<TransportBatch, SourceError> {
        crate::node::serve::snapshot_bazaar_transport(
            self.root,
            self.authority,
            self.proof,
            "oteryn:game:primary",
            true,
        )
        .await
    }

    async fn acknowledge(
        &mut self,
        _: DeliveryKey,
        _: [u8; 32],
        _: u16,
    ) -> Result<(), SourceError> {
        // This test drives one new command after an off cycle. Its receipt can
        // only be sent on a later cycle, after a durable snapshot reads it.
        Err(SourceError::InvalidInput)
    }

    async fn reconcile(&mut self, _: &RemoteState) -> Result<(), SourceError> {
        Err(SourceError::InvalidInput)
    }
}

type Responses = VecDeque<(Operation, Vec<u8>)>;

struct ScriptedExchange {
    responses: Arc<Mutex<Responses>>,
}

impl Exchange for ScriptedExchange {
    async fn exchange(
        &mut self,
        operation: Operation,
        body: &str,
    ) -> Result<(u16, Vec<u8>, Option<Duration>), SourceError> {
        let (expected, response) = self
            .responses
            .lock()
            .map_err(|_| SourceError::Unavailable)?
            .pop_front()
            .ok_or(SourceError::InvalidInput)?;
        assert_eq!(operation, expected);
        let request: serde_json::Value =
            serde_json::from_str(body).map_err(|_| SourceError::InvalidInput)?;
        assert_eq!(request["contract_version"], 2);
        assert_eq!(request["source_authority"], "oteryn:game:primary");
        Ok((200, response, None))
    }
}

#[test]
fn bazaar_consumer_dispatch_postgres_reaches_real_node_authority_and_outbox() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        (|| -> TestResult {
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
                let started = std::time::Instant::now();
                let harness = Harness::create(admin, "bazaar_dispatch", true).await?;
                let result = dispatch_case(&harness).await;
                harness.cleanup().await?;
                eprintln!("GPROD_BAZAAR_PG_EXECUTED consumer_real_dispatch elapsed_ms={} outcome={}",started.elapsed().as_millis(),if result.is_ok(){"PASS"}else{"FAIL"});
                result
            })
        })().map_err(|error|error.to_string())
    })?.join().map_err(|_| "Bazaar dispatch worker failed")?.map_err(Into::into)
}

async fn dispatch_case(harness: &Harness) -> TestResult {
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=false WHERE singleton")
        .execute(&harness.pool)
        .await?;
    let decisions = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([
        (Operation::ListBazaarCommandStatesV1, br#"{"contract_version":2,"operations":[]}"#.to_vec()),
        (Operation::ListBazaarCommandStatesV1, br#"{"contract_version":2,"operations":[]}"#.to_vec()),
        (Operation::ListPendingBazaarCommandsV1, format!(r#"{{"contract_version":2,"pending":[{{"command":"EscrowBazaarItem","operation_id":"{OPERATION}","source_revision":"1"}}]}}"#).into_bytes()),
        (Operation::ReadBazaarCommandIntentV1, b"{}".to_vec()),
    ])));
    let adapter = DispatchAuthority {
        root: &harness.root,
        authority: &authority,
        proof: &harness.node,
        decisions: Arc::clone(&decisions),
    };
    let mut consumer = Consumer::new(
        adapter,
        ScriptedExchange {
            responses: Arc::clone(&responses),
        },
        "oteryn:game:primary".to_owned(),
    );
    consumer.cycle(|_| {}).await?;
    assert!(decisions.lock().map_err(|_| "decisions mutex")?.is_empty());
    assert_eq!(harness.count("game_bazaar_decisions").await?, 0);
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=true WHERE singleton")
        .execute(&harness.pool)
        .await?;
    consumer.cycle(|_| {}).await?;
    drop(consumer);
    assert!(responses.lock().map_err(|_| "responses mutex")?.is_empty());
    {
        let decided = decisions.lock().map_err(|_| "decisions mutex")?;
        assert_eq!(decided.len(), 1);
        assert!(
            matches!(&decided[0],Decision::Publish{result_code,..} if result_code=="CHAR_CMD_INTENT_INVALID")
        );
    }
    let stored:(Vec<u8>,bool)=sqlx::query_as("SELECT d.receipt,o.receipt_acknowledged FROM game_bazaar_decisions d JOIN game_bazaar_outbox o USING(issuer,operation_id) WHERE d.operation_id=$1::uuid")
        .bind(OPERATION).fetch_one(&harness.pool).await?;
    let receipt: serde_json::Value = serde_json::from_slice(&stored.0)?;
    assert_eq!(receipt["contract_version"], 2);
    assert_eq!(receipt["outcome"], "REJECTED");
    assert!(
        receipt
            .get("result_data")
            .is_some_and(serde_json::Value::is_null)
    );
    assert!(!stored.1);
    assert_eq!(harness.count("game_bazaar_item_locations").await?, 0);
    let batch = crate::node::serve::snapshot_bazaar_transport(
        &harness.root,
        &authority,
        &harness.node,
        "oteryn:game:primary",
        true,
    )
    .await?;
    assert!(batch.enabled);
    let published = batch
        .receipts
        .iter()
        .find(|r| r.operation == OPERATION)
        .ok_or("real production snapshot omitted committed receipt")?;
    assert_eq!(published.body.as_bytes(), stored.0);
    harness
        .root
        .bazaar_transport_ack(
            &authority,
            &harness.node,
            DeliveryKey::Receipt {
                issuer: published.issuer.clone(),
                operation: published.operation.clone(),
            },
            Sha256::digest(published.body.as_bytes()).into(),
            200,
        )
        .await
        .map_err(debug)?;
    let acknowledged: bool = sqlx::query_scalar(
        "SELECT receipt_acknowledged FROM game_bazaar_outbox WHERE operation_id=$1::uuid",
    )
    .bind(OPERATION)
    .fetch_one(&harness.pool)
    .await?;
    assert!(acknowledged);
    let paused = crate::node::serve::snapshot_bazaar_transport(
        &harness.root,
        &authority,
        &harness.node,
        "oteryn:game:primary",
        false,
    )
    .await?;
    assert!(!paused.enabled);
    eprintln!(
        "GPROD_BAZAAR_REAL_SNAPSHOT_BOUNDARY_EXECUTED batch_limit=24 published_receipts=1 ack_persisted=true"
    );
    Ok(())
}
