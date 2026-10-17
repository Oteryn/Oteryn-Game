//! Node-local membership DATA. PostgreSQL owns every decision and exact-channel disclosure.
//! A caller installs only committed reads; no hint or cached presence becomes output.
use std::collections::BTreeMap;
use std::time::Duration;

use crate::foundation::{PartyMemberSnapshot, PartySnapshot};

const REVISION_INTERVAL: Duration = Duration::from_secs(5);
const STALE_AFTER: Duration = Duration::from_secs(10);
type Id = [u8; 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefreshTicket {
    generation: u64,
    admission: Option<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewError {
    Unavailable,
    InvalidSnapshot,
    Superseded,
    RevisionRegressed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartyView {
    revision: u64,
    leader: Id,
    shared_xp_enabled: bool,
    members: Vec<PartyMemberSnapshot>,
}

impl PartyView {
    fn from_snapshot(snapshot: &PartySnapshot) -> Result<Self, ViewError> {
        if !snapshot.is_well_formed() {
            return Err(ViewError::InvalidSnapshot);
        }
        Ok(Self {
            revision: snapshot.revision.ok_or(ViewError::InvalidSnapshot)?,
            leader: snapshot.leader.ok_or(ViewError::InvalidSnapshot)?,
            shared_xp_enabled: snapshot.shared_xp_enabled,
            members: snapshot.members.clone(),
        })
    }

    fn supersedes(&self, old: &Self) -> bool {
        if self.revision != old.revision {
            return self.revision > old.revision;
        }
        self.leader == old.leader
            && self.shared_xp_enabled == old.shared_xp_enabled
            && self.members.len() == old.members.len()
            && self.members.iter().zip(&old.members).all(|(new, old)| {
                new.character == old.character
                    && new.invitation_order == old.invitation_order
                    && (new.presence_revision > old.presence_revision
                        || (new.presence_revision == old.presence_revision && new == old))
            })
    }
}

/// Membership-only projection: deliberately has no channel, position, health or mana.
/// It must never authorize harm, durable rewards, consent or a membership mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyMembership {
    pub(crate) party: Id,
    pub(crate) revision: u64,
    pub(crate) leader: Id,
    pub(crate) shared_xp_enabled: bool,
    pub(crate) members: Vec<Id>,
}

struct CollectedViews {
    locals: BTreeMap<Id, Option<Id>>,
    views: BTreeMap<Id, PartyView>,
}

#[derive(Debug)]
pub(crate) struct PartyViews {
    world: Id,
    generation: u64,
    full_ready: bool,
    confirmed_at: Option<Duration>,
    // None is a committed solo observation; a missing character is not admitted/ready.
    locals: BTreeMap<Id, Option<Id>>,
    views: BTreeMap<Id, PartyView>,
}

impl PartyViews {
    pub(crate) fn new(world: Id) -> Self {
        Self {
            world,
            generation: 0,
            full_ready: false,
            confirmed_at: None,
            locals: BTreeMap::new(),
            views: BTreeMap::new(),
        }
    }

    /// Start, relay reconnect, revision gap or a failed pass disables party readiness.
    /// An older in-flight read cannot restore readiness after a later invalidation.
    pub(crate) fn begin_refresh(&mut self) -> Result<RefreshTicket, ViewError> {
        self.full_ready = false;
        self.confirmed_at = None;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ViewError::Unavailable)?;
        Ok(RefreshTicket {
            generation: self.generation,
            admission: None,
        })
    }

    /// A hint carries no view or presence DATA. Any hint for a cached party requires a full
    /// read: presence writes advance only `presence_revision`, so the party revision in the
    /// hint can equal the cached one. Hints for unknown parties are ignored.
    pub(crate) fn observe_hint(&mut self, party: Id, _revision: u64) -> Result<bool, ViewError> {
        let changed = self.views.contains_key(&party);
        if changed {
            self.begin_refresh()?;
        }
        Ok(changed)
    }

    pub(crate) fn revision_check_due(&self, now: Duration) -> bool {
        self.confirmed_at
            .and_then(|confirmed| now.checked_sub(confirmed))
            .is_none_or(|elapsed| elapsed >= REVISION_INTERVAL)
    }

    pub(crate) fn ready(&self, now: Duration) -> bool {
        self.full_ready
            && self
                .confirmed_at
                .and_then(|confirmed| now.checked_sub(confirmed))
                .is_some_and(|elapsed| elapsed < STALE_AFTER)
    }

    /// Called after the complete node census was read and its transaction committed.
    /// The periodic revision/presence pass uses the same single-batch replacement path.
    pub(crate) fn finish_refresh(
        &mut self,
        ticket: RefreshTicket,
        snapshots: Vec<PartySnapshot>,
        now: Duration,
    ) -> Result<(), ViewError> {
        if ticket.generation != self.generation || ticket.admission.is_some() {
            return Err(ViewError::Superseded);
        }
        let CollectedViews { locals, views } = self.collect(snapshots)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ViewError::Unavailable)?;
        self.locals = locals;
        self.views = views;
        self.confirmed_at = Some(now);
        self.full_ready = true;
        Ok(())
    }

    fn collect(&self, snapshots: Vec<PartySnapshot>) -> Result<CollectedViews, ViewError> {
        let mut locals = BTreeMap::new();
        let mut views = BTreeMap::new();
        for snapshot in snapshots {
            if snapshot.world != self.world
                || !snapshot.is_well_formed()
                || locals
                    .insert(snapshot.character, snapshot.party_id)
                    .is_some()
            {
                return Err(ViewError::InvalidSnapshot);
            }
            if let Some(party) = snapshot.party_id {
                let view = PartyView::from_snapshot(&snapshot)?;
                if self
                    .views
                    .get(&party)
                    .is_some_and(|old| !view.supersedes(old))
                {
                    return Err(ViewError::RevisionRegressed);
                }
                if views
                    .insert(party, view.clone())
                    .is_some_and(|old| old != view)
                {
                    return Err(ViewError::InvalidSnapshot);
                }
            }
        }
        Self::validate_memberships(&locals, &views)?;
        Ok(CollectedViews { locals, views })
    }

    fn validate_memberships(
        locals: &BTreeMap<Id, Option<Id>>,
        views: &BTreeMap<Id, PartyView>,
    ) -> Result<(), ViewError> {
        let mut memberships = BTreeMap::new();
        for (party, view) in views {
            for member in &view.members {
                if memberships.insert(member.character, *party).is_some() {
                    return Err(ViewError::InvalidSnapshot);
                }
            }
        }
        if locals
            .iter()
            .any(|(character, party)| memberships.get(character).copied() != *party)
        {
            return Err(ViewError::InvalidSnapshot);
        }
        Ok(())
    }

    /// Every admission/readmission/reconnect/entry withdraws the old local observation first.
    /// Its source transaction must write current presence and COMMIT before finish_admission.
    pub(crate) fn begin_admission(&mut self, character: Id) -> Result<RefreshTicket, ViewError> {
        self.remove_local(character)?;
        Ok(RefreshTicket {
            generation: self.generation,
            admission: Some(character),
        })
    }

    pub(crate) fn finish_admission(
        &mut self,
        ticket: RefreshTicket,
        snapshot: PartySnapshot,
    ) -> Result<(), ViewError> {
        if ticket.generation != self.generation || ticket.admission != Some(snapshot.character) {
            return Err(ViewError::Superseded);
        }
        let character = snapshot.character;
        let proposed = (|| {
            let CollectedViews { locals, views } = self.collect(vec![snapshot])?;
            let mut combined_locals = self.locals.clone();
            let mut combined_views = self.views.clone();
            combined_locals.extend(locals);
            combined_views.extend(views);
            Self::validate_memberships(&combined_locals, &combined_views)?;
            Ok(CollectedViews {
                locals: combined_locals,
                views: combined_views,
            })
        })();
        let CollectedViews { locals, views } = match proposed {
            Ok(proposed) => proposed,
            Err(error) => {
                self.full_ready = false;
                self.confirmed_at = None;
                return Err(error);
            }
        };
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ViewError::Unavailable)?;
        self.locals = locals;
        self.views = views;
        // Admission confirms only its own party; it cannot refresh other parties' timers
        // or satisfy the mandatory full node refresh.
        debug_assert!(self.locals.contains_key(&character));
        Ok(())
    }

    pub(crate) fn remove_local(&mut self, character: Id) -> Result<(), ViewError> {
        self.generation = match self.generation.checked_add(1) {
            Some(generation) => generation,
            None => {
                self.full_ready = false;
                self.confirmed_at = None;
                return Err(ViewError::Unavailable);
            }
        };
        if let Some(Some(party)) = self.locals.remove(&character)
            && !self.locals.values().any(|local| *local == Some(party))
        {
            self.views.remove(&party);
        }
        Ok(())
    }

    pub(crate) fn membership(
        &self,
        character: Id,
        now: Duration,
    ) -> Result<Option<PartyMembership>, ViewError> {
        if !self.ready(now) {
            return Err(ViewError::Unavailable);
        }
        let party = self.locals.get(&character).ok_or(ViewError::Unavailable)?;
        let Some(party) = party else { return Ok(None) };
        let view = self.views.get(party).ok_or(ViewError::Unavailable)?;
        Ok(Some(PartyMembership {
            party: *party,
            revision: view.revision,
            leader: view.leader,
            shared_xp_enabled: view.shared_xp_enabled,
            members: view.members.iter().map(|member| member.character).collect(),
        }))
    }
}
