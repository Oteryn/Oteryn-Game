//! Read the latest catalogue from the session owner, including after reconnect.
#[path = "state.rs"]
mod state;
pub use oteryn_session::spell_catalog::spellbook_details;
pub use oteryn_session::spell_catalog::{SpellCatalog, SpellCatalogEntry};
use oteryn_session::{Session, SessionError, SessionEvent, SessionStream};
pub use state::CatalogState;

/// Read confirmed state from this session rather than retaining a catalogue across admissions.
pub fn snapshot<S: SessionStream>(
    session: &Session<S>,
) -> Result<Option<SpellCatalog>, SessionError> {
    session.spell_catalog()
}

/// Project complete replacements delivered through the existing session event stream.
// oteryn-choice: use the existing replacement event; donors do not define Oteryn client events.
pub fn replacement(event: &SessionEvent) -> Option<Result<SpellCatalog, SessionError>> {
    match event {
        SessionEvent::GatedSnapshotReplaced(delta) if delta.value.domain_id == 100 => {
            Some(oteryn_session::spell_catalog::decode_snapshot(&delta.value))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{AppliedDelta, GatedSnapshot, spell_catalog::*};

    #[test]
    fn gap_spell_catalog_runtime_consumer_is_negotiated() -> Result<(), Box<dyn std::error::Error>>
    {
        assert!(oteryn_session::CLIENT_SUPPORTED_CAPABILITIES.contains(&287));
        let mut row = GatedSnapshot {
            domain_id: 100,
            revision: 1,
            snapshot_type: 1,
            payload: vec![],
        };
        let entry = SpellCatalogEntry {
            cast_index: 1,
            name: "Light Healing".into(),
            words: "exura".into(),
            level: 8,
            mana: 20,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: false,
            cooldown_groups: vec![1],
            rune_item: None,
        };
        row.payload = encode_catalog(std::slice::from_ref(&entry)).map_err(|e| format!("{e:?}"))?;
        let event = |value| {
            SessionEvent::GatedSnapshotReplaced(AppliedDelta {
                server_sequence: 41,
                base_revision: 0,
                new_revision: 1,
                value,
            })
        };
        let catalog = replacement(&event(row.clone())).ok_or("replacement")??;
        assert_eq!(catalog.entries, [entry]);
        assert_eq!(catalog.revision, 1);
        row.payload.clear();
        row.revision = 2;
        assert!(
            replacement(&event(row.clone()))
                .ok_or("replacement")??
                .entries
                .is_empty()
        );
        let mut entry = catalog.entries[0].clone();
        for cast_index in 1..=MAX_ENTRIES + 1 {
            entry.cast_index = u32::try_from(cast_index)?;
            row.payload.extend(
                encode_catalog(std::slice::from_ref(&entry)).map_err(|e| format!("{e:?}"))?,
            );
            if cast_index == MAX_ENTRIES {
                assert_eq!(
                    replacement(&event(row.clone()))
                        .ok_or("replacement")??
                        .entries
                        .len(),
                    MAX_ENTRIES
                );
            }
        }
        assert!(
            replacement(&event(row.clone()))
                .ok_or("replacement")?
                .is_err()
        );
        row.payload = vec![0; MAX_CATALOG_BYTES + 1];
        assert!(
            replacement(&event(row.clone()))
                .ok_or("replacement")?
                .is_err()
        );
        row.domain_id = 22;
        assert!(replacement(&event(row)).is_none());
        Ok(())
    }
}
