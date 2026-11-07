use super::{Reply, SpectateRuntime, TargetFacts};
use oteryn_gameplay_foundation::foundation::GameSessionId;
use oteryn_protocol_oteryn::spectate::{EndReason, SpectateMessage};
use std::collections::VecDeque;
use std::time::Instant;

/// O2 supplies these only after authentication; this value is not a credential.
/// Intentionally has no CharacterId, lease, actor, fence, position or command sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverIdentity {
    pub session: GameSessionId,
    pub generation: std::num::NonZeroU64,
    pub account: [u8; 16],
}
#[derive(Debug, Clone, Copy)]
pub enum ObserverCommand {
    Stop,
    Keepalive,
    Gameplay,
}

pub(super) struct Observer {
    pub(super) identity: ObserverIdentity,
    pub(super) target: GameSessionId,
    pub(super) queue: VecDeque<SpectateMessage>,
    pub(super) bytes: usize,
    pub(super) ended: Option<EndReason>,
}

impl SpectateRuntime {
    pub(super) fn admit(
        &mut self,
        facts: TargetFacts,
        identity: ObserverIdentity,
        code: &[u8],
        revision: u64,
        now: Instant,
    ) -> Reply {
        self.refresh(facts);
        let Some(limits) = self.limits else {
            return Reply::NotAvailable;
        };
        self.attempts.retain(|_, window| !window.expire(now));
        if !self.attempts.contains_key(&identity.account)
            && self.attempts.len() >= limits.rate_accounts.get()
        {
            return Reply::NotAvailable;
        }
        if !self
            .attempts
            .entry(identity.account)
            .or_default()
            .attempt(now)
        {
            return Reply::NotAvailable;
        }
        let target = facts.authority.current_game_session_id();
        let Some(grant) = self.grants.get(&target) else {
            return Reply::NotAvailable;
        };
        // Always perform the code comparison, including stale-revision attempts.
        let correct = grant.matches_code(code);
        if !correct || grant.revision != revision || identity.session == target {
            return Reply::NotAvailable;
        }
        if self.observers.contains_key(&identity.session) {
            return Reply::NotAvailable;
        }
        if self
            .observers
            .values()
            .filter(|o| o.target == target && o.ended.is_none())
            .count()
            >= 50
        {
            return Reply::Full;
        }
        // Ended streams occupy their bounded terminal slot until the egress consumes it.
        if self.observers.len() >= limits.node_sessions.get()
            || self
                .observers
                .values()
                .filter(|o| o.identity.account == identity.account)
                .count()
                >= 2
            || !self.audit.reserve_start(limits.audit_facts.get())
        {
            return Reply::NotAvailable;
        }
        self.audit.start(target, Some(identity.session));
        self.observers.insert(
            identity.session,
            Observer {
                identity,
                target,
                queue: VecDeque::new(),
                bytes: 0,
                ended: None,
            },
        );
        Reply::Admitted
    }

    pub(super) fn end_observer(&mut self, identity: ObserverIdentity, reason: EndReason) {
        let Some(observer) = self
            .observers
            .get_mut(&identity.session)
            .filter(|o| o.identity == identity && o.ended.is_none())
        else {
            return;
        };
        observer.queue.clear();
        observer.bytes = 0;
        observer.ended = Some(reason);
        self.audit
            .end(observer.target, Some(identity.session), reason);
    }

    pub(super) fn observer_input(
        &mut self,
        identity: ObserverIdentity,
        command: ObserverCommand,
    ) -> Reply {
        if !self
            .observers
            .get(&identity.session)
            .is_some_and(|o| o.identity == identity && o.ended.is_none())
        {
            return Reply::NotAvailable;
        }
        match command {
            ObserverCommand::Stop => {
                self.end_observer(identity, EndReason::ObserverStopped);
                Reply::Applied
            }
            ObserverCommand::Keepalive => Reply::Applied,
            ObserverCommand::Gameplay => Reply::NotAllowed,
        }
    }

    pub(super) fn drain(&mut self, identity: ObserverIdentity) -> Reply {
        let Some(observer) = self
            .observers
            .get_mut(&identity.session)
            .filter(|o| o.identity == identity)
        else {
            return Reply::NotAvailable;
        };
        if let Some(reason) = observer.ended {
            self.observers.remove(&identity.session);
            return Reply::Frames(vec![SpectateMessage::End(reason)]);
        }
        observer.bytes = 0;
        Reply::Frames(observer.queue.drain(..).collect())
    }
}
