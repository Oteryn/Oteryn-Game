use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::time::Instant;

use super::{
    AuditFact, ObserverCommand, ObserverIdentity, WorldProjection, audit::AuditJournal,
    grant::Grant, rate_window::RateWindow, session::Observer,
};
use oteryn_gameplay_foundation::foundation::{
    AuthenticatedTransportRefV1, GameSessionAuthoritySnapshot, GameSessionId,
};
use oteryn_protocol_oteryn::spectate::{Audience, CODE_BYTES, EndReason, SpectateMessage};

/// All three capacities are measured composition inputs, with no activation default.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub node_sessions: NonZeroUsize,
    pub rate_accounts: NonZeroUsize,
    pub audit_facts: NonZeroUsize,
}

/// Independent current native authority plus scope/death facts from its owning work cycle.
/// Neither these facts nor a GameSessionId are observer admission credentials.
#[derive(Debug, Clone, Copy)]
pub struct TargetFacts {
    pub authority: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    pub alive: bool,
    pub observable: bool,
    pub house: Option<[u8; 16]>,
}

pub enum Request {
    Bootstrap,
    GrantSet {
        facts: TargetFacts,
        expected_revision: u64,
        audience: Audience,
    },
    GrantRevoke {
        facts: TargetFacts,
        expected_revision: u64,
    },
    Refresh(TargetFacts),
    TargetGone(GameSessionId),
    /// Only a future authenticated O2 consumer may issue this internal request.
    Admit {
        facts: TargetFacts,
        observer: ObserverIdentity,
        code: Vec<u8>,
        grant_revision: u64,
    },
    ObserverInput {
        observer: ObserverIdentity,
        command: ObserverCommand,
    },
    Publish {
        facts: TargetFacts,
        projection: WorldProjection,
    },
    Drain(ObserverIdentity),
    WriteFailed(ObserverIdentity),
    AuditRead,
    /// Issued only after a sink durably commits the copied prefix, outside the owner lock.
    AuditCommitted {
        incarnation: [u8; 16],
        through: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    NotAvailable,
    StaleRevision,
    Full,
    Grant {
        revision: u64,
        code: [u8; CODE_BYTES],
    },
    State {
        active: bool,
        count: u8,
    },
    Admitted,
    NotAllowed,
    Frames(Vec<SpectateMessage>),
    Audit(Vec<AuditFact>),
    Applied,
}

/// Restart constructs an empty owner; limits alone do not open a transport admission path.
#[derive(Default)]
pub struct SpectateRuntime {
    pub(super) limits: Option<Limits>,
    pub(super) grants: HashMap<GameSessionId, Grant>,
    pub(super) observers: HashMap<GameSessionId, Observer>,
    pub(super) attempts: HashMap<[u8; 16], RateWindow>,
    pub(super) audit: AuditJournal,
    pub(super) revision: u64,
}

impl From<Limits> for SpectateRuntime {
    fn from(limits: Limits) -> Self {
        Self {
            limits: Some(limits),
            ..Self::default()
        }
    }
}

impl SpectateRuntime {
    /// The only mutation seam, called under the dispatch spectate-owner lock. It never performs IO.
    pub fn apply(&mut self, request: Request) -> Reply {
        let now = Instant::now();
        match request {
            Request::Bootstrap => Reply::NotAvailable,
            Request::GrantSet {
                facts,
                expected_revision,
                audience,
            } => self.set_grant(facts, expected_revision, audience),
            Request::GrantRevoke {
                facts,
                expected_revision,
            } => self.revoke(facts, expected_revision),
            Request::Refresh(facts) => {
                self.refresh(facts);
                self.target_state(facts.authority.current_game_session_id())
            }
            Request::TargetGone(target) => {
                self.end_grant(target, EndReason::TargetGone);
                Reply::Applied
            }
            Request::Admit {
                facts,
                observer,
                code,
                grant_revision,
            } => self.admit(facts, observer, &code, grant_revision, now),
            Request::ObserverInput { observer, command } => self.observer_input(observer, command),
            Request::Publish { facts, projection } => {
                self.publish(facts, projection);
                Reply::Applied
            }
            Request::Drain(observer) => self.drain(observer),
            Request::WriteFailed(observer) => {
                self.end_observer(observer, EndReason::Overload);
                Reply::Applied
            }
            Request::AuditRead => Reply::Audit(self.audit.pending.iter().cloned().collect()),
            Request::AuditCommitted {
                incarnation,
                through,
            } => {
                if self.audit.ack(incarnation, through) {
                    Reply::Applied
                } else {
                    Reply::StaleRevision
                }
            }
        }
    }
}
