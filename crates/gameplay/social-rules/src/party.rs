//! Pure membership projections and consistency checks for committed party reads.

use std::collections::BTreeMap;

type Id = [u8; 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewError {
    Unavailable,
    InvalidSnapshot,
    Superseded,
    RevisionRegressed,
}

/// Membership-only projection: deliberately has no channel, position, health or mana.
/// It must never authorize harm, durable rewards, consent or a membership mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyMembership {
    pub party: Id,
    pub revision: u64,
    pub leader: Id,
    pub shared_xp_enabled: bool,
    pub members: Vec<Id>,
}

/// Check that each member belongs to one party and that every local observation agrees.
/// These supplied membership facts establish no freshness or mutation authority.
pub fn validate_memberships(
    locals: &BTreeMap<Id, Option<Id>>,
    views: impl IntoIterator<Item = (Id, impl IntoIterator<Item = Id>)>,
) -> Result<(), ViewError> {
    let mut memberships = BTreeMap::new();
    for (party, members) in views {
        for character in members {
            if memberships.insert(character, party).is_some() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_reads_require_consistent_solo_and_party_observations() {
        let party = [1; 16];
        let other = [2; 16];
        let member = [3; 16];
        let solo = [4; 16];
        let locals = BTreeMap::from([(member, Some(party)), (solo, None)]);
        assert_eq!(validate_memberships(&locals, [(party, [member])]), Ok(()));
        assert_eq!(
            validate_memberships(&locals, [(party, [member, solo])]),
            Err(ViewError::InvalidSnapshot)
        );
        assert_eq!(
            validate_memberships(&locals, [(other, [member])]),
            Err(ViewError::InvalidSnapshot)
        );
        assert_eq!(
            validate_memberships(&locals, [(party, [member]), (other, [member])]),
            Err(ViewError::InvalidSnapshot)
        );
        assert_eq!(
            validate_memberships(&locals, [] as [(Id, [Id; 0]); 0]),
            Err(ViewError::InvalidSnapshot)
        );
    }
}
