//! Lossless historical guard encoding. Decoding never registers an owning source.
use super::DurabilityError;
use oteryn_game_server::foundation::admission_authority_publication::*;
use oteryn_game_server::foundation::fnd04_verifier::*;
use oteryn_game_server::foundation::*;

type Result<T> = std::result::Result<T, DurabilityError>;
fn invalid<T>() -> Result<T> {
    Err(DurabilityError::InvalidStoredState)
}
pub(super) fn checked<T, E>(result: std::result::Result<T, E>) -> Result<T> {
    result.map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) struct Writer {
    pub bytes: Vec<u8>,
    maximum: usize,
}
impl Writer {
    pub fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
        }
    }
    pub fn bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(DurabilityError::InvalidStoredState)?;
        if length > self.maximum {
            return invalid();
        }
        checked(self.bytes.try_reserve_exact(bytes.len()))?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn tag(&mut self, value: u8) -> Result<()> {
        self.bytes(&[value])
    }
    pub fn boolean(&mut self, value: bool) -> Result<()> {
        self.tag(u8::from(value))
    }
    pub fn u64(&mut self, value: u64) -> Result<()> {
        self.bytes(&value.to_be_bytes())
    }
    pub fn i64(&mut self, value: i64) -> Result<()> {
        self.bytes(&value.to_be_bytes())
    }
    pub fn text(&mut self, value: &str) -> Result<()> {
        self.u64(checked(u64::try_from(value.len()))?)?;
        self.bytes(value.as_bytes())
    }
}
pub(super) struct Reader<'a> {
    remaining: &'a [u8],
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }
    pub fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let (value, rest) = self
            .remaining
            .split_at_checked(length)
            .ok_or(DurabilityError::InvalidStoredState)?;
        self.remaining = rest;
        Ok(value)
    }
    pub fn bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        checked(self.take(N)?.try_into())
    }
    pub fn tag(&mut self) -> Result<u8> {
        Ok(self.bytes::<1>()?[0])
    }
    pub fn boolean(&mut self) -> Result<bool> {
        match self.tag()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => invalid(),
        }
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.bytes()?))
    }
    pub fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.bytes()?))
    }
    pub fn text(&mut self) -> Result<String> {
        let length = checked(usize::try_from(self.u64()?))?;
        // Check the complete declared field against retained input before allocating.
        Ok(checked(std::str::from_utf8(self.take(length)?))?.to_owned())
    }
    pub fn finish(self) -> Result<()> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            invalid()
        }
    }
}

