//! Session-scoped Team Finder handles; durable identities never cross the wire.
use crate::durability::team_finder::Target;
use crate::foundation::GameSessionId;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HandleKind {
    Listing,
    Applicant,
}

/// A replacement page expires the previous page's handles. Keeping one page per
/// kind bounds memory without truncating a World with many pages of listings.
#[derive(Default)]
pub struct SessionTeamFinderView {
    handles: BTreeMap<u64, (HandleKind, Target)>,
}

impl SessionTeamFinderView {
    fn key(
        session: GameSessionId,
        generation: u64,
        kind: HandleKind,
        target: Target,
    ) -> Option<u64> {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.team-finder.handle.v1");
        hash.update(session.as_bytes());
        hash.update(generation.to_be_bytes());
        hash.update([match kind {
            HandleKind::Listing => 1,
            HandleKind::Applicant => 2,
        }]);
        hash.update(target.character);
        hash.update(target.session.as_bytes());
        hash.update(target.connection_generation.to_be_bytes());
        let bytes: [u8; 32] = hash.finalize().into();
        let handle = u64::from_be_bytes(bytes[..8].try_into().ok()?);
        (handle != 0).then_some(handle)
    }

    pub(super) fn replace_page(&mut self, kind: HandleKind) {
        self.handles.retain(|_, (existing, _)| *existing != kind);
    }

    pub(super) fn handle(
        &mut self,
        session: GameSessionId,
        generation: u64,
        kind: HandleKind,
        target: Target,
    ) -> Option<u64> {
        let handle = Self::key(session, generation, kind, target)?;
        if self
            .handles
            .get(&handle)
            .is_some_and(|old| *old != (kind, target))
        {
            return None;
        }
        // The producer is one bounded query page, including its continuation.
        if !self.handles.contains_key(&handle)
            && self.handles.len()
                >= 2 * (oteryn_protocol_oteryn::team_finder_members::MAX_PAGE_ENTRIES + 1)
        {
            return None;
        }
        self.handles.insert(handle, (kind, target));
        Some(handle)
    }

    pub(super) fn resolve(
        &self,
        session: GameSessionId,
        generation: u64,
        kind: HandleKind,
        handle: u64,
    ) -> Option<Target> {
        let (stored_kind, target) = *self.handles.get(&handle)?;
        if stored_kind != kind {
            return None;
        }
        (Self::key(session, generation, kind, target) == Some(handle)).then_some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(last: u8) -> Result<GameSessionId, Box<dyn std::error::Error>> {
        let mut identity = [0; 16];
        identity[6] = 0x70;
        identity[8] = 0x80;
        identity[15] = last;
        Ok(GameSessionId::decode(&identity)?)
    }

    #[test]
    fn team_finder_handles_fence_both_sessions_generations_kind_and_page()
    -> Result<(), Box<dyn std::error::Error>> {
        let session = session(1)?;
        let target = Target {
            character: [2; 16],
            session,
            connection_generation: 1,
        };
        let mut view = SessionTeamFinderView::default();
        let listing = view
            .handle(session, 1, HandleKind::Listing, target)
            .ok_or("listing")?;
        assert_eq!(
            view.resolve(session, 1, HandleKind::Listing, listing),
            Some(target)
        );
        assert_eq!(
            view.resolve(session, 1, HandleKind::Applicant, listing),
            None
        );
        assert_eq!(view.resolve(session, 2, HandleKind::Listing, listing), None);
        assert_eq!(
            SessionTeamFinderView::default().resolve(session, 1, HandleKind::Listing, listing),
            None
        );
        let applicant = view
            .handle(session, 1, HandleKind::Applicant, target)
            .ok_or("applicant")?;
        let next_generation = view
            .handle(session, 2, HandleKind::Listing, target)
            .ok_or("generation")?;
        let target_generation = view
            .handle(
                session,
                1,
                HandleKind::Listing,
                Target {
                    connection_generation: 2,
                    ..target
                },
            )
            .ok_or("target generation")?;
        assert_ne!(listing, applicant);
        assert_ne!(listing, next_generation);
        assert_ne!(listing, target_generation);
        let other = self::session(2)?;
        assert_eq!(view.resolve(other, 1, HandleKind::Listing, listing), None);
        let other_target = view
            .handle(
                session,
                1,
                HandleKind::Listing,
                Target {
                    session: other,
                    ..target
                },
            )
            .ok_or("target session")?;
        assert_ne!(listing, other_target);
        view.replace_page(HandleKind::Listing);
        assert_eq!(view.resolve(session, 1, HandleKind::Listing, listing), None);
        assert_eq!(
            view.resolve(session, 1, HandleKind::Applicant, applicant),
            Some(target)
        );
        Ok(())
    }

    #[test]
    fn team_finder_page_replacement_keeps_browse_memory_bounded()
    -> Result<(), Box<dyn std::error::Error>> {
        let session = session(1)?;
        for kind in [HandleKind::Listing, HandleKind::Applicant] {
            let mut view = SessionTeamFinderView::default();
            for page in 0..50_u128 {
                view.replace_page(kind);
                for offset in 0..oteryn_protocol_oteryn::team_finder_members::MAX_PAGE_ENTRIES {
                    let target = Target {
                        character: (page * 1000 + offset as u128).to_be_bytes(),
                        session,
                        connection_generation: 1,
                    };
                    assert!(view.handle(session, 1, kind, target).is_some());
                }
                assert_eq!(
                    view.handles.len(),
                    oteryn_protocol_oteryn::team_finder_members::MAX_PAGE_ENTRIES
                );
            }
        }
        Ok(())
    }
}
