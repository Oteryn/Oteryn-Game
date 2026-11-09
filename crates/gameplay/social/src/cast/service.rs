//! Bounded off-tick command owner. Its task owns the cast state exclusively;
//! neither world locks nor native actors are retained across storage/network IO.
use super::*;
use oteryn_gameplay_durability::durability::{
    DurabilityError, DurabilityRoot, runtime_scope_assignment::NodeIncarnationProof,
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
        tokio::spawn(run(owner, root, node, access, rx, origin));
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
) {
    // Canary livestream.cpp:249 starts the live service explicitly. S17 additionally
    // requires a consent-worker lease: retry only a busy durable holder, never
    // process a queued Start before registration or after the bounded deadline.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut privacy = loop {
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
    let mut privacy_pulse = std::time::Instant::now();
    let mut accounts = std::collections::HashMap::new();
    let mut last_announce = 0u64;
    let mut announced = std::collections::HashSet::new();
    let mut freshness = std::collections::HashMap::new();
    'worker: loop {
        let mut request =
            match tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await {
                Ok(Some(request)) => Some(request),
                Ok(None) => break,
                Err(_) => None,
            };
        let now = u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !privacy.valid() {
            for target in owner.casts.keys().copied().collect::<Vec<_>>() {
                owner.target_gone(target);
            }
        }
        if privacy_pulse.elapsed() >= std::time::Duration::from_millis(250) || !privacy.valid() {
            privacy_pulse = std::time::Instant::now();
            if let Ok(batch) = root.refresh_cast_consent_worker(&node, &mut privacy).await {
                for event in batch.events() {
                    // Unknown characters have no queued frames. A capture made
                    // before this ACK cannot publish at the new privacy version.
                    if !owner.consents.contains_key(&event.character)
                        && !owner.consent_floor.contains_key(&event.character)
                    {
                        continue;
                    }
                    if owner
                        .invalidate(event.character, event.minimum_revision - 1)
                        .is_err()
                    {
                        break 'worker;
                    }
                }
                if !batch.is_empty()
                    && root
                        .acknowledge_cast_consent(&node, &mut privacy, batch)
                        .await
                        .is_err()
                {
                    break 'worker;
                }
            }
        }
        if !privacy.valid() {
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
            let _ = persist_pending(&mut owner, &root, &node).await;
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
        if request
            .as_ref()
            .is_some_and(|r| now.saturating_sub(r.submitted_ms) > 1000)
        {
            if let Some(expired) = request.take() {
                let _ = expired.reply.send(Err(Error::Unavailable));
            }
            let _ = persist_pending(&mut owner, &root, &node).await;
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
                    let until = privacy.valid_until();
                    Operation::Flush(
                        id,
                        Box::new(move |state, content| {
                            std::time::Instant::now() < until && write(state, content)
                        }),
                    )
                }
                operation => operation,
            };
            let result = apply_durable_with_lease(
                &mut owner,
                &root,
                &node,
                access.as_ref(),
                &accounts,
                operation,
                u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX),
                privacy.version(),
                Some(privacy.valid_until()),
            )
            .await;
            let committed = persist_pending(&mut owner, &root, &node).await;
            completed = Some((request.reply, committed.and(result)));
        } else {
            let _ = persist_pending(&mut owner, &root, &node).await;
        }
        let active: std::collections::HashSet<_> =
            owner.casts.values().map(|c| c.state.cast_id).collect();
        for cast in announced.difference(&active) {
            let _ = access.stop(*cast).await;
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
                    Some(account) => access.announce(account, state, settings).await.is_ok(),
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
            let _ = access.release(reservation).await;
        }
        if ending {
            break;
        }
    }
    for target in owner.casts.keys().copied().collect::<Vec<_>>() {
        owner.target_gone(target);
    }
    let _ = persist_pending(&mut owner, &root, &node).await;
    let _ = root.close_cast_consent_worker(&node, privacy).await;
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
    apply_durable_with_lease(
        owner,
        root,
        node,
        access,
        accounts,
        op,
        now,
        privacy_version,
        None,
    )
    .await
}

#[allow(
    clippy::too_many_arguments,
    reason = "the worker additionally supplies its local consent lease fence"
)]
async fn apply_durable_with_lease(
    owner: &mut CastOwner,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    access: &impl preferences::PreferencesStore,
    accounts: &HashMap<GameSessionId, [u8; 16]>,
    mut op: Operation,
    now: u64,
    privacy_version: i64,
    privacy_until: Option<std::time::Instant>,
) -> Result<Reply, Error> {
    let started = std::time::Instant::now();
    // Leave time for the next worker turn (including its 100 ms receive wait)
    // to refresh consent. A busy holder must not consume the media lease.
    let retry_until = privacy_until.map_or(started + std::time::Duration::from_secs(1), |until| {
        until
            .checked_sub(std::time::Duration::from_millis(250))
            .unwrap_or(started)
            .min(started + std::time::Duration::from_secs(1))
    });
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
            if retry_busy_holder(root, retry_until, || {
                root.cast_account_banned(node, *account, a.account)
            })
            .await?
            {
                return Err(Error::Unavailable);
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
            retry_busy_holder(root, retry_until, || {
                root.ban_cast_account(node, account, subject, record.clone())
            })
            .await?;
            for held in accounts
                .iter()
                .filter_map(|(held, held_account)| (*held_account == account).then_some(*held))
            {
                owner.remove_banned(held, subject)?;
            }
            return Ok(Reply::Applied);
        }
        _ => {}
    }
    let now = if privacy_until.is_some() && matches!(op, Operation::Join(_) | Operation::Renew(..))
    {
        if privacy_until.is_some_and(|until| std::time::Instant::now() >= until) {
            return Err(Error::Unavailable);
        }
        // Admission can expire while waiting for the holder. Re-run the owner's
        // expiry/epoch/access checks against the clock after IO, under this fence.
        now.saturating_add(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX))
    } else {
        now
    };
    apply(owner, op, now, privacy_version)
}

// Retry only acquisition refusal: a storage/receipt failure must never replay
// the operation. The worker caps this window by its remaining consent lease.
async fn retry_busy_holder<T, F: std::future::Future<Output = Result<T, DurabilityError>>>(
    root: &DurabilityRoot,
    deadline: std::time::Instant,
    mut operation: impl FnMut() -> F,
) -> Result<T, Error> {
    use tokio::time::Instant;
    let deadline = Instant::from_std(deadline);
    loop {
        if Instant::now() >= deadline {
            return Err(Error::Unavailable);
        }
        match operation().await {
            Ok(value) => return Ok(value),
            Err(DurabilityError::RootUnavailable) if Instant::now() < deadline => {
                let now = Instant::now();
                root.wait_holder_retry(
                    now + std::time::Duration::from_millis(2),
                    (now + std::time::Duration::from_millis(50)).min(deadline),
                )
                .await;
            }
            Err(_) => return Err(Error::Unavailable),
        }
    }
}
