//! S17 off-tick owner. No player packets, account passwords or gameplay commands
//! enter this owner. Production admission stays closed until the Gateway profile
//! and global reservation/release adapter are pinned and composed.
use crate::spectate::TargetFacts;
use oteryn_gameplay_foundation::foundation::{GameSessionId, GameSessionState};
use std::collections::HashMap;
pub mod access;
pub mod preferences;
// 300 s at one committed Scene/s plus the 20 lines/s room ceiling and
// bounded metadata churn. Shared Arc payloads avoid copying media per viewer.
// 300 one-second full maps (63 chunks + scene), 20 chat lines/s and
// bounded public event batches; payload Arcs are shared across the room.
const VIEWER_QUEUE_ITEMS: usize = 32768;
const EGRESS_BATCH: usize = 32;
mod admission;
mod audit;
#[cfg(feature = "test-support")]
pub use audit::test_support;
mod chat;
pub use chat::{ChatLine, Content};
mod projection;
pub mod service;
use oteryn_gameplay_domain::domain::CharacterId;
use oteryn_gameplay_durability::durability::cast_consent::CastConsent;
pub use projection::Scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Unavailable,
    Conflict,
    Invalid,
    Full,
    Muted,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cast {self:?}")
    }
}
impl std::error::Error for Error {}

