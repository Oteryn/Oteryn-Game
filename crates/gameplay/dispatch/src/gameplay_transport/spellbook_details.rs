//! Read-only full spellbook projection. The exact actor fence and known-spell reader
//! are shared with the existing catalog; no imported client metadata reaches casting.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::spell::{Carrier, SpellBook, Vocation};
use oteryn_protocol_oteryn::spellbook_details::{
    CAPABILITY_SPELLBOOK_DETAILS_V1, DetailedSpell, PAGE_SIZE, Page, Query,
};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_SPELLBOOK_DETAILS_V1)
        .collect()
}
fn vocation_bit(vocation: Vocation) -> u16 {
    1 << match vocation {
        Vocation::None => 0,
        Vocation::Druid => 1,
        Vocation::ElderDruid => 2,
        Vocation::Sorcerer => 3,
        Vocation::MasterSorcerer => 4,
        Vocation::Knight => 5,
        Vocation::EliteKnight => 6,
        Vocation::Paladin => 7,
        Vocation::RoyalPaladin => 8,
        Vocation::Monk => 9,
        Vocation::ExaltedMonk => 10,
    }
}
fn page(
    book: &SpellBook,
    entries: Vec<oteryn_protocol_oteryn::actor_spell_catalog::SpellCatalogEntry>,
    query: &Query,
) -> Option<Page> {
    if !query.valid() {
        return None;
    }
    let detailed: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            let spell = book
                .spells
                .get(usize::try_from(entry.cast_index.checked_sub(1)?).ok()?)?;
            Some(DetailedSpell {
                spell: entry,
                vocations: spell
                    .vocations
                    .iter()
                    .fold(0, |bits, &vocation| bits | vocation_bit(vocation)),
                premium: spell.premium,
                magic_level: match spell.carrier {
                    Carrier::Rune { magic_level, .. } => magic_level,
                    _ => 0,
                },
            })
        })
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .filter(|entry| query.matches(entry))
        .collect();
    let total = u16::try_from(detailed.len()).ok()?;
    let start = usize::from(query.page) * PAGE_SIZE;
    if start >= detailed.len() && !(start == 0 && detailed.is_empty()) {
        return None;
    }
    Some(Page {
        page: query.page,
        total,
        entries: detailed.into_iter().skip(start).take(PAGE_SIZE).collect(),
    })
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn spellbook_details_page(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SPELLBOOK_DETAILS_V1)
        {
            return None;
        }
        let query = Query::decode(payload)?;
        let actor = admitted.runtime_actor?;
        let entries = self
            .actor_spell_catalog_with_visibility(actor, admitted.game_session_id, true)
            .await?;
        page(self.spells, entries, &query)?.encode()
    }
}
macro_rules! authority_methods {
    () => {
        fn spellbook_details_available(&self) -> bool {
            false
        }
        fn spellbook_details_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
            _payload: Vec<u8>,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn spellbook_details_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
        async fn spellbook_details_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
            payload: Vec<u8>,
        ) -> Option<Vec<u8>> {
            self.spellbook_details_page(admitted, &payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spellbook_details_uses_real_book_vocations_premium_and_filters() -> Result<(), String> {
        let book = crate::spell::cast::v1_spell_book()?;
        let state = crate::spell::cast::PlayerSpellState::new(
            crate::gameplay_transport::actor_spell::tests::FACTS,
            0,
            0,
        )
        .ok_or("state")?;
        let entries = super::super::spell_catalog::catalog_with_visibility(&book, &state, 0, true)
            .ok_or("catalog")?;
        let first = page(&book, entries.clone(), &Query::default()).ok_or("page")?;
        assert!(!first.entries.is_empty());
        assert!(first.total as usize >= first.entries.len());
        assert_eq!(
            Page::decode(&first.encode().ok_or("first encode")?),
            Some(first.clone())
        );
        for entry in &first.entries {
            let source = &book.spells[entry.spell.cast_index as usize - 1];
            assert_eq!(entry.premium, source.premium);
            assert_eq!(
                entry.vocations,
                source
                    .vocations
                    .iter()
                    .fold(0, |bits, &v| bits | vocation_bit(v))
            );
        }
        let premium = page(
            &book,
            entries.clone(),
            &Query {
                premium: 2,
                ..Default::default()
            },
        )
        .ok_or("premium")?;
        assert!(premium.entries.iter().all(|entry| entry.premium));
        let monk = page(
            &book,
            entries,
            &Query {
                vocation: 10,
                ..Default::default()
            },
        )
        .ok_or("monk")?;
        assert!(
            monk.entries
                .iter()
                .all(|entry| entry.vocations & vocation_bit(Vocation::Monk) != 0)
        );
        assert_eq!(Page::decode(&monk.encode().ok_or("encode")?), Some(monk));
        Ok(())
    }
}
