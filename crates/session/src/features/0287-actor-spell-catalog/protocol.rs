//! Capability 287 / domain 100. Wire rules: docs/contracts/protocol-oteryn/v1/actor-spell-catalog-v1.md:3-6.
//! oteryn-choice: donors have no Oteryn revisioned catalogue; retain validated replacements in the existing domain store.
use crate::*;
pub use oteryn_protocol_oteryn::actor_spell_catalog::*;
pub use oteryn_protocol_oteryn::spellbook_details;

fn malformed() -> SessionError {
    SessionError::Protocol(FoundationProtocolError::MalformedFrame)
}

/// A complete catalogue at one connection-local revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellCatalog {
    pub revision: u64,
    pub entries: Vec<SpellCatalogEntry>,
}

/// Decode the existing bounded wire format without changing confirmed state.
pub fn decode_snapshot(row: &GatedSnapshot) -> Result<SpellCatalog, SessionError> {
    if row.domain_id != STATE_DOMAIN_ACTOR_SPELL_CATALOG
        || row.snapshot_type != 1
        || row.revision == 0
    {
        return Err(malformed());
    }
    Ok(SpellCatalog {
        revision: row.revision,
        entries: decode_catalog(&row.payload).map_err(|_| malformed())?,
    })
}

pub(crate) fn validate_snapshot(
    row: &GatedSnapshot,
    previous: Option<u64>,
) -> Result<(), SessionError> {
    // oteryn-choice: recovery snapshots may repeat or skip revisions, like other domain snapshots,
    // but must not roll back confirmed state; fresh admissions start at revision 1.
    if previous.map_or(row.revision != 1, |revision| row.revision < revision) {
        return Err(malformed());
    }
    decode_snapshot(row)?;
    Ok(())
}

impl<S: SessionStream> Session<S> {
    /// Latest validated complete catalogue; a fresh session has no inherited catalogue.
    pub fn spell_catalog(&self) -> Result<Option<SpellCatalog>, SessionError> {
        self.gated_snapshot(STATE_DOMAIN_ACTOR_SPELL_CATALOG)
            .map(decode_snapshot)
            .transpose()
    }

    pub(crate) fn record_spell_catalog(
        &mut self,
        base: u64,
        new: u64,
        kind: u32,
        payload: &[u8],
    ) -> Result<GatedSnapshot, SessionError> {
        if !self.is_selected(CAPABILITY_ACTOR_SPELL_CATALOG_V1) {
            return Err(SessionError::UnselectedDomain {
                domain_id: STATE_DOMAIN_ACTOR_SPELL_CATALOG,
            });
        }
        let revision = self
            .gated_revisions
            .iter_mut()
            .find(|(id, _)| *id == STATE_DOMAIN_ACTOR_SPELL_CATALOG)
            .ok_or(SessionError::MissingDomain(
                STATE_DOMAIN_ACTOR_SPELL_CATALOG,
            ))?;
        let row = self
            .gated_snapshots
            .iter_mut()
            .find(|row| row.domain_id == STATE_DOMAIN_ACTOR_SPELL_CATALOG)
            .ok_or(SessionError::MissingDomain(
                STATE_DOMAIN_ACTOR_SPELL_CATALOG,
            ))?;
        if kind != 1 || base != row.revision || base.checked_add(1) != Some(new) {
            return Err(malformed());
        }
        decode_catalog(payload).map_err(|_| malformed())?;
        let next = GatedSnapshot {
            revision: new,
            payload: payload.to_vec(),
            ..row.clone()
        };
        *row = next.clone();
        revision.1 = new;
        Ok(next)
    }
}

pub(crate) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let delta = decode_state_delta(payload)?;
    let value = session.record_spell_catalog(
        delta.base_revision,
        delta.new_revision,
        delta.delta_type,
        delta.payload,
    )?;
    Ok(SessionEvent::GatedSnapshotReplaced(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}