/// Passwords are checked in Platform, never supplied to Game. Rotating the
/// access revision revokes every observer, even if the boolean stays equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub delay_seconds: u16,
    pub viewer_cap: u8,
    pub chat_enabled: bool,
    pub password_required: bool,
    pub access_revision: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            delay_seconds: 0,
            viewer_cap: 50,
            chat_enabled: true,
            password_required: false,
            access_revision: 1,
        }
    }
}
impl Settings {
    fn validate(self) -> Result<(), Error> {
        if self.delay_seconds > 300
            || !(1..=50).contains(&self.viewer_cap)
            || self.access_revision == 0
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub chat_enabled: bool,
    pub cast_id: [u8; 16],
    pub revision: u64,
    pub epoch: u64,
    pub delay_seconds: u16,
    pub viewers: u8,
}
struct Cast {
    facts: TargetFacts,
    settings: Settings,
    state: State,
    chat: chat::Room,
    audit_ordinal: u64,
}

/// Only the authenticated online Platform consume adapter constructs production
/// admission proofs; callers cannot supply a `logged_in` boolean.
pub struct VerifiedAdmission {
    cast_id: [u8; 16],
    epoch: u64,
    account: [u8; 16],
    reservation: [u8; 16],
    access_revision: u64,
    expires_ms: u64,
    chat: bool,
}

impl VerifiedAdmission {
    pub fn chat_selected(&self) -> bool {
        self.chat
    }
    pub fn reservation_id(&self) -> [u8; 16] {
        self.reservation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverId([u8; 16]);
impl ObserverId {
    pub fn from_handle(handle: [u8; 16]) -> Self {
        Self(handle)
    }
}
struct Viewer {
    target: GameSessionId,
    account: [u8; 16],
    reservation: [u8; 16],
    expires_ms: u64,
    reservation_expires_ms: u64,
    chat_selected: bool,
    queue: std::collections::VecDeque<projection::Queued>,
    shown: std::sync::Arc<[(CharacterId, i64)]>,
    audit_ordinal: u64,
}

/// Constructed only with an explicit measured node capacity. It is not mounted
/// by default and cannot authenticate a network connection on its own.
pub struct CastOwner {
    casts: HashMap<GameSessionId, Cast>,
    viewers: HashMap<ObserverId, Viewer>,
    used_tickets: HashMap<[u8; 16], u64>,
    releases: Vec<[u8; 16]>,
    consents: HashMap<CharacterId, (CastConsent, u64)>,
    consent_floor: HashMap<CharacterId, i64>,
    audit: audit::Journal,
    node_capacity: usize,
    revision: u64,
    clock_ms: u64,
}
impl CastOwner {
    pub fn new(node_capacity: usize) -> Result<Self, Error> {
        if node_capacity == 0 || node_capacity > 4096 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            casts: HashMap::new(),
            viewers: HashMap::new(),
            used_tickets: HashMap::new(),
            releases: Vec::new(),
            consents: HashMap::new(),
            consent_floor: HashMap::new(),
            audit: audit::Journal::new(node_capacity * 8 + 64)?,
            node_capacity,
            revision: 0,
            clock_ms: 0,
        })
    }
    fn tick(&mut self, now_ms: u64) -> Result<(), Error> {
        if now_ms < self.clock_ms || now_ms > u64::MAX - 300_000 {
            return Err(Error::Invalid);
        }
        self.clock_ms = now_ms;
        self.used_tickets.retain(|_, expiry| *expiry > now_ms);
        let expired: Vec<_> = self
            .viewers
            .iter()
            .filter(|(_, v)| v.expires_ms <= now_ms)
            .map(|(id, _)| *id)
            .collect();
        for id in expired {
            self.leave(id)?;
        }
        Ok(())
    }
    fn next_revision(&mut self) -> Result<u64, Error> {
        self.revision = self.revision.checked_add(1).ok_or(Error::Unavailable)?;
        Ok(self.revision)
    }
    pub fn state(&self, target: GameSessionId) -> Option<State> {
        self.casts.get(&target).map(|cast| cast.state)
    }
    pub fn start(
        &mut self,
        facts: TargetFacts,
        settings: Settings,
        now_ms: u64,
    ) -> Result<State, Error> {
        self.tick(now_ms)?;
        settings.validate()?;
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        if !live(facts) {
            return Err(Error::Unavailable);
        }
        if self.casts.contains_key(&target) {
            return Err(Error::Conflict);
        }
        if self.casts.len() >= self.node_capacity {
            return Err(Error::Full);
        }
        let revision = self.next_revision()?;
        let mut cast_id = [0; 16];
        aws_lc_rs::rand::fill(&mut cast_id).map_err(|_| Error::Unavailable)?;
        let state = State {
            chat_enabled: settings.chat_enabled,
            cast_id,
            revision,
            epoch: revision,
            delay_seconds: settings.delay_seconds,
            viewers: 0,
        };
        self.audit.reserve()?;
        self.audit.append(1, target, state, None);
        self.casts.insert(
            target,
            Cast {
                facts,
                settings,
                state,
                chat: chat::Room::default(),
                audit_ordinal: self.audit.ordinal,
            },
        );
        Ok(state)
    }
    pub fn stop(&mut self, facts: TargetFacts, expected: u64) -> Result<(), Error> {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        if cast.state.revision != expected {
            return Err(Error::Conflict);
        }
        self.end(target);
        Ok(())
    }
    pub fn update(
        &mut self,
        facts: TargetFacts,
        expected: u64,
        settings: Settings,
        now_ms: u64,
    ) -> Result<State, Error> {
        self.tick(now_ms)?;
        settings.validate()?;
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        let cast = self.casts.get(&target).ok_or(Error::Unavailable)?;
        if cast.state.revision != expected
            || settings.access_revision < cast.settings.access_revision
            || (settings.password_required != cast.settings.password_required
                && settings.access_revision == cast.settings.access_revision)
        {
            return Err(Error::Conflict);
        }
        if settings.viewer_cap < cast.state.viewers {
            return Err(Error::Full);
        }
        self.audit.room(1)?;
        let access_changed = settings.access_revision != cast.settings.access_revision;
        if settings.access_revision != cast.settings.access_revision {
            let ids: Vec<_> = self
                .viewers
                .iter()
                .filter(|(_, v)| v.target == target)
                .map(|(id, _)| *id)
                .collect();
            for id in ids {
                self.leave(id)?;
            }
        }
        let revision = self.next_revision()?;
        let cast = self.casts.get_mut(&target).ok_or(Error::Unavailable)?;
        cast.settings = settings;
        cast.state.chat_enabled = settings.chat_enabled;
        cast.state.revision = revision;
        cast.state.epoch = revision;
        cast.state.delay_seconds = settings.delay_seconds;
        for viewer in self.viewers.values_mut().filter(|v| v.target == target) {
            viewer.queue.clear();
            viewer.shown = std::sync::Arc::from([]);
            viewer
                .queue
                .push_back(projection::Queued::plain(now_ms, Content::Clear));
        }
        let state = cast.state;
        self.audit
            .append(if access_changed { 9 } else { 10 }, target, state, None);
        self.casts
            .get_mut(&target)
            .ok_or(Error::Unavailable)?
            .audit_ordinal = self.audit.ordinal;
        self.queue_metadata(target);
        Ok(state)
    }
    pub fn refresh(&mut self, facts: TargetFacts) {
        let current = facts.authority.current_game_session_id();
        if self
            .casts
            .get(&current)
            .is_some_and(|cast| !same_authority(cast.facts, facts))
        {
            self.end(current);
            return;
        }
        let old = self.casts.iter().find(|(_, c)| {
            c.facts.authority.commit().character_id() == facts.authority.commit().character_id()
        });
        if let Some((&target, cast)) = old
            && !same_authority(cast.facts, facts)
        {
            self.end(target);
        }
    }
    pub fn target_gone(&mut self, target: GameSessionId) {
        self.end(target);
    }
    /// Periodic off-tick worker timer; needed even when no viewer sends input.
    pub fn poll(&mut self, now_ms: u64) -> Result<(), Error> {
        self.tick(now_ms)
    }
    fn end(&mut self, target: GameSessionId) {
        let ids: Vec<_> = self
            .viewers
            .iter()
            .filter(|(_, v)| v.target == target)
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            let _ = self.leave(id);
        }
        if let Some(cast) = self.casts.remove(&target) {
            self.audit.terminal(2, target, cast.state, None);
        }
    }
}
fn live(facts: TargetFacts) -> bool {
    facts.alive
        && facts.authority.session_state() == GameSessionState::Active
        && facts.authority.current_transport().is_some()
}
fn same_authority(old: TargetFacts, new: TargetFacts) -> bool {
    let a = old.authority;
    let b = new.authority;
    live(new)
        && a.commit().character_id() == b.commit().character_id()
        && a.current_game_session_id() == b.current_game_session_id()
        && a.current_connection_generation() == b.current_connection_generation()
        && a.current_character_lease() == b.current_character_lease()
        && a.current_runtime_scope() == b.current_runtime_scope()
        && a.current_scope_generation() == b.current_scope_generation()
        && a.current_transport() == b.current_transport()
        && old.house == new.house
}

#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod tests;
