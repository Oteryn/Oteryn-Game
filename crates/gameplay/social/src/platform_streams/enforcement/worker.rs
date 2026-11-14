//! Bounded enforcement passes, shared by supervised production tasks and deterministic tests.
use super::transport::{Clients, Failure, Route, backoff};
use super::*;
pub use crate::durability::platform_streams::enforcement_transport::{
    Action, Delivery, Publication, Reply,
};
use std::{future::Future, time::Duration};

pub trait Store: Send + Sync {
    fn alarm(&self, reason: &'static str) {
        alarm(reason);
    }
    fn transact(&self, action: Action) -> impl Future<Output = Result<Reply, ()>> + Send;
    fn decide(&self, raw: Vec<u8>) -> impl Future<Output = Result<Vec<u8>, ()>> + Send;
}
pub trait Peer: Send + Sync {
    fn request(
        &self,
        route: Route,
        raw: Vec<u8>,
    ) -> impl Future<Output = Result<Vec<u8>, Failure>> + Send;
}
impl Peer for Clients {
    async fn request(&self, route: Route, raw: Vec<u8>) -> Result<Vec<u8>, Failure> {
        self.exchange(route, raw).await
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    More,
    Idle,
    Stopped,
}
fn alarm(reason: &'static str) {
    if reason == "drift" {
        oteryn_error_codes::metrics::Registry::global()
            .counter("enforcement_reconcile_drift_total", &[])
            .increment();
    }
    oteryn_error_codes::emit(
        "oteryn-game-server",
        &oteryn_error_codes::Line::new(
            oteryn_error_codes::Level::Warn,
            "social",
            "enforcement_alarm",
        )
        .field("reason", oteryn_error_codes::FieldValue::Token(reason)),
    );
}
fn request(schema: &str, after: Option<String>) -> Result<Vec<u8>, Failure> {
    let request = Message::Request(StreamRequest {
        schema: schema.into(),
        after,
        limit: 200,
    });
    request.encode().ok_or(Failure::Defect)
}
async fn cursor(store: &impl Store, reconcile: bool) -> Result<(Option<String>, bool), Failure> {
    match store
        .transact(Action::Cursor { reconcile })
        .await
        .map_err(|_| Failure::Retry(None))?
    {
        Reply::Cursor(cursor, stopped) => Ok((cursor, stopped)),
        _ => Err(Failure::Defect),
    }
}
async fn terminal(store: &impl Store, reconcile: bool, failure: Failure) -> Result<Step, Failure> {
    match failure {
        Failure::Defect | Failure::Configuration | Failure::Missing | Failure::Conflict => {
            store
                .transact(Action::Stop { reconcile })
                .await
                .map_err(|_| Failure::Retry(None))?;
            store.alarm(match failure {
                Failure::Defect => "defect",
                Failure::Configuration => "configuration",
                Failure::Missing => "missing",
                Failure::Conflict => "conflict",
                Failure::Retry(_) => "transport",
            });
            Ok(Step::Stopped)
        }
        other => Err(other),
    }
}
pub async fn pull(store: &impl Store, peer: &impl Peer) -> Result<Step, Failure> {
    let (after, stopped) = cursor(store, false).await?;
    if stopped {
        return Ok(Step::Stopped);
    }
    let raw = match peer
        .request(
            Route::Stream,
            request(
                "oteryn.enforcement_command_stream_request.v1",
                after.clone(),
            )?,
        )
        .await
    {
        Ok(raw) => raw,
        Err(error) => return terminal(store, false, error).await,
    };
    let Some(Message::Page(page)) = Message::parse(&raw) else {
        return terminal(store, false, Failure::Defect).await;
    };
    for command in page.items {
        store
            .decide(command.encode().ok_or(Failure::Defect)?)
            .await
            .map_err(|_| Failure::Retry(None))?;
    }
    let more = page.next_cursor.is_some();
    store
        .transact(Action::Advance {
            reconcile: false,
            expected: after,
            next: page.next_cursor,
        })
        .await
        .map_err(|_| Failure::Retry(None))?;
    Ok(if more { Step::More } else { Step::Idle })
}
pub async fn publish(store: &impl Store, peer: &impl Peer, _now_ms: i64) -> Result<Step, Failure> {
    let publication = match store
        .transact(Action::Next)
        .await
        .map_err(|_| Failure::Retry(None))?
    {
        Reply::Publication(Some(item)) => item,
        Reply::Publication(None) => return Ok(Step::Idle),
        _ => return Err(Failure::Defect),
    };
    let delivery = match peer.request(Route::Results, publication.bytes).await {
        Ok(raw) => {
            if !matches!(Message::parse(&raw), Some(Message::Ack(_))) {
                return Err(Failure::Retry(None));
            }
            Delivery::Ack
        }
        Err(Failure::Conflict) => {
            store.alarm("conflict");
            Delivery::Hold
        }
        Err(Failure::Missing) => {
            store.alarm("missing");
            Delivery::Missing
        }
        Err(Failure::Defect) => {
            store.alarm("defect");
            return Err(Failure::Retry(None));
        }
        Err(Failure::Configuration) => {
            store.alarm("configuration");
            return Err(Failure::Retry(None));
        }
        Err(other) => return Err(other),
    };
    store
        .transact(Action::Mark {
            operation: publication.operation,
            revision: publication.revision,
            delivery,
        })
        .await
        .map_err(|_| Failure::Retry(None))?;
    Ok(Step::More)
}
pub async fn reconcile(store: &impl Store, peer: &impl Peer) -> Result<Step, Failure> {
    let (after, stopped) = cursor(store, true).await?;
    if stopped {
        return Ok(Step::Stopped);
    }
    // Reconcile requests share the same ordered request shape, with their own schema.
    let body = Message::Request(StreamRequest {
        schema: "oteryn.enforcement_reconcile_request.v1".into(),
        after: after.clone(),
        limit: 200,
    })
    .encode()
    .ok_or(Failure::Defect)?;
    let raw = match peer.request(Route::Reconcile, body).await {
        Ok(raw) => raw,
        Err(error) => return terminal(store, true, error).await,
    };
    let Some(Message::Reconcile(page)) = Message::parse(&raw) else {
        return terminal(store, true, Failure::Defect).await;
    };
    for item in page.items {
        let stored = match store
            .transact(Action::Lookup {
                operation: item.operation_id.clone(),
            })
            .await
            .map_err(|_| Failure::Retry(None))?
        {
            Reply::Bytes(raw) => raw,
            _ => return Err(Failure::Defect),
        };
        if let Some(raw) = stored {
            let result = EnforcementResult::parse(&raw).ok_or(Failure::Defect)?;
            if result.sanction_stream_id != item.sanction_stream_id {
                store.alarm("drift");
                continue;
            }
            if item.state == "PENDING" {
                store
                    .transact(Action::Mark {
                        operation: item.operation_id,
                        revision: result.receipt_revision,
                        delivery: Delivery::Republish,
                    })
                    .await
                    .map_err(|_| Failure::Retry(None))?;
            } else if item.result_digest.as_deref() != Some(digest(&raw).as_str()) {
                store.alarm("drift");
            }
        } else {
            let body = Message::Read(ReadRequest {
                schema: "oteryn.enforcement_command_read_request.v1".into(),
                operation_id: item.operation_id.clone(),
            })
            .encode()
            .ok_or(Failure::Defect)?;
            let raw = match peer.request(Route::Read, body).await {
                Ok(raw) => raw,
                Err(error) => return terminal(store, true, error).await,
            };
            let command = match Command::parse(&raw) {
                Some(command) => command,
                None => return terminal(store, true, Failure::Defect).await,
            };
            if command.operation_id != item.operation_id
                || command.sanction_stream_id != item.sanction_stream_id
            {
                return terminal(store, true, Failure::Defect).await;
            }
            store.decide(raw).await.map_err(|_| Failure::Retry(None))?;
        }
    }
    let more = page.next_cursor.is_some();
    store
        .transact(Action::Advance {
            reconcile: true,
            expected: after,
            next: page.next_cursor,
        })
        .await
        .map_err(|_| Failure::Retry(None))?;
    Ok(if more { Step::More } else { Step::Idle })
}
/// Scan independently of outbox throughput; timestamps are injected for deterministic tests.
pub const OVERDUE_SCAN_INTERVAL_MS: i64 = 60_000;
#[derive(Default)]
pub struct Publisher {
    next_alarm_scan_ms: Option<i64>,
}
impl Publisher {
    pub async fn step(
        &mut self,
        store: &impl Store,
        peer: &impl Peer,
        now_ms: i64,
    ) -> Result<Step, Failure> {
        if self.next_alarm_scan_ms.is_none_or(|next| now_ms >= next) {
            self.next_alarm_scan_ms = Some(now_ms.saturating_add(OVERDUE_SCAN_INTERVAL_MS));
            if let Reply::Alarms(count) = store
                .transact(Action::Overdue { now_ms })
                .await
                .map_err(|_| Failure::Retry(None))?
            {
                for _ in 0..count {
                    store.alarm("overdue");
                }
            }
        }
        publish(store, peer, now_ms).await
    }
}
#[derive(Clone, Copy)]
pub enum Task {
    Pull,
    Publish,
    Reconcile,
}
pub async fn run(
    store: &impl Store,
    peer: &impl Peer,
    task: Task,
    heartbeat: oteryn_task_supervisor::Heartbeat,
) {
    let mut attempt = 0;
    let mut publisher = Publisher::default();
    loop {
        heartbeat.beat();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|v| i64::try_from(v.as_millis()).ok())
            .unwrap_or(0);
        let result = match task {
            Task::Pull => pull(store, peer).await,
            Task::Publish => publisher.step(store, peer, now).await,
            Task::Reconcile => reconcile(store, peer).await,
        };
        let delay = match result {
            Ok(Step::Stopped) => return,
            Ok(Step::More) => {
                attempt = 0;
                Duration::ZERO
            }
            Ok(Step::Idle) => {
                attempt = 0;
                Duration::from_secs(5)
            }
            Err(error) => {
                let retry = match error {
                    Failure::Retry(retry) => retry,
                    _ => None,
                };
                let wait = backoff(attempt, retry);
                attempt = attempt.saturating_add(1);
                wait
            }
        };
        let _idle = heartbeat.expected_idle();
        tokio::time::sleep(delay).await;
    }
}

