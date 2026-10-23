use super::{Reply, SpectateRuntime, runtime::TargetFacts};
use aws_lc_rs::constant_time::verify_slices_are_equal;
use oteryn_gameplay_foundation::foundation::{GameSessionId, GameSessionState};
use oteryn_protocol_oteryn::spectate::{Audience, CODE_BYTES, EndReason};

pub(super) struct Grant {
    pub(super) facts: TargetFacts,
    pub(super) revision: u64,
    code: [u8; CODE_BYTES],
}
impl Grant {
    pub(super) fn matches_code(&self, code: &[u8]) -> bool {
        verify_slices_are_equal(&self.code, code).is_ok()
    }
}

impl SpectateRuntime {
    pub(super) fn refresh(&mut self, current: TargetFacts) {
        let next = current.authority;
        let Some((&target, grant)) = self.grants.iter().find(|(_, grant)| {
            grant.facts.authority.commit().character_id() == next.commit().character_id()
        }) else {
            return;
        };
        let old = grant.facts;
        let previous = old.authority;
        let reason = if next.current_game_session_id() != previous.current_game_session_id()
            || !current.alive
            || next.session_state() != GameSessionState::Active
            || next.current_transport().is_none()
        {
            Some(EndReason::TargetGone)
        } else if next.current_runtime_scope() != previous.current_runtime_scope()
            || next.current_scope_generation() != previous.current_scope_generation()
            || current.house != old.house
        {
            Some(EndReason::TargetMoved)
        } else if !current.observable {
            Some(EndReason::NotObservable)
        } else if next.current_connection_generation() != previous.current_connection_generation()
            || next.current_character_lease() != previous.current_character_lease()
            || next.current_transport() != previous.current_transport()
        {
            Some(EndReason::TargetGone)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.end_grant(target, reason);
        }
    }

    pub(super) fn set_grant(
        &mut self,
        facts: TargetFacts,
        expected: u64,
        audience: Audience,
    ) -> Reply {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let Some(limits) = self.limits else {
            return Reply::NotAvailable;
        };
        if audience != Audience::Code
            || !facts.alive
            || !facts.observable
            || facts.authority.session_state() != GameSessionState::Active
            || facts.authority.current_transport().is_none()
        {
            return Reply::NotAvailable;
        }
        let current = self.grants.get(&target).map_or(0, |grant| grant.revision);
        if current != expected {
            return Reply::StaleRevision;
        }
        if current == 0 && self.grants.len() >= limits.node_sessions.get() {
            return Reply::NotAvailable;
        }
        let Some(revision) = self.revision.checked_add(1) else {
            return Reply::NotAvailable;
        };
        let mut code = [0; CODE_BYTES];
        if rustls::crypto::aws_lc_rs::default_provider()
            .secure_random
            .fill(&mut code)
            .is_err()
            || !self.audit.reserve_start(limits.audit_facts.get())
        {
            return Reply::NotAvailable;
        }
        // Rotation retires the old grant and every old stream before publishing the new code.
        self.end_grant(target, EndReason::Revoked);
        self.audit.start(target, None);
        self.revision = revision;
        self.grants.insert(
            target,
            Grant {
                facts,
                revision,
                code,
            },
        );
        Reply::Grant { revision, code }
    }

    pub(super) fn revoke(&mut self, facts: TargetFacts, expected: u64) -> Reply {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        if self.grants.get(&target).map_or(0, |grant| grant.revision) != expected {
            return Reply::StaleRevision;
        }
        self.end_grant(target, EndReason::Revoked);
        self.target_state(target)
    }

    pub(super) fn end_grant(&mut self, target: GameSessionId, reason: EndReason) {
        if self.grants.remove(&target).is_none() {
            return;
        }
        let observers: Vec<_> = self
            .observers
            .values()
            .filter(|observer| observer.target == target && observer.ended.is_none())
            .map(|observer| observer.identity)
            .collect();
        for observer in observers {
            self.end_observer(observer, reason);
        }
        self.audit.end(target, None, reason);
    }

    pub(super) fn target_state(&self, target: GameSessionId) -> Reply {
        let count = self
            .observers
            .values()
            .filter(|o| o.target == target && o.ended.is_none())
            .count();
        Reply::State {
            active: self.grants.contains_key(&target),
            count: count as u8,
        }
    }
}
