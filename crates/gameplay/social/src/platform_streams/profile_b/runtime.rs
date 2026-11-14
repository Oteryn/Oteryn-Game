//! B1 per-online-account renewal and B2's reduction-only feed. No product behaviour is enabled here.
use super::{
    transport::{Client, Clients},
    *,
};
use crate::durability::{
    DurabilityRoot,
    platform_streams::profile_b_store::{Acceptance, View},
};
use crate::platform_streams::enforcement::transport::Failure;
use crate::premium::{PremiumConsumer, snapshot::canonical_uuid};
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::Notify, task::JoinHandle};
pub const CHANGES_POLL: Duration = Duration::from_secs(10);
pub fn retry_delay(attempt: u32, retry: Option<Duration>) -> Duration {
    crate::platform_streams::enforcement::transport::backoff(
        attempt,
        retry.map(|d| d.min(Duration::from_secs(60))),
    )
    .min(Duration::from_secs(60))
}
pub const REQUEST_LIMIT: u16 = 200;
pub trait Store: Send + Sync {
    fn load(&self, account: &str) -> impl Future<Output = Result<View, ()>> + Send;
    fn accept(
        &self,
        request: &Request,
        products: &[(String, u64)],
        raw: &[u8],
    ) -> impl Future<Output = Result<Acceptance, ()>> + Send;
    fn cursor(&self) -> impl Future<Output = Result<(Option<String>, bool), ()>> + Send;
    fn apply(
        &self,
        expected: Option<&str>,
        raw: &[u8],
    ) -> impl Future<Output = Result<bool, ()>> + Send;
    fn stop(&self) -> impl Future<Output = Result<(), ()>> + Send;
    fn known_hints(
        &self,
        accounts: &[String],
        products: &[(String, u64)],
    ) -> impl Future<Output = Result<Vec<Hint>, ()>> + Send;
}
impl Store for DurabilityRoot {
    async fn load(&self, account: &str) -> Result<View, ()> {
        self.load_profile_b(account).await.map_err(|_| ())
    }
    async fn accept(
        &self,
        request: &Request,
        products: &[(String, u64)],
        raw: &[u8],
    ) -> Result<Acceptance, ()> {
        self.accept_profile_b_answer(request, products, raw)
            .await
            .map_err(|_| ())
    }
    async fn cursor(&self) -> Result<(Option<String>, bool), ()> {
        self.profile_b_changes_cursor().await.map_err(|_| ())
    }
    async fn apply(&self, expected: Option<&str>, raw: &[u8]) -> Result<bool, ()> {
        self.apply_profile_b_changes(expected, raw)
            .await
            .map_err(|_| ())
    }
    async fn stop(&self) -> Result<(), ()> {
        self.stop_profile_b_changes().await.map_err(|_| ())
    }
    async fn known_hints(&self, a: &[String], p: &[(String, u64)]) -> Result<Vec<Hint>, ()> {
        self.profile_b_known_hints(a, p).await.map_err(|_| ())
    }
}
pub trait Peer: Send + Sync {
    fn authority(
        &self,
        request: &Request,
        products: &[(String, u64)],
    ) -> impl Future<Output = Result<Vec<u8>, Failure>> + Send;
    fn changes(
        &self,
        after: Option<String>,
        limit: u16,
    ) -> impl Future<Output = Result<Vec<u8>, Failure>> + Send;
    fn stop(&self);
}
impl Peer for Client {
    async fn authority(&self, r: &Request, p: &[(String, u64)]) -> Result<Vec<u8>, Failure> {
        self.read_authority(r, p).await
    }
    async fn changes(&self, a: Option<String>, l: u16) -> Result<Vec<u8>, Failure> {
        self.read_changes(a, l).await
    }
    fn stop(&self) {
        Client::stop(self);
    }
}
fn alarm(operation: &'static str, reason: &'static str) {
    oteryn_error_codes::emit(
        "oteryn-game-server",
        &oteryn_error_codes::Line::new(
            oteryn_error_codes::Level::Warn,
            "social",
            "profile_b_alarm",
        )
        .field(
            "operation",
            oteryn_error_codes::FieldValue::Token(operation),
        )
        .field("reason", oteryn_error_codes::FieldValue::Token(reason)),
    );
}
#[derive(Default)]
struct Account {
    answer: Option<Answer>,
    hints: HashMap<(String, u64), u64>,
    proven: bool,
    conflicting: bool,
}
#[derive(Default)]
pub struct Consumer {
    accounts: Mutex<HashMap<String, Account>>,
}
impl Consumer {
    fn merge(&self, account: &str, view: View, fresh: bool) {
        let mut accounts = self.accounts.lock().unwrap_or_else(PoisonError::into_inner);
        let v = accounts.entry(account.into()).or_default();
        if let Some(answer) = view.answer {
            let current = v.answer.as_ref().map_or(0, |a| a.authority_revision);
            if answer.authority_revision >= current {
                v.answer = Some(answer);
                if fresh {
                    v.proven = true;
                    v.conflicting = false;
                }
            }
        }
        for h in view.hints {
            let old = v
                .hints
                .entry((h.product_id, h.product_version))
                .or_default();
            *old = (*old).max(h.lifecycle_revision);
        }
    }
    pub fn hint(&self, h: &Hint) -> bool {
        let mut accounts = self.accounts.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(v) = accounts.get_mut(&h.account_id) {
            let accepted = v
                .answer
                .as_ref()
                .and_then(|a| {
                    a.entitlements.iter().find(|e| {
                        e.product_id == h.product_id && e.product_version == h.product_version
                    })
                })
                .map_or(0, |e| e.lifecycle_revision);
            let old = v
                .hints
                .entry((h.product_id.clone(), h.product_version))
                .or_default();
            if h.lifecycle_revision > (*old).max(accepted) {
                *old = h.lifecycle_revision;
                return true;
            }
        }
        false
    }
    fn conflict(&self, id: &str) {
        if let Some(v) = self
            .accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(id)
        {
            v.conflicting = true;
        }
    }
    fn class_at(&self, id: &str, product: &str, version: u64, now: Option<GameNow>) -> Evaluation {
        let accounts = self.accounts.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(v) = accounts.get(id).filter(|v| v.proven && !v.conflicting) else {
            return Evaluation::None;
        };
        if now.is_none_or(|n| {
            v.answer
                .as_ref()
                .and_then(|a| time(&a.authority_issued_at))
                .is_none_or(|issued| n.now_us.saturating_add(n.uncertainty_us) < issued)
        }) {
            return Evaluation::None;
        }
        let Some(e) = v.answer.as_ref().and_then(|a| {
            a.entitlements
                .iter()
                .find(|e| e.product_id == product && e.product_version == version)
        }) else {
            return Evaluation::None;
        };
        evaluate(
            e,
            now,
            v.hints
                .get(&(product.into(), version))
                .copied()
                .unwrap_or(0),
        )
    }
    /// Production reads always consult the existing NTP-disciplined clock; no answer anchors it.
    pub fn current(&self, id: &str, product: &str, version: u64) -> bool {
        let now = crate::premium::clock::system_trusted_now().and_then(|w| {
            let lower = i128::from(w.lower_us());
            let upper = i128::from(w.upper_us());
            Some(GameNow {
                now_us: i64::try_from((lower + upper) / 2).ok()?,
                uncertainty_us: i64::try_from((upper - lower + 1) / 2).ok()?,
            })
        });
        self.class_at(id, product, version, now) == Evaluation::Active
    }
}
/// Renew at or before the conservative cutoff, including commercially clipped leases.
/// The packet's 1..60s retry policy is shorter than §9.1's 15..120s maximums.
pub fn refresh_at(answer: &Answer, jitter_seconds: u64) -> Option<i64> {
    let issued = time(&answer.authority_issued_at)?;
    let cutoff = answer
        .entitlements
        .iter()
        .filter_map(|e| time(&e.authority_valid_until))
        .min()?;
    let refresh = answer
        .entitlements
        .iter()
        .filter_map(|e| time(&e.refresh_after))
        .min()?;
    Some(
        issued
            .saturating_add((cutoff - issued) / 2)
            .saturating_add((jitter_seconds.min(120) as i64) * 1_000_000)
            .min(refresh),
    )
}
#[derive(Debug, PartialEq, Eq)]
pub enum Refresh {
    Accepted(Answer),
    Stopped,
}
async fn failed(peer: &impl Peer, operation: &'static str, error: Failure) -> Result<(), Failure> {
    if matches!(error, Failure::Defect | Failure::Configuration) {
        peer.stop();
        alarm(
            operation,
            if error == Failure::Defect {
                "defect"
            } else {
                "configuration"
            },
        );
        return Ok(());
    }
    Err(error)
}
pub async fn refresh_once(
    store: &impl Store,
    consumer: &Consumer,
    peer: &impl Peer,
    request: &Request,
    products: &[(String, u64)],
) -> Result<Refresh, Failure> {
    if products.is_empty() {
        return Ok(Refresh::Stopped);
    }
    let raw = match peer.authority(request, products).await {
        Ok(raw) => raw,
        Err(error) => {
            failed(peer, "authority", error).await?;
            return Ok(Refresh::Stopped);
        }
    };
    let Some(Message::Answer(answer)) = Message::parse(&raw) else {
        failed(peer, "authority", Failure::Defect).await?;
        return Ok(Refresh::Stopped);
    };
    if !answer.bound_to(request, products) {
        failed(peer, "authority", Failure::Defect).await?;
        return Ok(Refresh::Stopped);
    }
    match store
        .accept(request, products, &raw)
        .await
        .map_err(|_| Failure::Retry(None))?
    {
        Acceptance::Accepted(view) => {
            consumer.merge(&request.account_id, view, true);
            Ok(Refresh::Accepted(answer))
        }
        Acceptance::Ignored(view) => {
            consumer.merge(&request.account_id, view, false);
            Err(Failure::Retry(None))
        }
        Acceptance::Conflict(view) => {
            consumer.merge(&request.account_id, view, false);
            consumer.conflict(&request.account_id);
            alarm("authority", "conflict");
            Err(Failure::Retry(None))
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Changes {
    More,
    Idle,
    Stopped,
}
pub async fn changes_once(
    store: &impl Store,
    peer: &impl Peer,
    mut reduce: impl FnMut(&Hint),
) -> Result<Changes, Failure> {
    let (after, stopped) = store.cursor().await.map_err(|_| Failure::Retry(None))?;
    if stopped {
        return Ok(Changes::Stopped);
    }
    let raw = match peer.changes(after.clone(), REQUEST_LIMIT).await {
        Ok(raw) => raw,
        Err(error) => {
            failed(peer, "changes", error).await?;
            store.stop().await.map_err(|_| Failure::Retry(None))?;
            return Ok(Changes::Stopped);
        }
    };
    let page = match Message::parse(&raw) {
        Some(Message::Page(p)) if p.items.len() <= usize::from(REQUEST_LIMIT) => p,
        _ => {
            failed(peer, "changes", Failure::Defect).await?;
            store.stop().await.map_err(|_| Failure::Retry(None))?;
            return Ok(Changes::Stopped);
        }
    };
    for hint in &page.items {
        reduce(hint);
    }
    store
        .apply(after.as_deref(), &raw)
        .await
        .map_err(|_| Failure::Retry(None))?;
    Ok(if page.items.is_empty() {
        Changes::Idle
    } else {
        Changes::More
    })
}
struct Schedule {
    wake: Arc<Notify>,
    task: JoinHandle<()>,
}
#[derive(Default)]
struct Schedules(Mutex<HashMap<[u8; 16], Schedule>>);
impl Drop for Schedules {
    fn drop(&mut self) {
        for (_, s) in self
            .0
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .drain()
        {
            s.task.abort();
        }
    }
}
struct Inner {
    root: DurabilityRoot,
    client: Option<Client>,
    products: Vec<(String, u64)>,
    consumer: Arc<Consumer>,
}
#[derive(Clone)]
pub struct Refresher {
    inner: Arc<Inner>,
    schedules: Arc<Schedules>,
}
impl Refresher {
    pub fn new(root: DurabilityRoot, client: Option<Client>, products: Vec<(String, u64)>) -> Self {
        Self {
            inner: Arc::new(Inner {
                root,
                client,
                products,
                consumer: Arc::default(),
            }),
            schedules: Arc::default(),
        }
    }
    pub fn consumer(&self) -> &Consumer {
        &self.inner.consumer
    }
    pub fn admit(&self, account: [u8; 16]) {
        if self.inner.products.is_empty()
            || self.inner.client.as_ref().is_none_or(Client::stopped)
            || !crate::durability::platform_streams::enforcement_wire::uuid7(&canonical_uuid(
                account,
            ))
        {
            return;
        }
        let mut schedules = self
            .schedules
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(s) = schedules.get(&account).filter(|s| !s.task.is_finished()) {
            s.wake.notify_one();
            return;
        }
        let wake = Arc::new(Notify::new());
        let task_wake = wake.clone();
        let inner = self.inner.clone();
        let task = oteryn_task_supervisor::spawn_supervised_with_heartbeat(
            "profile_b_authority",
            None,
            move |heartbeat| run_authority(inner, account, task_wake, heartbeat),
        );
        schedules.insert(account, Schedule { wake, task });
    }
    pub fn release(&self, account: [u8; 16]) {
        if let Some(s) = self
            .schedules
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&account)
        {
            s.task.abort();
        }
        self.inner
            .consumer
            .accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&canonical_uuid(account));
    }
    pub fn hint(&self, h: &Hint) {
        if !self
            .inner
            .products
            .iter()
            .any(|(id, v)| id == &h.product_id && *v == h.product_version)
            || !self.inner.consumer.hint(h)
        {
            return;
        }
        let id = crate::platform_streams::uuid(&h.account_id);
        if let Some(id) = id
            && let Some(s) = self
                .schedules
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&id)
        {
            s.wake.notify_one();
        }
    }
    #[cfg(test)]
    pub fn active_tasks(&self) -> usize {
        self.schedules
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}
fn now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|t| i64::try_from(t.as_micros()).ok())
        .unwrap_or(0)
}
async fn run_authority(
    inner: Arc<Inner>,
    account: [u8; 16],
    wake: Arc<Notify>,
    heartbeat: oteryn_task_supervisor::Heartbeat,
) {
    let Some(client) = &inner.client else {
        return;
    };
    let id = canonical_uuid(account);
    // Reserve before loading so a concurrent hint cannot be lost by this process.
    inner
        .consumer
        .accounts
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(id.clone())
        .or_default();
    if let Ok(view) = Store::load(&inner.root, &id).await {
        inner.consumer.merge(&id, view, false);
    }
    let mut attempt = 0;
    loop {
        heartbeat.beat();
        if client.stopped() {
            return;
        }
        let nonce = crate::premium::client::fresh_nonce();
        let result = match nonce.as_ref() {
            Some(nonce) => {
                refresh_once(
                    &inner.root,
                    &inner.consumer,
                    client,
                    &Request {
                        schema: "oteryn.entitlement_authority_request.v1".into(),
                        account_id: id.clone(),
                        nonce: nonce.clone(),
                    },
                    &inner.products,
                )
                .await
            }
            None => Err(Failure::Retry(None)),
        };
        let delay = match result {
            Ok(Refresh::Stopped) => return,
            Ok(Refresh::Accepted(answer)) => {
                attempt = 0;
                let jitter = nonce
                    .as_deref()
                    .and_then(|n| u64::from_str_radix(&n[..8], 16).ok())
                    .unwrap_or(0)
                    % 121;
                Duration::from_micros(
                    u64::try_from(
                        refresh_at(&answer, jitter)
                            .unwrap_or_else(now_us)
                            .saturating_sub(now_us()),
                    )
                    .unwrap_or(0),
                )
                .max(Duration::from_secs(1))
            }
            Err(error) => {
                let retry = match error {
                    Failure::Retry(r) => r,
                    _ => None,
                };
                let delay = retry_delay(attempt, retry);
                attempt = attempt.saturating_add(1);
                delay
            }
        };
        let _idle = heartbeat.expected_idle();
        let _ = tokio::time::timeout(delay, wake.notified()).await;
    }
}
pub async fn sync_known_hints(
    store: &impl Store,
    accounts: &[String],
    products: &[(String, u64)],
    mut reduce: impl FnMut(&Hint),
) -> Result<(), Failure> {
    for accounts in accounts.chunks(500) {
        for hint in store
            .known_hints(accounts, products)
            .await
            .map_err(|_| Failure::Retry(None))?
        {
            reduce(&hint);
        }
    }
    Ok(())
}
#[derive(Default)]
pub struct Tasks(Option<JoinHandle<()>>);
impl Tasks {
    pub fn spawn(
        root: DurabilityRoot,
        clients: Option<Clients>,
        premium: Arc<PremiumConsumer>,
    ) -> Self {
        let Some(clients) = clients else {
            return Self::default();
        };
        let refresher = Arc::new(Refresher::new(
            root.clone(),
            clients.authority,
            clients.products,
        ));
        premium.install_profile_b(refresher.clone());
        let Some(client) = clients.changes else {
            return Self::default();
        };
        Self(Some(
            oteryn_task_supervisor::spawn_supervised_with_heartbeat(
                "profile_b_changes",
                None,
                move |heartbeat| async move {
                    let mut attempt = 0;
                    loop {
                        heartbeat.beat();
                        if client.stopped() {
                            return;
                        }
                        let reduce = |hint: &Hint| {
                            refresher.hint(hint);
                            if hint.product_id == crate::premium::PRODUCT_ID
                                && hint.product_version == 1
                                && let Some(id) = crate::platform_streams::uuid(&hint.account_id)
                            {
                                premium.profile_b_hint(id, hint.lifecycle_revision);
                            }
                        };
                        let mut accounts = refresher
                            .inner
                            .consumer
                            .accounts
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>();
                        accounts.extend(
                            premium
                                .profile_b_known_accounts()
                                .into_iter()
                                .map(canonical_uuid)
                                .filter(|id| {
                                    crate::durability::platform_streams::enforcement_wire::uuid7(id)
                                }),
                        );
                        accounts.sort_unstable();
                        accounts.dedup();
                        let mut products = refresher.inner.products.clone();
                        products.push((crate::premium::PRODUCT_ID.into(), 1));
                        let result =
                            match sync_known_hints(&root, &accounts, &products, reduce).await {
                                Ok(()) => changes_once(&root, &client, reduce).await,
                                Err(error) => Err(error),
                            };
                        let delay = match result {
                            Ok(Changes::Stopped) => return,
                            Ok(Changes::More) => {
                                attempt = 0;
                                Duration::ZERO
                            }
                            Ok(Changes::Idle) => {
                                attempt = 0;
                                CHANGES_POLL
                            }
                            Err(error) => {
                                let retry = match error {
                                    Failure::Retry(r) => r,
                                    _ => None,
                                };
                                let delay = retry_delay(attempt, retry);
                                attempt = attempt.saturating_add(1);
                                delay
                            }
                        };
                        let _idle = heartbeat.expected_idle();
                        tokio::time::sleep(delay).await;
                    }
                },
            ),
        ))
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}
impl Drop for Tasks {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}

impl std::fmt::Debug for Refresher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileBRefresher")
            .field("product_count", &self.inner.products.len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
