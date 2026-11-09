use super::*;
use crate::domain::appearance::AppearanceContent;
use wire::gallery::{Entry, Kind, PAGE_SIZE, Page};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn cyclopedia_gallery(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Result<Vec<u8>, ()> {
        let offset = wire::gallery::decode_request(payload).map_err(|_| ())?;
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
        let state = self
            .root
            .read_current_character_appearance_state(
                self.character,
                self.holder,
                fence,
                content.earned_grant_keys(),
            )
            .await
            .map_err(|_| ())?;
        let catalogue = super::super::appearance_change::catalogue::catalogue(
            &content,
            &state,
            fence.expected_character_revision.get(),
            context,
        )
        .ok_or(())?;
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
        let native = self
            .active_generation
            .ok_or(())?
            .native_gameplay()
            .ok_or(())?;
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
