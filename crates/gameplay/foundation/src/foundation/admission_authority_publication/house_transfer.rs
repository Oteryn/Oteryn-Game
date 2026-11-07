//! HOUSE-TRANSFER-1 changes claims only alongside the fenced native session transfer.
use super::*;

impl AdmissionAuthorityPublicationV1 {
    /// Validate the exact Account/Character claim move. The durable consumer must
    /// independently prove source, destination, transport, node and assignments
    /// in the same transaction. Ordinary publication still cannot change claims.
    pub fn prepare_house_transfer(
        owner: &dyn AdmissionAuthorityOwningPublisherV1,
        current: &[Option<AdmissionAuthorityPublicationChangeV1>],
        character: CharacterId,
        sessions: (GameSessionId, GameSessionId),
        now: i64,
    ) -> Result<Self, AdmissionAuthorityPublicationErrorV1> {
        use AdmissionAuthorityPublicationErrorV1::Conflict;
        let (source, destination) = sessions;
        let request = Self::prepare(owner, now)?;
        if source == destination || request.changes.len() != 2 || current.len() != 2 {
            return Err(Conflict);
        }
        let mut unchanged_claims = request.clone();
        let (mut account_seen, mut character_seen) = (None, false);
        for ((next, normalized), prior) in request
            .changes
            .iter()
            .zip(&mut unchanged_claims.changes)
            .zip(current)
        {
            let prior = prior.as_ref().ok_or(Conflict)?;
            match (&next.key, &prior.state, &next.state, &mut normalized.state) {
                (
                    AdmissionAuthorityGuardKeyV1::Account { account_id },
                    AdmissionAuthorityGuardStateV1::Account {
                        presence: before, ..
                    },
                    AdmissionAuthorityGuardStateV1::Account {
                        presence: after,
                        security,
                    },
                    AdmissionAuthorityGuardStateV1::Account { presence, .. },
                ) if *before == Some((character, source))
                    && *after == Some((character, destination))
                    && security.allowed
                    && account_seen.is_none() =>
                {
                    account_seen = Some(account_id.as_str());
                    *presence = *before;
                }
                (
                    AdmissionAuthorityGuardKeyV1::Character(id),
                    AdmissionAuthorityGuardStateV1::Character {
                        account_id: old_account,
                        world_id: old_world,
                        eligible: true,
                        lease_generation: old_lease,
                        holder: Some(old),
                    },
                    AdmissionAuthorityGuardStateV1::Character {
                        account_id: new_account,
                        world_id: new_world,
                        eligible: true,
                        lease_generation: new_lease,
                        holder: Some(new),
                    },
                    AdmissionAuthorityGuardStateV1::Character {
                        lease_generation,
                        holder,
                        ..
                    },
                ) if *id == character
                    && *old == source
                    && *new == destination
                    && old_lease.checked_add(1) == Some(*new_lease)
                    && old_account == new_account
                    && old_world == new_world
                    && !character_seen =>
                {
                    character_seen = true;
                    *lease_generation = *old_lease;
                    *holder = Some(source);
                }
                _ => return Err(Conflict),
            }
        }
        let account = account_seen.ok_or(Conflict)?;
        if !character_seen || !request.changes.iter().any(|change| matches!(&change.state,AdmissionAuthorityGuardStateV1::Character{account_id,..} if account_id==account)) {return Err(Conflict)}
        // Preserve every ordinary publisher/CAS/security monotonicity check.
        // Only the two claim changes proven above are normalized for that check.
        unchanged_claims.validate_locked(current)?;
        Ok(request)
    }
}
