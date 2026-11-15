//! Bounded off-tick command owner. Its task owns the cast state exclusively;
//! neither world locks nor native actors are retained across storage/network IO.
use super::*;
use oteryn_gameplay_durability::durability::{
    DurabilityError, DurabilityRoot,
    cast_privacy::{CastConsentBatch, CastConsentWorker},
    runtime_scope_assignment::NodeIncarnationProof,
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub type CastWriter = Box<dyn FnMut(State, &Content) -> bool + Send>;

pub struct CapturePermit {
    cast_id: [u8; 16],
    target: GameSessionId,
    epoch: u64,
    privacy_version: i64,
    expires_ms: u64,
}

pub enum Operation {
    Start(TargetFacts, Settings, [u8; 16]),
    Update(TargetFacts, u64, Settings),
    Stop(TargetFacts, u64),
    Join(VerifiedAdmission),
    Renew(ObserverId, VerifiedAdmission),
    Leave(ObserverId),
    Keepalive(ObserverId),
    Chat(ObserverId, String),
    Reply(TargetFacts, String),
    Ban(TargetFacts, u64, ObserverId),
    PublishView(
        CapturePermit,
        TargetFacts,
        Vec<CharacterId>,
        Scene,
        Vec<oteryn_protocol_oteryn::cast::PublicMedia>,
        Vec<(CharacterId, CastConsent)>,
    ),
    Mute(TargetFacts, u64, ObserverId, bool),
    Kick(TargetFacts, u64, ObserverId),
    Consent(CharacterId, CastConsent),
    Capture(TargetFacts),
    Publish(
        CapturePermit,
        TargetFacts,
        Vec<CharacterId>,
        Scene,
        Vec<(CharacterId, CastConsent)>,
    ),
    Slate(TargetFacts),
    Gone(GameSessionId),
    TransportGone(
        GameSessionId,
        oteryn_gameplay_foundation::foundation::AuthenticatedTransportRefV1,
    ),
    Refresh(TargetFacts),
    ObserverState(ObserverId),
    CasterRoster(GameSessionId),
    State(GameSessionId),
    Invalidate(CharacterId, i64),
    CasterChat(GameSessionId),
    CasterPresence(GameSessionId),
    Shutdown,
    /// Synchronous final writes run on the worker, serialized with revocation.
    Flush(ObserverId, CastWriter),
}
pub enum Reply {
    Roster(Option<(u64, Vec<[u8; 16]>)>),
    State(State),
    Capture(CapturePermit),
    Observer(ObserverId),
    Chat(Vec<ChatLine>),
    Applied,
}
struct Request {
    submitted_ms: u64,
    operation: Operation,
    reply: oneshot::Sender<Result<Reply, Error>>,
}
// External work can take longer than the media lease, including a sequence of
// individually fast calls. Keep polling both futures, so a root holder owned by
// the external work can still make progress while renewal attempts run.
struct PrivacyIo<'a> {
    root: &'a DurabilityRoot,
    node: &'a NodeIncarnationProof,
    worker: CastConsentWorker,
    pulse: std::time::Instant,
    pending: Option<CastConsentBatch>,
}
enum First<A, B> {
    Left(A),
    Right(B),
}
async fn first<A: Future, B: Future>(
    mut left: std::pin::Pin<&mut A>,
    mut right: std::pin::Pin<&mut B>,
) -> First<A::Output, B::Output> {
    std::future::poll_fn(|cx| {
        if let std::task::Poll::Ready(value) = left.as_mut().poll(cx) {
            return std::task::Poll::Ready(First::Left(value));
        }
        right.as_mut().poll(cx).map(First::Right)
    })
    .await
}
impl PrivacyIo<'_> {
    fn expire(&self, owner: &mut CastOwner) {
        if !self.worker.valid() {
            for target in owner.casts.keys().copied().collect::<Vec<_>>() {
                owner.target_gone(target);
            }
        }
    }
    async fn apply_batch(
        &mut self,
        owner: &mut CastOwner,
        batch: CastConsentBatch,
    ) -> Result<(), Error> {
        self.expire(owner);
        for event in batch.events() {
            // Unknown characters have no queued frames. Captures made before
            // this ACK cannot publish at the new privacy version.
            if owner.consents.contains_key(&event.character)
                || owner.consent_floor.contains_key(&event.character)
            {
                owner.invalidate(event.character, event.minimum_revision - 1)?;
            }
        }
        if !batch.is_empty() {
            self.root
                .acknowledge_cast_consent(self.node, &mut self.worker, batch)
                .await
                .map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }
    async fn apply_pending(&mut self, owner: &mut CastOwner) -> Result<(), Error> {
        if let Some(batch) = self.pending.take() {
            self.apply_batch(owner, batch).await?;
        }
        self.expire(owner);
        Ok(())
    }
    async fn refresh(&mut self, owner: &mut CastOwner) -> Result<(), Error> {
        self.expire(owner);
        if !self.worker.valid() {
            return Ok(());
        }
        self.pulse = std::time::Instant::now();
        if let Ok(batch) = self
            .root
            .refresh_cast_consent_worker(self.node, &mut self.worker)
            .await
        {
            self.apply_batch(owner, batch).await?;
        }
        self.expire(owner);
        Ok(())
    }
    // HTTP futures retain no cast state, so revocations can be applied and
    // acknowledged under the exclusive owner while a response is outstanding.
    async fn external<F: Future>(
        &mut self,
        owner: &mut CastOwner,
        work: F,
    ) -> Result<F::Output, Error> {
        self.apply_pending(owner).await?;
        let mut work = std::pin::pin!(work);
        loop {
            if !self.worker.valid() {
                self.expire(owner);
                return Ok(work.await);
            }
            let next = self.pulse + std::time::Duration::from_millis(250);
            let mut timer = std::pin::pin!(tokio::time::sleep_until(
                tokio::time::Instant::from_std(next)
            ));
            match first(timer.as_mut(), work.as_mut()).await {
                First::Right(result) => {
                    self.expire(owner);
                    return Ok(result);
                }
                First::Left(()) => {
                    let (refreshed, completed) = {
                        let mut refresh = std::pin::pin!(self.refresh(owner));
                        match first(refresh.as_mut(), work.as_mut()).await {
                            First::Right(result) => (refresh.await, Some(result)),
                            First::Left(result) => (result, None),
                        }
                    };
                    if let Err(error) = refreshed {
                        // Finish external side effects before shutting down;
                        // their outcome must not be lost by cancellation.
                        if completed.is_none() {
                            let _ = work.await;
                        }
                        return Err(error);
                    }
                    if let Some(result) = completed {
                        return Ok(result);
                    }
                }
            }
        }
    }
    fn apply_current(
        &self,
        owner: &mut CastOwner,
        operation: Operation,
        now: u64,
    ) -> Result<Reply, Error> {
        if !self.worker.valid() {
            return Err(Error::Unavailable);
        }
        apply(owner, operation, now, self.worker.version())
    }
    async fn apply_once(
        &mut self,
        owner: &mut CastOwner,
        access: &impl preferences::PreferencesStore,
        accounts: &HashMap<GameSessionId, [u8; 16]>,
        operation: Operation,
        now: u64,
    ) -> Result<DurableOutcome, Error> {
        match operation {
            Operation::Start(facts, settings, account) => {
                let policy = self
                    .external(owner, preferences::authorize(access, account, settings))
                    .await?;
                Ok(DurableOutcome::Complete(policy.and_then(|policy| {
                    self.apply_current(
                        owner,
                        Operation::Start(facts, policy.settings, account),
                        now,
                    )
                })))
            }
            Operation::Update(facts, revision, settings) => {
                let Some(account) = accounts
                    .get(&facts.authority.current_game_session_id())
                    .copied()
                else {
                    return Ok(DurableOutcome::Complete(Err(Error::Unavailable)));
                };
                let policy = self
                    .external(owner, preferences::authorize(access, account, settings))
                    .await?;
                Ok(DurableOutcome::Complete(policy.and_then(|policy| {
                    self.apply_current(
                        owner,
                        Operation::Update(facts, revision, policy.settings),
                        now,
                    )
                })))
            }
            operation => {
                let root = self.root;
                let node = self.node;
                let version = self.worker.version();
                let result = self
                    .during(apply_durable_once(
                        owner, root, node, access, accounts, operation, now, version,
                    ))
                    .await;
                self.apply_pending(owner).await?;
                Ok(result)
            }
        }
    }
    async fn persist_pending(&mut self, owner: &mut CastOwner) -> Result<(), Error> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
        while !owner.audit.pending.is_empty() {
            self.during(owner.persist_audit_batch(self.root, self.node, deadline))
                .await?;
            self.apply_pending(owner).await?;
            // The audit transaction has released the holder here. Renewal must
            // have an opportunity before the next batch acquires it again.
            if self.pending.is_none()
                && self.worker.valid()
                && self
                    .worker
                    .valid_until()
                    .saturating_duration_since(std::time::Instant::now())
                    <= std::time::Duration::from_millis(750)
            {
                self.pulse = std::time::Instant::now();
                if let Ok(batch) = self
                    .root
                    .try_refresh_cast_consent_worker(self.node, &mut self.worker)
                    .await
                    && !batch.is_empty()
                {
                    self.pending = Some(batch);
                }
                self.apply_pending(owner).await?;
            }
        }
        Ok(())
    }
    async fn during<F: Future>(&mut self, work: F) -> F::Output {
        let mut work = std::pin::pin!(work);
        loop {
            // Never ACK or renew past a withdrawal before the exclusive cast
            // owner has applied it. Final media writes retain the old deadline.
            // An expired lease must reach the outer loop's cast termination
            // before that loop attempts recovery.
            if self.pending.is_some() || !self.worker.valid() {
                return work.await;
            }
            let next = self.pulse + std::time::Duration::from_millis(250);
            let mut timer = std::pin::pin!(tokio::time::sleep_until(
                tokio::time::Instant::from_std(next)
            ));
            match first(timer.as_mut(), work.as_mut()).await {
                First::Right(result) => return result,
                First::Left(()) => {
                    if !self.worker.valid() {
                        return work.await;
                    }
                    self.pulse = std::time::Instant::now();
                    let (refreshed, completed) = {
                        let mut refresh = std::pin::pin!(
                            self.root
                                .try_refresh_cast_consent_worker(self.node, &mut self.worker)
                        );
                        match first(refresh.as_mut(), work.as_mut()).await {
                            // Finish an issued durable pass even if the external
                            // work completes first; cancellation could lose its receipt.
                            First::Right(result) => (refresh.await, Some(result)),
                            First::Left(result) => (result, None),
                        }
                    };
                    if let Ok(batch) = refreshed
                        && !batch.is_empty()
                    {
                        self.pending = Some(batch);
                    }
                    if let Some(result) = completed {
                        return result;
                    }
                }
            }
        }
    }
}
#[derive(Clone)]
pub struct CastService {
    tx: mpsc::Sender<Request>,
    origin: std::time::Instant,
}
impl CastService {
    pub fn spawn(
        root: DurabilityRoot,
        node: NodeIncarnationProof,
        capacity: usize,
        access: Arc<access::CastAccessClient>,
    ) -> Result<Self, Error> {
        let owner = CastOwner::new(capacity)?;
        let (tx, rx) = mpsc::channel(capacity.min(256));
        let origin = std::time::Instant::now();
        oteryn_task_supervisor::spawn_supervised_with_heartbeat(
            "cast_service",
            None,
            move |heartbeat| run(owner, root, node, access, rx, origin, heartbeat),
        );
        Ok(Self { tx, origin })
    }
    pub fn clock(&self) -> (u64, u64) {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        (
            unix,
            u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX),
        )
    }
    pub async fn apply(&self, operation: Operation) -> Result<Reply, Error> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .try_send(Request {
                submitted_ms: self.clock().1,
                operation,
                reply,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::Full,
                mpsc::error::TrySendError::Closed(_) => Error::Unavailable,
            })?;
        rx.await.map_err(|_| Error::Unavailable)?
    }
}
async fn run(
    mut owner: CastOwner,
    root: DurabilityRoot,
    node: NodeIncarnationProof,
    access: Arc<access::CastAccessClient>,
    mut rx: mpsc::Receiver<Request>,
    clock: std::time::Instant,
    heartbeat: oteryn_task_supervisor::Heartbeat,
) {
    // Canary livestream.cpp:249 starts the live service explicitly. S17 additionally
    // requires a consent-worker lease: retry only a busy durable holder, never
    // process a queued Start before registration or after the bounded deadline.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let privacy = loop {
        match root
            .register_cast_consent_worker(&node, owner.audit.incarnation)
            .await
        {
            Ok(worker) => break worker,
            Err(oteryn_gameplay_durability::durability::DurabilityError::RootUnavailable)
                if tokio::time::Instant::now() < deadline =>
            {
                let now = tokio::time::Instant::now();
                root.wait_holder_retry(
                    now + std::time::Duration::from_millis(2),
                    (now + std::time::Duration::from_millis(50)).min(deadline),
                )
                .await;
            }
            Err(_) => return,
        }
    };
    let mut privacy = PrivacyIo {
        root: &root,
        node: &node,
        worker: privacy,
        pulse: std::time::Instant::now(),
        pending: None,
    };
    let mut accounts = std::collections::HashMap::new();
    let mut last_announce = 0u64;
    let mut announced = std::collections::HashSet::new();
    let mut freshness = std::collections::HashMap::new();
    let mut pending = None;
    'worker: loop {
        heartbeat.beat();
        let retrying = pending.is_some();
        let mut request = if retrying {
            pending.take()
        } else {
            match tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await {
                Ok(Some(request)) => Some(request),
                Ok(None) => break,
                Err(_) => None,
            }
        };
        let now = u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        privacy.expire(&mut owner);
        if privacy.pending.is_some()
            || privacy.pulse.elapsed() >= std::time::Duration::from_millis(250)
            || !privacy.worker.valid()
        {
            privacy.pulse = std::time::Instant::now();
            // Observer storage retries must reach their existing rejection boundary:
            // their caller can be holding the root until it receives that reply.
            let observer_storage = request.as_ref().is_some_and(|request| {
                matches!(
                    &request.operation,
                    Operation::Join(_) | Operation::Renew(..) | Operation::Ban(..)
                )
            });
            let batch = if let Some(batch) = privacy.pending.take() {
                Ok(batch)
            } else if observer_storage {
                root.try_refresh_cast_consent_worker(&node, &mut privacy.worker)
                    .await
            } else {
                root.refresh_cast_consent_worker(&node, &mut privacy.worker)
                    .await
            };
            if let Ok(batch) = batch
                && privacy.apply_batch(&mut owner, batch).await.is_err()
            {
                break 'worker;
            }
        }
        if !privacy.worker.valid() {
            if request
                .as_ref()
                .is_some_and(|r| matches!(r.operation, Operation::Shutdown))
            {
                if let Some(request) = request.take() {
                    let _ = request.reply.send(Ok(Reply::Applied));
                }
                break;
            }
            if let Some(request) = request.take() {
                let _ = request.reply.send(Err(Error::Unavailable));
            }
            let _ = privacy.persist_pending(&mut owner).await;
            continue;
        }
        let result = owner.poll(now);
        if result.is_err() {
            break;
        }
        for target in freshness
            .iter()
            .filter_map(|(target, seen)| (now.saturating_sub(*seen) >= 5000).then_some(*target))
            .collect::<Vec<_>>()
        {
            owner.target_gone(target);
            freshness.remove(&target);
        }
        if request.as_ref().is_some_and(|r| {
            now.saturating_sub(r.submitted_ms) > 1000
                || (retrying
                    && privacy
                        .worker
                        .valid_until()
                        .saturating_duration_since(std::time::Instant::now())
                        <= std::time::Duration::from_millis(600))
        }) {
            if let Some(expired) = request.take() {
                let _ = expired.reply.send(Err(Error::Unavailable));
            }
            let _ = privacy.persist_pending(&mut owner).await;
            continue;
        }
        let mut force_announce = false;
        let mut completed = None;
        let mut ending = false;
        if let Some(request) = request {
            match &request.operation {
                Operation::Start(f, _, _)
                | Operation::Update(f, _, _)
                | Operation::Refresh(f)
                | Operation::Publish(_, f, _, _, _)
                | Operation::PublishView(_, f, _, _, _, _)
                | Operation::Capture(f)
                | Operation::Slate(f) => {
                    freshness.insert(f.authority.current_game_session_id(), now);
                }
                _ => {}
            }
            if let Operation::Start(f, _, account) = &request.operation {
                accounts.insert(f.authority.current_game_session_id(), *account);
            }
            force_announce = matches!(
                &request.operation,
                Operation::Start(..) | Operation::Update(..)
            );
            ending = matches!(&request.operation, Operation::Shutdown);
            let operation = match request.operation {
                Operation::Flush(id, mut write) => {
                    let until = privacy.worker.valid_until();
                    Operation::Flush(
                        id,
                        Box::new(move |state, content| {
                            std::time::Instant::now() < until && write(state, content)
                        }),
                    )
                }
                operation => operation,
            };
            let result = match privacy
                .apply_once(
                    &mut owner,
                    access.as_ref(),
                    &accounts,
                    operation,
                    u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX),
                )
                .await
            {
                Ok(result) => result,
                Err(_) => break 'worker,
            };
            match result {
                DurableOutcome::Busy(operation) => {
                    pending = Some(Request {
                        operation: *operation,
                        ..request
                    });
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                DurableOutcome::Complete(result) => {
                    let committed = privacy.persist_pending(&mut owner).await;
                    completed = Some((request.reply, committed.and(result)));
                }
            }
        } else {
            let _ = privacy.persist_pending(&mut owner).await;
        }
        let active: std::collections::HashSet<_> =
            owner.casts.values().map(|c| c.state.cast_id).collect();
        for cast in announced.difference(&active) {
            if privacy
                .external(&mut owner, access.stop(*cast))
                .await
                .is_err()
            {
                break 'worker;
            }
        }
        announced = active;
        if force_announce || now.saturating_sub(last_announce) >= 2000 {
            last_announce = now;
            let policies: Vec<_> = owner
                .casts
                .values()
                .map(|c| {
                    (
                        c.facts.authority.current_game_session_id(),
                        accounts
                            .get(&c.facts.authority.current_game_session_id())
                            .copied(),
                        c.state,
                        c.settings,
                    )
                })
                .collect();
            for (target, account, state, settings) in policies {
                let ok = match account {
                    Some(account) => match privacy
                        .external(&mut owner, access.announce(account, state, settings))
                        .await
                    {
                        Ok(result) => result.is_ok(),
                        Err(_) => break 'worker,
                    },
                    None => false,
                };
                if !ok {
                    owner.target_gone(target);
                }
            }
        }
        accounts.retain(|target, _| owner.casts.contains_key(target));
        freshness.retain(|target, _| owner.casts.contains_key(target));
        if let Some((reply, mut result)) = completed {
            if let Ok(Reply::State(state)) = &result
                && !owner
                    .casts
                    .values()
                    .any(|c| c.state.cast_id == state.cast_id)
            {
                result = Err(Error::Unavailable);
            }
            if let Err(Ok(Reply::Observer(id))) = reply.send(result) {
                let _ = owner.leave(id);
            }
        }
        for reservation in owner.take_releases() {
            if privacy
                .external(&mut owner, access.release(reservation))
                .await
                .is_err()
            {
                break 'worker;
            }
        }
        if ending {
            break;
        }
    }
    for target in owner.casts.keys().copied().collect::<Vec<_>>() {
        owner.target_gone(target);
    }
    let _ = persist_pending(&mut owner, &root, &node).await;
    let _ = root.close_cast_consent_worker(&node, privacy.worker).await;
    for reservation in owner.take_releases() {
        let _ = access.release(reservation).await;
    }
}
pub(super) fn apply(
    owner: &mut CastOwner,
    op: Operation,
    now: u64,
    privacy_version: i64,
) -> Result<Reply, Error> {
    match op {
        Operation::Start(f, s, _) => owner.start(f, s, now).map(Reply::State),
        Operation::Update(f, r, s) => owner.update(f, r, s, now).map(Reply::State),
        Operation::Stop(f, r) => owner.stop(f, r).map(|()| Reply::Applied),
        Operation::Join(a) => owner.join(&a, now).map(Reply::Observer),
        Operation::Renew(id, a) => owner.renew(id, &a, now).map(|()| Reply::Applied),
        Operation::Leave(id) => owner.leave(id).map(|()| Reply::Applied),
        Operation::Keepalive(id) => owner.keepalive(id, now).map(|()| Reply::Applied),
        Operation::Ban(..) => Err(Error::Unavailable), // Only apply_durable may authorize it.
        Operation::Reply(f, text) => owner.reply(f, &text, now).map(|()| Reply::Applied),
        Operation::PublishView(permit, f, visible, scene, media, consents) => {
            apply(
                owner,
                Operation::Publish(permit, f, visible.clone(), scene, consents),
                now,
                privacy_version,
            )?;
            owner
                .publish_media(f, &visible, media, now)
                .map(|()| Reply::Applied)
        }
        Operation::Chat(id, text) => owner.chat(id, &text, now).map(|()| Reply::Applied),
        Operation::Mute(f, r, id, v) => owner.mute(f, r, id, v).map(|()| Reply::Applied),
        Operation::Kick(f, r, id) => owner.kick(f, r, id).map(|()| Reply::Applied),
        Operation::Consent(id, v) => owner.consent(id, v).map(|()| Reply::Applied),
        Operation::Slate(f) => owner.slate(f, now).map(|()| Reply::Applied),
        Operation::Capture(f) => {
            owner.refresh(f);
            let target = f.authority.current_game_session_id();
            let state = owner.state(target).ok_or(Error::Unavailable)?;
            Ok(Reply::Capture(CapturePermit {
                cast_id: state.cast_id,
                target,
                epoch: state.epoch,
                privacy_version,
                expires_ms: now.saturating_add(1000),
            }))
        }
        Operation::Publish(permit, f, visible, scene, consents) => {
            let target = f.authority.current_game_session_id();
            let state = owner.state(target).ok_or(Error::Unavailable)?;
            if permit.target != target
                || permit.cast_id != state.cast_id
                || permit.epoch != state.epoch
                || permit.privacy_version != privacy_version
                || permit.expires_ms <= now
            {
                return Err(Error::Unavailable);
            }
            for (character, consent) in consents {
                if let Err(error) = owner.consent(character, consent) {
                    let _ = owner.slate(f, now);
                    return Err(error);
                }
            }
            owner
                .publish(f, &visible, scene, now)
                .map(|()| Reply::Applied)
        }
        Operation::Shutdown => {
            for target in owner.casts.keys().copied().collect::<Vec<_>>() {
                owner.target_gone(target);
            }
            Ok(Reply::Applied)
        }
        Operation::Refresh(f) => {
            owner.refresh(f);
            Ok(Reply::Applied)
        }
        Operation::Invalidate(id, revision) => {
            owner.invalidate(id, revision).map(|()| Reply::Applied)
        }
        Operation::State(id) => owner.state(id).map(Reply::State).ok_or(Error::Unavailable),
        Operation::CasterRoster(target) => {
            let mut subjects: Vec<_> = owner
                .viewers
                .iter()
                .filter(|(_, viewer)| viewer.target == target)
                .map(|(id, _)| id.0)
                .collect();
            subjects.sort_unstable();
            let cast = owner.casts.get_mut(&target).ok_or(Error::Unavailable)?;
            if cast.chat.last_roster == subjects
                && cast.chat.roster_revision == cast.state.revision
                && now.saturating_sub(cast.chat.roster_at) < 2000
            {
                return Ok(Reply::Roster(None));
            }
            cast.chat.last_roster = subjects.clone();
            cast.chat.roster_revision = cast.state.revision;
            cast.chat.roster_at = now;
            Ok(Reply::Roster(Some((cast.state.revision, subjects))))
        }
        Operation::ObserverState(id) => {
            let viewer = owner.viewers.get(&id).ok_or(Error::Unavailable)?;
            owner
                .state(viewer.target)
                .map(Reply::State)
                .ok_or(Error::Unavailable)
        }
        Operation::TransportGone(id, transport) => {
            if owner
                .casts
                .get(&id)
                .is_some_and(|c| c.facts.authority.current_transport() == Some(transport))
            {
                owner.target_gone(id);
            }
            Ok(Reply::Applied)
        }
        Operation::Gone(id) => {
            owner.target_gone(id);
            Ok(Reply::Applied)
        }
        Operation::CasterPresence(id) => Ok(Reply::Chat(owner.take_caster_presence(id))),
        Operation::CasterChat(id) => Ok(Reply::Chat(owner.take_caster_chat(id))),
        Operation::Flush(id, mut write) => {
            let viewer = owner.viewers.get(&id).ok_or(Error::Unavailable)?;
            let state = owner.state(viewer.target).ok_or(Error::Unavailable)?;
            owner
                .flush(id, now, &mut |content| write(state, content))
                .map(|()| Reply::Applied)
        }
    }
}

