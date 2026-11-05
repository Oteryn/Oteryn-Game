use crate::{
    config::Config,
    policy::Policy,
    store::{IdempotencyStore, NonceStore},
    verifier::{self, Decision, Evidence},
};
use ed25519_dalek::SigningKey;
use oteryn_admin_contracts::{ErrorCode as E, *};
use serde_json::{Value, json};
use std::{collections::BTreeSet, future::Future, time::Duration};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const PREFIX: &str = "/internal/v1/admin-control";
pub trait Transport {
    fn post(&self, endpoint: &str, body: Value) -> impl Future<Output = Result<Value, E>> + Send;
}
pub trait Clock {
    fn now(&self) -> i64;
}
#[derive(Clone)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> i64 {
        OffsetDateTime::now_utc().unix_timestamp()
    }
}

#[derive(Clone)]
pub struct HttpsTransport {
    client: reqwest::Client,
    origin: String,
}
impl HttpsTransport {
    pub fn new(config: &Config) -> Result<Self, WireError> {
        let mut pem = std::fs::read(&config.mtls_certificate_path)?;
        pem.push(b'\n');
        pem.extend(std::fs::read(&config.mtls_private_key_path)?);
        let identity = reqwest::Identity::from_pem(&pem)?;
        let ca = reqwest::Certificate::from_pem(&std::fs::read(&config.mtls_ca_path)?)?;
        let client = reqwest::Client::builder()
            .identity(identity)
            .add_root_certificate(ca)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            origin: config.platform_endpoint.trim_end_matches('/').into(),
        })
    }
}
impl Transport for HttpsTransport {
    async fn post(&self, endpoint: &str, body: Value) -> Result<Value, E> {
        let mut response = self
            .client
            .post(format!("{}{PREFIX}/{endpoint}", self.origin))
            .json(&body)
            .send()
            .await
            .map_err(|_| E::AdmTargetUnavailable)?;
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(E::AdmAgentSuperseded);
        }
        if !response.status().is_success() {
            return Err(E::AdmTargetUnavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| E::AdmTargetUnavailable)?
        {
            if bytes.len() + chunk.len() > 1_048_576 {
                return Err(E::AdmLimitExceeded);
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        parse_json(&bytes).map_err(|_| E::AdmInvalidParams)
    }
}
/// Only implemented executors are advertised. G3 adds the other read executors.
pub fn advertised() -> BTreeSet<String> {
    BTreeSet::from([Operation::AdminPolicyGet.name().into()])
}
pub fn execute(operation: &str, policy: &Policy) -> Result<OperationResult, E> {
    if operation == Operation::AdminPolicyGet.name() {
        Ok(OperationResult::AdminPolicyGet(policy.view()))
    } else {
        Err(E::AdmUnknownOperation)
    }
}

pub struct Agent<T, S, C> {
    pub config: Config,
    pub policy: Policy,
    pub transport: T,
    pub store: S,
    pub clock: C,
    pub signing_key: SigningKey,
}
impl<T: Transport + Clone, S: NonceStore + IdempotencyStore, C: Clock + Clone> Agent<T, S, C> {
    fn clock_text(&self) -> Result<String, E> {
        OffsetDateTime::from_unix_timestamp(self.clock.now())
            .map_err(|_| E::AdmInternal)?
            .format(&Rfc3339)
            .map_err(|_| E::AdmInternal)
    }
    pub async fn hello(&self) -> Result<(), E> {
        let operations = advertised();
        self.transport.post("hello", json!({
            "agent_id":self.config.agent_id, "build_revision":self.config.build_revision,
            "catalog_digest":digest(&schemas().map_err(|_| E::AdmInternal)?).map_err(|_| E::AdmInternal)?,
            "operations":operations.iter().map(|op| json!({"operation":op,"version":1})).collect::<Vec<_>>(),
            "worlds":self.config.worlds,"nodes":self.config.nodes,"owner_epoch":self.config.owner_epoch,"clock":self.clock_text()?
        })).await?;
        Ok(())
    }
    pub async fn heartbeat(&self) -> Result<(), E> {
        self.transport.post("heartbeat", json!({"agent_id":self.config.agent_id,"owner_epoch":self.config.owner_epoch,"clock":self.clock_text()?})).await?;
        Ok(())
    }
    fn response(
        &self,
        request: &Request,
        result: Result<OperationResult, E>,
    ) -> Result<Response, E> {
        let (status, error, result) = match result {
            Ok(value) => (Status::Succeeded, None, Some(value)),
            Err(code) => (
                if code == E::AdmExpired {
                    Status::Expired
                } else {
                    Status::Rejected
                },
                Some(ErrorDetail {
                    code,
                    message: format!("{code:?}"),
                    retryable: code.retryable(),
                }),
                None,
            ),
        };
        let mut response = Response {
            request_id: request.request_id.clone(),
            admin_operation_id: request.admin_operation_id.clone(),
            status,
            error,
            result,
            dry_run_preview: None,
            game_ledger_id: Some(uuid::Uuid::now_v7().to_string()),
            audit_event_ids: Vec::new(),
            effective_revision: None,
            progress: None,
            completed_at: Some(self.clock_text()?),
            agent_id: self.config.agent_id.clone(),
            agent_build: self.config.build_revision.clone(),
            signature: String::new(),
        };
        response.signature = sign(&response, &self.signing_key).map_err(|_| E::AdmInternal)?;
        Ok(response)
    }
    async fn process_inner(&mut self, r: &Request) -> Result<Response, E> {
        let now = self.clock.now();
        let op = verifier::authenticate(r, &self.policy, &advertised(), now)?;
        // Step 4 is atomic across agents/restarts; a failed later check still burns the nonce.
        let nonce_seen = !self.store.claim_nonce(&r.nonce, now).await?;
        let evidence = Evidence {
            nonce_seen,
            ..Evidence::default()
        };
        let params_digest = verifier::authorize(r, &self.policy, op, &evidence, now)?;
        // Steps 5–8 succeeded before touching idempotency state.
        let prior = self.store.lookup(&r.admin_operation_id).await?;
        match verifier::idempotency(&params_digest, prior.as_ref())? {
            Decision::Recorded(response) => Ok(*response),
            Decision::Execute { params_digest } => {
                // Store I/O may have consumed the remaining lifetime.
                if crate::policy::timestamp(&r.expires_at)
                    .is_none_or(|expires| self.clock.now() >= expires)
                {
                    return Err(E::AdmExpired);
                }
                let response = self.response(r, execute(&r.operation, &self.policy))?;
                self.store.finish(r, &params_digest, &response, now).await
            }
        }
    }
    pub async fn process(&mut self, request: &Request) -> Result<Response, E> {
        match self.process_inner(request).await {
            Ok(response) => Ok(response),
            Err(error) => {
                let mut response = self.response(request, Err(error))?;
                // A failure is never retained as a successful idempotent effect.
                // If the DB itself is unavailable, report no ledger id rather than
                // claiming an audit row that could not be persisted.
                let params_digest = digest(&request.params).unwrap_or_default();
                if self
                    .store
                    .audit(request, &params_digest, &response, self.clock.now())
                    .await
                    .is_err()
                {
                    response.game_ledger_id = None;
                }
                response.signature =
                    sign(&response, &self.signing_key).map_err(|_| E::AdmInternal)?;
                Ok(response)
            }
        }
    }
    pub async fn deliver(&mut self, batch: Value) -> Result<(), E> {
        let requests = batch.as_array().ok_or(E::AdmInvalidParams)?;
        if requests.len() > 32 {
            return Err(E::AdmLimitExceeded);
        }
        for value in requests {
            let request: Request =
                serde_json::from_value(value.clone()).map_err(|_| E::AdmInvalidParams)?;
            let response = self.process(&request).await?;
            self.transport
                .post(
                    "results",
                    serde_json::to_value(response).map_err(|_| E::AdmInternal)?,
                )
                .await?;
        }
        Ok(())
    }
    /// Heartbeats run independently during both polling and batch processing.
    /// Any supersession response terminates the session immediately.
    pub async fn run(&mut self) -> Result<(), E> {
        self.hello().await?;
        let transport = self.transport.clone();
        let agent_id = self.config.agent_id.clone();
        let owner_epoch = self.config.owner_epoch;
        let clock = self.clock.clone();
        let heartbeat = async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let time = OffsetDateTime::from_unix_timestamp(clock.now())
                    .map_err(|_| E::AdmInternal)?
                    .format(&Rfc3339)
                    .map_err(|_| E::AdmInternal)?;
                transport
                    .post(
                        "heartbeat",
                        json!({"agent_id":agent_id,"owner_epoch":owner_epoch,"clock":time}),
                    )
                    .await?;
            }
        };
        let worker = async {
            loop {
                let body = json!({"agent_id":self.config.agent_id,"owner_epoch":self.config.owner_epoch,"wait_seconds":25,"limit":32});
                let batch = self.transport.post("channel", body).await?;
                self.deliver(batch).await?;
            }
        };
        tokio::select! {
            result = worker => result,
            result = heartbeat => result,
        }
    }
}