pub(super) fn write_provenance(w: &mut Writer, p: &FreshEvidenceProvenanceV1) -> Result<()> {
    w.text(&p.source_authority)?;
    w.tag(match p.purpose {
        FreshEvidencePurposeV1::PlatformSecurity => 1,
        FreshEvidencePurposeV1::SigningTrust => 2,
    })?;
    w.tag(match p.scope {
        Fnd04EvidenceScope::FreshAdmission => 1,
        Fnd04EvidenceScope::ExistingActorRecovery => 2,
    })?;
    w.u64(p.source_revision)?;
    w.u64(p.accepted_source_revision)?;
    w.text(&p.decision_identity)?;
    w.text(&p.accepted_decision_identity)?;
    w.i64(p.source_observed_at)?;
    w.u64(p.clock_uncertainty_seconds)?;
    w.u64(p.publication_revision)
}
pub(super) fn read_provenance(r: &mut Reader<'_>) -> Result<FreshEvidenceProvenanceV1> {
    Ok(FreshEvidenceProvenanceV1 {
        source_authority: r.text()?,
        purpose: match r.tag()? {
            1 => FreshEvidencePurposeV1::PlatformSecurity,
            2 => FreshEvidencePurposeV1::SigningTrust,
            _ => return invalid(),
        },
        scope: match r.tag()? {
            1 => Fnd04EvidenceScope::FreshAdmission,
            2 => Fnd04EvidenceScope::ExistingActorRecovery,
            _ => return invalid(),
        },
        source_revision: r.u64()?,
        accepted_source_revision: r.u64()?,
        decision_identity: r.text()?,
        accepted_decision_identity: r.text()?,
        source_observed_at: r.i64()?,
        clock_uncertainty_seconds: r.u64()?,
        publication_revision: r.u64()?,
    })
}
pub(super) fn write_security(w: &mut Writer, s: &FreshAccountSecurityObservationV1) -> Result<()> {
    w.text(&s.account_id)?;
    w.u64(s.minimum_generation)?;
    w.boolean(s.allowed)?;
    write_provenance(w, &s.provenance)
}
pub(super) fn read_security(r: &mut Reader<'_>) -> Result<FreshAccountSecurityObservationV1> {
    Ok(FreshAccountSecurityObservationV1 {
        account_id: r.text()?,
        minimum_generation: r.u64()?,
        allowed: r.boolean()?,
        provenance: read_provenance(r)?,
    })
}
pub(super) fn write_scope(w: &mut Writer, scope: RuntimeScopeRefV1) -> Result<()> {
    match scope {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => {
            w.tag(1)?;
            w.bytes(world_id.as_bytes())?;
            w.bytes(channel_id.as_bytes())
        }
        RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => {
            w.tag(2)?;
            w.bytes(world_id.as_bytes())?;
            w.bytes(&instance_id)
        }
    }
}
pub(super) fn read_scope(r: &mut Reader<'_>) -> Result<RuntimeScopeRefV1> {
    let tag = r.tag()?;
    let world = checked(WorldId::decode(&r.bytes::<16>()?))?;
    match tag {
        1 => Ok(RuntimeScopeRefV1::channel(
            world,
            checked(ChannelId::decode(&r.bytes::<16>()?))?,
        )),
        2 => checked(RuntimeScopeRefV1::instance(world, r.bytes()?)),
        _ => invalid(),
    }
}
fn write_key(w: &mut Writer, key: &AdmissionAuthorityGuardKeyV1) -> Result<()> {
    match key {
        AdmissionAuthorityGuardKeyV1::Account { account_id } => {
            w.tag(1)?;
            w.text(account_id)
        }
        AdmissionAuthorityGuardKeyV1::Character(id) => {
            w.tag(2)?;
            w.bytes(id.as_bytes())
        }
        AdmissionAuthorityGuardKeyV1::Runtime(scope) => {
            w.tag(3)?;
            write_scope(w, *scope)
        }
        AdmissionAuthorityGuardKeyV1::SigningTrust { key_id, profile } => {
            w.tag(4)?;
            w.text(key_id)?;
            w.text(profile)
        }
    }
}
fn read_key(r: &mut Reader<'_>) -> Result<AdmissionAuthorityGuardKeyV1> {
    Ok(match r.tag()? {
        1 => AdmissionAuthorityGuardKeyV1::Account {
            account_id: r.text()?,
        },
        2 => AdmissionAuthorityGuardKeyV1::Character(checked(CharacterId::decode(
            &r.bytes::<16>()?,
        ))?),
        3 => AdmissionAuthorityGuardKeyV1::Runtime(read_scope(r)?),
        4 => AdmissionAuthorityGuardKeyV1::SigningTrust {
            key_id: r.text()?,
            profile: r.text()?,
        },
        _ => return invalid(),
    })
}
fn write_state(w: &mut Writer, state: &AdmissionAuthorityGuardStateV1) -> Result<()> {
    match state {
        AdmissionAuthorityGuardStateV1::Account { security, presence } => {
            w.tag(1)?;
            write_security(w, security)?;
            w.boolean(presence.is_some())?;
            if let Some((character, session)) = presence {
                w.bytes(character.as_bytes())?;
                w.bytes(session.as_bytes())?;
            }
        }
        AdmissionAuthorityGuardStateV1::Character {
            account_id,
            world_id,
            eligible,
            lease_generation,
            holder,
        } => {
            w.tag(2)?;
            w.text(account_id)?;
            w.bytes(world_id.as_bytes())?;
            w.boolean(*eligible)?;
            w.u64(*lease_generation)?;
            w.boolean(holder.is_some())?;
            if let Some(holder) = holder {
                w.bytes(holder.as_bytes())?;
            }
        }
        AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready,
            route_revision,
            runtime_observation_revision,
            protocol_major,
            transport_profile,
            ruleset_revision,
            content_revision,
            map_revision,
            world_policy_revision,
            offer_revision,
        } => {
            w.tag(3)?;
            w.u64(*ownership_generation)?;
            w.boolean(*ready)?;
            w.text(route_revision)?;
            w.text(runtime_observation_revision)?;
            w.u64(*protocol_major)?;
            w.u64(*transport_profile)?;
            for value in [
                ruleset_revision,
                content_revision,
                map_revision,
                world_policy_revision,
                offer_revision,
            ] {
                w.text(value)?;
            }
        }
        AdmissionAuthorityGuardStateV1::SigningTrust {
            public_key,
            trusted,
        } => {
            w.tag(4)?;
            w.bytes(public_key)?;
            w.boolean(*trusted)?;
        }
    }
    Ok(())
}
fn read_state(r: &mut Reader<'_>) -> Result<AdmissionAuthorityGuardStateV1> {
    Ok(match r.tag()? {
        1 => {
            let security = read_security(r)?;
            let presence = if r.boolean()? {
                Some((
                    checked(CharacterId::decode(&r.bytes::<16>()?))?,
                    checked(GameSessionId::decode(&r.bytes::<16>()?))?,
                ))
            } else {
                None
            };
            AdmissionAuthorityGuardStateV1::Account { security, presence }
        }
        2 => {
            let account_id = r.text()?;
            let world_id = checked(WorldId::decode(&r.bytes::<16>()?))?;
            let eligible = r.boolean()?;
            let lease_generation = r.u64()?;
            let holder = if r.boolean()? {
                Some(checked(GameSessionId::decode(&r.bytes::<16>()?))?)
            } else {
                None
            };
            AdmissionAuthorityGuardStateV1::Character {
                account_id,
                world_id,
                eligible,
                lease_generation,
                holder,
            }
        }
        3 => AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: r.u64()?,
            ready: r.boolean()?,
            route_revision: r.text()?,
            runtime_observation_revision: r.text()?,
            protocol_major: r.u64()?,
            transport_profile: r.u64()?,
            ruleset_revision: r.text()?,
            content_revision: r.text()?,
            map_revision: r.text()?,
            world_policy_revision: r.text()?,
            offer_revision: r.text()?,
        },
        4 => AdmissionAuthorityGuardStateV1::SigningTrust {
            public_key: r.bytes()?,
            trusted: r.boolean()?,
        },
        _ => return invalid(),
    })
}
pub(super) fn write_change(
    w: &mut Writer,
    c: &AdmissionAuthorityPublicationChangeV1,
) -> Result<()> {
    write_key(w, &c.key)?;
    w.text(&c.source.authority)?;
    w.tag(match c.source.purpose {
        AdmissionPublicationPurposeV1::AccountSecurityAndPresence => 1,
        AdmissionPublicationPurposeV1::CharacterOwnershipAndLease => 2,
        AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness => 3,
        AdmissionPublicationPurposeV1::FixedFreshSigningTrust => 4,
    })?;
    w.u64(c.source.source_revision)?;
    w.text(&c.source.decision_identity)?;
    w.i64(c.source.source_observed_at)?;
    w.u64(c.source.clock_uncertainty_seconds)?;
    match c.precondition {
        AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water,
        } => {
            w.tag(1)?;
            w.boolean(restored_publication_high_water.is_some())?;
            if let Some(value) = restored_publication_high_water {
                w.u64(value)?;
            }
        }
        AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision,
        } => {
            w.tag(2)?;
            w.u64(expected_publication_revision)?;
        }
    }
    w.u64(c.publication_revision)?;
    write_state(w, &c.state)
}
pub(super) fn read_change(r: &mut Reader<'_>) -> Result<AdmissionAuthorityPublicationChangeV1> {
    let key = read_key(r)?;
    let authority = r.text()?;
    let purpose = match r.tag()? {
        1 => AdmissionPublicationPurposeV1::AccountSecurityAndPresence,
        2 => AdmissionPublicationPurposeV1::CharacterOwnershipAndLease,
        3 => AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
        4 => AdmissionPublicationPurposeV1::FixedFreshSigningTrust,
        _ => return invalid(),
    };
    let source = AdmissionPublicationSourceV1 {
        authority,
        purpose,
        source_revision: r.u64()?,
        decision_identity: r.text()?,
        source_observed_at: r.i64()?,
        clock_uncertainty_seconds: r.u64()?,
    };
    let precondition = match r.tag()? {
        1 => AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: if r.boolean()? { Some(r.u64()?) } else { None },
        },
        2 => AdmissionPublicationPreconditionV1::CompareAndSet {
            expected_publication_revision: r.u64()?,
        },
        _ => return invalid(),
    };
    Ok(AdmissionAuthorityPublicationChangeV1 {
        key,
        source,
        precondition,
        publication_revision: r.u64()?,
        state: read_state(r)?,
    })
}
pub(super) fn write_changes(
    w: &mut Writer,
    changes: &[AdmissionAuthorityPublicationChangeV1],
) -> Result<()> {
    if changes.len() > 4 {
        return invalid();
    }
    w.tag(checked(u8::try_from(changes.len()))?)?;
    for change in changes {
        write_change(w, change)?;
    }
    Ok(())
}
pub(super) fn read_changes(
    r: &mut Reader<'_>,
) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>> {
    let count = r.tag()?;
    if count > 4 {
        return invalid();
    }
    (0..count).map(|_| read_change(r)).collect()
}

/// Encode one historical guard change; this does not prepare a publication.
pub fn encode_guard(
    change: &AdmissionAuthorityPublicationChangeV1,
    maximum_bytes: usize,
) -> Result<String> {
    let mut writer = Writer::new(maximum_bytes);
    write_change(&mut writer, change)?;
    super::fresh_admission::encode_envelope(&writer.bytes, maximum_bytes)
}
/// Decode a historical guard; consumers must still compare every SQL mirror and
/// invoke their sealed Foundation predicate against independently current rows.
pub fn decode_guard(
    encoded: &str,
    maximum_bytes: usize,
) -> Result<AdmissionAuthorityPublicationChangeV1> {
    let bytes = super::fresh_admission::decode_envelope(encoded, maximum_bytes)?;
    let mut reader = Reader::new(&bytes);
    let change = read_change(&mut reader)?;
    reader.finish()?;
    Ok(change)
}