async fn persist_pending(
    owner: &mut CastOwner,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
) -> Result<(), Error> {
    if owner.audit.pending.is_empty() {
        return Ok(());
    }
    owner.persist_audit(root, node).await
}

// IO runs exclusively on the cast task, with no world/session lock retained.
#[allow(
    clippy::too_many_arguments,
    reason = "one off-tick authority boundary carries the durable owners"
)]
pub async fn apply_durable(
    owner: &mut CastOwner,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    access: &impl preferences::PreferencesStore,
    accounts: &HashMap<GameSessionId, [u8; 16]>,
    op: Operation,
    now: u64,
    privacy_version: i64,
) -> Result<Reply, Error> {
    let started = std::time::Instant::now();
    let deadline = started + std::time::Duration::from_secs(1);
    let mut op = op;
    loop {
        // A holder retry cannot preserve an observer grant's old clock sample.
        let attempt_now = if matches!(&op, Operation::Join(_) | Operation::Renew(..)) {
            elapsed_now(now, started)?
        } else {
            now
        };
        match apply_durable_attempt(
            owner,
            root,
            node,
            access,
            accounts,
            op,
            attempt_now,
            privacy_version,
        )
        .await
        {
            DurableOutcome::Complete(result) => return result,
            DurableOutcome::Busy(returned) if std::time::Instant::now() < deadline => {
                op = *returned;
                let started = tokio::time::Instant::now();
                root.wait_holder_retry(
                    started + std::time::Duration::from_millis(2),
                    (started + std::time::Duration::from_millis(50))
                        .min(tokio::time::Instant::from_std(deadline)),
                )
                .await;
            }
            DurableOutcome::Busy(_) => return Err(Error::Unavailable),
        }
    }
}