#[cfg(test)]
mod tests;

/// Bounded requests keep the non-'static sealed Character authority in its serving owner.
/// Network tasks own only this queue; they cannot mint or retain an authority capability.
pub enum StoreRequest {
    Transact(Action, tokio::sync::oneshot::Sender<Result<Reply, ()>>),
    Decide(Vec<u8>, tokio::sync::oneshot::Sender<Result<Vec<u8>, ()>>),
}
#[derive(Clone)]
pub struct ChannelStore {
    sender: tokio::sync::mpsc::Sender<StoreRequest>,
}
pub fn store_channel() -> (ChannelStore, tokio::sync::mpsc::Receiver<StoreRequest>) {
    let (sender, receiver) = tokio::sync::mpsc::channel(3);
    (ChannelStore { sender }, receiver)
}
impl Store for ChannelStore {
    async fn transact(&self, action: Action) -> Result<Reply, ()> {
        let (send, receive) = tokio::sync::oneshot::channel();
        self.sender
            .send(StoreRequest::Transact(action, send))
            .await
            .map_err(|_| ())?;
        receive.await.map_err(|_| ())?
    }
    async fn decide(&self, raw: Vec<u8>) -> Result<Vec<u8>, ()> {
        let (send, receive) = tokio::sync::oneshot::channel();
        self.sender
            .send(StoreRequest::Decide(raw, send))
            .await
            .map_err(|_| ())?;
        receive.await.map_err(|_| ())?
    }
}
/// Same supervised spawn and abort-on-drop lifetime as the existing Platform stream pullers.
#[derive(Default)]
pub struct Tasks(Vec<tokio::task::JoinHandle<()>>);
impl Tasks {
    pub fn spawn(clients: Option<Clients>, store: ChannelStore) -> Self {
        let Some(clients) = clients else {
            return Self::default();
        };
        Self(
            [
                (Task::Pull, "platform_enforcement_pull"),
                (Task::Reconcile, "platform_enforcement_reconcile"),
                (Task::Publish, "platform_enforcement_results"),
            ]
            .into_iter()
            .map(|(task, name)| {
                let store = store.clone();
                let clients = clients.clone();
                oteryn_task_supervisor::spawn_supervised_with_heartbeat(
                    name,
                    None,
                    move |heartbeat| async move { run(&store, &clients, task, heartbeat).await },
                )
            })
            .collect(),
        )
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl Drop for Tasks {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}
