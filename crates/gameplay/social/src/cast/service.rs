//! Bounded off-tick command owner. Its task owns the cast state exclusively;
//! neither world locks nor native actors are retained across storage/network IO.
use super::*;
use oteryn_gameplay_durability::durability::{
    DurabilityRoot, runtime_scope_assignment::NodeIncarnationProof,
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
    State(GameSessionId),
    Invalidate(CharacterId, i64),
    CasterChat(GameSessionId),
    Shutdown,
    /// Synchronous final writes run on the worker, serialized with revocation.
    Flush(ObserverId, CastWriter),
}
pub enum Reply {
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
            .map_err(|_| Error::Full)?;
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
    let Ok(mut privacy) = root
        .register_cast_consent_worker(&node, owner.audit.incarnation)
        .await
    else {
        return;
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
            let policy = match &operation {
                Operation::Start(_, settings, account) => {
                    preferences::authorize(access.as_ref(), *account, *settings)
                        .await
                        .map(|_| ())
                }
                Operation::Update(facts, _, settings) => match accounts
                    .get(&facts.authority.current_game_session_id())
                {
                    Some(account) => preferences::authorize(access.as_ref(), *account, *settings)
                        .await
                        .map(|_| ()),
                    None => Err(Error::Unavailable),
                },
                _ => Ok(()),
            };
            let result = policy.and_then(|()| apply(&mut owner, operation, now, privacy.version()));
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
