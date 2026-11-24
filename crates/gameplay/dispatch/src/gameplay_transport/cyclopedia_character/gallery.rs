use super::*;
use crate::content::native_gameplay::{NativeCosmeticsView, NativeGameplayState};
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
use crate::durability::character_appearance::CurrentCharacterAppearance;
use wire::gallery::owned::{self, Query, Source};
use wire::gallery::{Entry, Kind, PAGE_SIZE, Page};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn cyclopedia_gallery(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Result<Vec<u8>, ()> {
        let query = if payload.first() == Some(&owned::SELECTOR) {
            if !admitted.continuity.selected_capabilities.contains(owned::CAPABILITY) {
                return Err(());
            }
            Some(Query::decode(payload).map_err(|_| ())?)
        } else {
            None
        };
        let offset = match query {
            Some(query) => query.offset,
            None => wire::gallery::decode_request(payload).map_err(|_| ())?,
        };
        let fence =
            super::super::character_gameplay_fence::current(self, admitted.game_session_id, false)
                .await
                .ok_or(())?;
        if fence.connection_generation.get() != admitted.continuity.connection_generation
            || fence.character_id != admitted.item_fence.ok_or(())?.character_id
        {
            return Err(());
        }
        let (context, content) = self.appearance_context().ok_or(())?;
        let grant_keys = content.earned_grant_keys();
        let state = if query.is_some() {
            self.root.read_current_character_gallery(self.character, self.holder, fence, grant_keys).await
        } else {
            self.root.read_current_character_appearance_state(self.character, self.holder, fence, grant_keys).await
        }.map_err(|_| ())?;
        let catalogue = super::super::appearance_change::catalogue::catalogue(
            &content,
            &state,
            fence.expected_character_revision.get(),
            context,
        )
        .ok_or(())?;
        let native = self
            .active_generation
            .ok_or(())?
            .native_gameplay()
            .ok_or(())?;
        if let Some(query) = query {
            return owned_page(&content, &state, native, catalogue, query)?.encode().map_err(|_| ());
        }
        let mut entries: Vec<_> = catalogue
            .outfits
            .into_iter()
            .map(|outfit| Entry {
                kind: Kind::Outfit,
                key: outfit.key,
                look_type: outfit.look_type,
            })
            .collect();
        for key in catalogue.mounts {
            let look_type = content.mount_metadata(&key).ok_or(())?.0;
            entries.push(Entry {
                kind: Kind::Mount,
                key,
                look_type,
            });
        }
        for policy in native
            .policies()
            .policies()
            .iter()
            .filter(|p| p.is_familiar)
        {
            entries.push(Entry {
                kind: Kind::Familiar,
                key: policy.definition_key.clone(),
                look_type: policy.outfit_appearance().ok_or(())?,
            });
        }
        page(entries, offset)?.encode().map_err(|_| ())
    }
}
/// Sources are resolved with the same provenance and sex checks as the selection catalogue.
fn earned_sources(
    content: &NativeCosmeticsView,
    state: &CurrentCharacterAppearance,
    target: EarnedCosmetic<'_>,
) -> u8 {
    state.earned.iter().zip(&state.earned_sources).fold(0, |mask, ((key, provenance), source)| {
        if content.earned_unlock_for_sex(key, provenance, target, state.sex) == UnlockFact::Compatible {
            mask | *source as u8
        } else {
            mask
        }
    })
}

fn owned_page(
    content: &NativeCosmeticsView,
    state: &CurrentCharacterAppearance,
    native: &NativeGameplayState,
    catalogue: oteryn_protocol_oteryn::appearance::Catalogue,
    query: Query,
) -> Result<owned::Page, ()> {
    let mut rows = Vec::new();
    for outfit in catalogue.outfits {
        let mut sources = earned_sources(content, state, EarnedCosmetic::Outfit(&outfit.key));
        if content.outfit_is_starter(&outfit.key, state.sex) {
            sources |= Source::Starter as u8;
        }
        if query.outfit_source.is_none_or(|source| sources & source as u8 != 0) {
            rows.push((Entry { kind: Kind::Outfit, key: outfit.key, look_type: outfit.look_type }, sources));
        }
    }
    if query.outfit_source.is_none() {
        for key in catalogue.mounts {
            let look_type = content.mount_metadata(&key).ok_or(())?.0;
            let source = earned_sources(content, state, EarnedCosmetic::Mount(&key));
            rows.push((Entry { kind: Kind::Mount, key, look_type }, source));
        }
        for policy in native.policies().policies().iter().filter(|policy| policy.is_familiar) {
            if let Some(look_type) = policy.outfit_appearance()
                && state.familiar_looks.binary_search(&look_type).is_ok()
            {
                rows.push((Entry {
                    kind: Kind::Familiar, key: policy.definition_key.clone(), look_type,
                }, Source::FamiliarGrant as u8));
            }
        }
    }
    rows.sort_by(|(a, _), (b, _)| (a.kind, &a.key).cmp(&(b.kind, &b.key)));
    let offset = query.offset;
    let total = u16::try_from(rows.len()).map_err(|_| ())?;
    if offset > total {
        return Err(());
    }
    let (entries, sources) = rows.into_iter().skip(usize::from(offset)).take(PAGE_SIZE).unzip();
    Ok(owned::Page { gallery: Page { offset, total, entries }, sources })
}

fn page(mut entries: Vec<Entry>, offset: u16) -> Result<Page, ()> {
    entries.sort_by(|a, b| (a.kind, &a.key).cmp(&(b.kind, &b.key)));
    let total = u16::try_from(entries.len()).map_err(|_| ())?;
    if offset > total {
        return Err(());
    }
    Ok(Page {
        offset,
        total,
        entries: entries
            .into_iter()
            .skip(usize::from(offset))
            .take(PAGE_SIZE)
            .collect(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gallery_dispatch_pages_confirmed_choices_in_stable_order() -> Result<(), ()> {
        let entries = (0..25)
            .rev()
            .map(|i| Entry {
                kind: Kind::Outfit,
                key: format!("outfit/{i:02}"),
                look_type: i + 1,
            })
            .collect();
        let first = page(
            entries,
            wire::gallery::decode_request(&wire::gallery::request(0)).map_err(|_| ())?,
        )?;
        assert_eq!(first.total, 25);
        assert_eq!(first.entries.len(), PAGE_SIZE);
        assert_eq!(first.entries[0].look_type, 1);
        assert!(page(Vec::new(), 1).is_err());
        Ok(())
    }
}