enum DurableOutcome {
    Complete(Result<Reply, Error>),
    Busy(Box<Operation>),
}

enum DurablePreflight {
    Proceed,
    Applied,
    Busy,
}

#[allow(
    clippy::too_many_arguments,
    reason = "one off-tick authority boundary carries the durable owners"
)]
async fn apply_durable_once(
    owner: &mut CastOwner,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    access: &impl preferences::PreferencesStore,
    accounts: &HashMap<GameSessionId, [u8; 16]>,
    op: Operation,
    now: u64,
    privacy_version: i64,
) -> DurableOutcome {
    apply_durable_attempt(
        owner,
        root,
        node,
        access,
        accounts,
        op,
        now,
        privacy_version,
    )
    .await
}

#[allow(
    clippy::too_many_arguments,
    reason = "one off-tick authority boundary carries the durable owners"
)]
async fn apply_durable_attempt(
    owner: &mut CastOwner,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    access: &impl preferences::PreferencesStore,
    accounts: &HashMap<GameSessionId, [u8; 16]>,
    mut op: Operation,
    now: u64,
    privacy_version: i64,
) -> DurableOutcome {
    let started = std::time::Instant::now();
    let result: Result<DurablePreflight, Error> = async {
        match &mut op {
            Operation::Start(_, settings, account) => {
                *settings = preferences::authorize(access, *account, *settings)
                    .await?
                    .settings;
            }
            Operation::Update(facts, _, settings) => {
                let account = accounts
                    .get(&facts.authority.current_game_session_id())
                    .ok_or(Error::Unavailable)?;
                *settings = preferences::authorize(access, *account, *settings)
                    .await?
                    .settings;
            }
            Operation::Join(a) | Operation::Renew(_, a) => {
                let target = owner
                    .casts
                    .iter()
                    .find(|(_, c)| c.state.cast_id == a.cast_id)
                    .map(|(t, _)| t)
                    .ok_or(Error::Unavailable)?;
                let account = accounts.get(target).ok_or(Error::Unavailable)?;
                match root.cast_account_banned(node, *account, a.account).await {
                    Ok(true) => return Err(Error::Unavailable),
                    Ok(false) => {}
                    Err(DurabilityError::RootUnavailable) => return Ok(DurablePreflight::Busy),
                    Err(_) => return Err(Error::Unavailable),
                }
            }
            Operation::Ban(facts, revision, id) => {
                let target = facts.authority.current_game_session_id();
                let subject = owner.ban_subject(*facts, *revision, *id)?;
                let account = *accounts.get(&target).ok_or(Error::Unavailable)?;
                let state = owner.state(target).ok_or(Error::Unavailable)?;
                let record = oteryn_gameplay_durability::durability::cast_audit::CastAuditRecord {
                    ordinal: 1,
                    kind: 12,
                    target_session: target.as_bytes().to_vec(),
                    cast_id: state.cast_id.to_vec(),
                    revision: state.revision,
                    epoch: state.epoch,
                    subject_account: Some(subject.to_vec()),
                    ..Default::default()
                };
                match root.ban_cast_account(node, account, subject, record).await {
                    Ok(()) => {}
                    Err(DurabilityError::RootUnavailable) => return Ok(DurablePreflight::Busy),
                    Err(_) => return Err(Error::Unavailable),
                }
                for held in accounts
                    .iter()
                    .filter_map(|(held, held_account)| (*held_account == account).then_some(*held))
                {
                    owner.remove_banned(held, subject)?;
                }
                return Ok(DurablePreflight::Applied);
            }
            _ => {}
        }
        Ok(DurablePreflight::Proceed)
    }
    .await;
    match result {
        Ok(DurablePreflight::Proceed) => {
            // Even a successful ban lookup may cross the grant or current
            // observer lease expiry. Validate after I/O using the same clock.
            let now = if matches!(&op, Operation::Join(_) | Operation::Renew(..)) {
                elapsed_now(now, started)
            } else {
                Ok(now)
            };
            DurableOutcome::Complete(now.and_then(|now| apply(owner, op, now, privacy_version)))
        }
        Ok(DurablePreflight::Applied) => DurableOutcome::Complete(Ok(Reply::Applied)),
        Ok(DurablePreflight::Busy) => DurableOutcome::Busy(Box::new(op)),
        Err(error) => DurableOutcome::Complete(Err(error)),
    }
}

fn elapsed_now(now: u64, started: std::time::Instant) -> Result<u64, Error> {
    let elapsed = u64::try_from(started.elapsed().as_millis()).map_err(|_| Error::Unavailable)?;
    now.checked_add(elapsed).ok_or(Error::Unavailable)
}
