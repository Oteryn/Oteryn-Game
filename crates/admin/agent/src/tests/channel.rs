use super::*;
use crate::{
    channel::{Agent, Clock, Transport},
    config::Config,
    store::{IdempotencyStore, NonceStore},
};
use ErrorCode as E;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};
#[derive(Default)]
pub struct MemoryStore {
    nonces: BTreeMap<String, i64>,
    results: BTreeMap<String, (String, Response)>,
}
impl NonceStore for MemoryStore {
    async fn claim_nonce(&mut self, nonce: &str, now: i64) -> Result<bool, E> {
        if self.nonces.get(nonce).is_some_and(|expiry| *expiry > now) {
            return Ok(false);
        }
        self.nonces.insert(nonce.into(), now + 86400);
        Ok(true)
    }
}
impl IdempotencyStore for MemoryStore {
    async fn audit(&mut self, _: &Request, _: &str, _: &Response, _: i64) -> Result<(), E> {
        Ok(())
    }

    async fn lookup(&mut self, id: &str) -> Result<Option<(String, Response)>, E> {
        Ok(self.results.get(id).cloned())
    }
    async fn finish(
        &mut self,
        r: &Request,
        d: &str,
        response: &Response,
        _: i64,
    ) -> Result<Response, E> {
        if let Some((prior, value)) = self.results.get(&r.admin_operation_id) {
            return if prior == d {
                Ok(value.clone())
            } else {
                Err(E::AdmIdempotencyConflict)
            };
        }
        self.results
            .insert(r.admin_operation_id.clone(), (d.into(), response.clone()));
        Ok(response.clone())
    }
}
#[derive(Clone, Default)]
struct FakeTransport {
    calls: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    batches: Arc<Mutex<VecDeque<serde_json::Value>>>,
    superseded: bool,
    slow_results: bool,
}
impl Transport for FakeTransport {
    async fn post(&self, endpoint: &str, body: serde_json::Value) -> Result<serde_json::Value, E> {
        self.calls
            .lock()
            .map_err(|_| E::AdmInternal)?
            .push((endpoint.into(), body));
        if self.superseded {
            return Err(E::AdmAgentSuperseded);
        }
        if endpoint == "channel" {
            let batch = self.batches.lock().map_err(|_| E::AdmInternal)?.pop_front();
            if let Some(batch) = batch {
                return Ok(batch);
            }
            return std::future::pending().await;
        }
        if endpoint == "results" && self.slow_results {
            tokio::time::sleep(std::time::Duration::from_secs(21)).await;
            self.calls
                .lock()
                .map_err(|_| E::AdmInternal)?
                .push(("results_finished".into(), serde_json::Value::Null));
        }
        Ok(serde_json::Value::Null)
    }
}
#[derive(Clone)]
struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> i64 {
        NOW
    }
}
fn agent() -> Result<Agent<FakeTransport, MemoryStore, FixedClock>, WireError> {
    let config = Config::parse(
        r#"
agent_id="test-agent"
platform_endpoint="https://platform.example/"
mtls_certificate_path="cert.pem"
mtls_private_key_path="key.pem"
mtls_ca_path="ca.pem"
signing_key_path="signing.key"
policy_file_path="policy.toml"
db_url_env_name="ADMIN_TEST_DB_URL"
owner_epoch=1
worlds=["w-antica"]
nodes=["node-1"]
build_revision="test-build"
"#,
    )?;
    Ok(Agent {
        config,
        policy: policy()?,
        transport: FakeTransport::default(),
        store: MemoryStore::default(),
        clock: FixedClock,
        signing_key: key(),
    })
}
#[tokio::test(start_paused = true)]
async fn outbound_loop_hello_channel_results_heartbeat_and_supersession() -> Result<(), WireError> {
    let mut a = agent()?;
    a.transport
        .batches
        .lock()
        .map_err(|_| "poisoned")?
        .push_back(json!([request()?]));
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(21), a.run())
            .await
            .is_err()
    );
    {
        let calls = a.transport.calls.lock().map_err(|_| "poisoned")?;
        assert_eq!(calls.first().map(|v| v.0.as_str()), Some("hello"));
        assert!(calls.iter().any(|(name, _)| name == "channel"));
        let result = calls
            .iter()
            .find(|(name, _)| name == "results")
            .ok_or("missing result")?;
        let response: Response = serde_json::from_value(result.1.clone())?;
        assert_eq!(response.status, Status::Succeeded);
        verify_signature(&response, &response.signature, &key().verifying_key())?;
        assert!(calls.iter().filter(|(name, _)| name == "heartbeat").count() >= 3);
    }
    a.transport.superseded = true;
    assert_eq!(a.run().await, Err(E::AdmAgentSuperseded));
    Ok(())
}
#[tokio::test]
async fn replay_expiry_idempotency_and_schema_rejections_reach_real_driver() -> Result<(), WireError>
{
    let mut a = agent()?;
    let mut r = request()?;
    let result = a.process(&r).await.map_err(|e| format!("{e:?}"))?;
    assert_eq!(result.status, Status::Succeeded);
    assert_eq!(
        a.process(&r)
            .await
            .map_err(|e| format!("{e:?}"))?
            .error
            .ok_or("missing replay")?
            .code,
        E::AdmReplay
    );
    r.nonce = URL_SAFE_NO_PAD.encode([2; 16]);
    resign(&mut r)?;
    let prior = a.process(&r).await.map_err(|e| format!("{e:?}"))?;
    assert_eq!(prior.game_ledger_id, result.game_ledger_id);
    r.nonce = URL_SAFE_NO_PAD.encode([3; 16]);
    r.expires_at = "2026-11-05T12:00:00Z".into();
    resign(&mut r)?;
    assert_eq!(
        a.process(&r)
            .await
            .map_err(|e| format!("{e:?}"))?
            .error
            .ok_or("missing expiry")?
            .code,
        E::AdmExpired
    );
    r = request()?;
    r.nonce = URL_SAFE_NO_PAD.encode([4; 16]);
    r.params = json!({"extra":1});
    resign(&mut r)?;
    assert_eq!(
        a.process(&r)
            .await
            .map_err(|e| format!("{e:?}"))?
            .error
            .ok_or("missing schema error")?
            .code,
        E::AdmInvalidParams
    );
    r.params = json!({});
    resign(&mut r)?;
    assert_eq!(
        a.process(&r)
            .await
            .map_err(|e| format!("{e:?}"))?
            .error
            .ok_or("nonce not consumed")?
            .code,
        E::AdmReplay
    );
    assert_eq!(a.store.claim_nonce("expired", NOW).await, Ok(true));
    assert_eq!(a.store.claim_nonce("expired", NOW + 86399).await, Ok(false));
    assert_eq!(a.store.claim_nonce("expired", NOW + 86400).await, Ok(true));
    assert_eq!(a.deliver(json!([{}])).await, Err(E::AdmInvalidParams));
    assert_eq!(
        a.deliver(json!(vec![json!({}); 33])).await,
        Err(E::AdmLimitExceeded)
    );
    Ok(())
}

#[tokio::test(start_paused = true)]
async fn heartbeat_continues_while_results_transport_is_blocked() -> Result<(), WireError> {
    let mut a = agent()?;
    a.transport.slow_results = true;
    a.transport
        .batches
        .lock()
        .map_err(|_| "poisoned")?
        .push_back(json!([request()?]));
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(22), a.run())
            .await
            .is_err()
    );
    let calls = a.transport.calls.lock().map_err(|_| "poisoned")?;
    let finished = calls
        .iter()
        .position(|(name, _)| name == "results_finished")
        .ok_or("result did not finish")?;
    assert!(
        calls[..finished]
            .iter()
            .filter(|(name, _)| name == "heartbeat")
            .count()
            >= 3
    );
    Ok(())
}
