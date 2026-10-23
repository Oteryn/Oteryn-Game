//! Project authorised choices without granting a key or changing stored selection.
use super::*;
use crate::domain::appearance::{AccountUnlocks, EarnedCosmetics, UnlockFact};

pub(super) fn catalogue(
    content: &NativeCosmeticsView,
    state: &CurrentCharacterAppearance,
    root_revision: u64,
    context: wire::Context,
) -> Option<wire::Catalogue> {
    let unlocks = EarnedCosmetics {
        content,
        facts: &state.earned,
    };
    let mut outfits = Vec::new();
    for key in content.outfit_keys() {
        let key = key.to_owned();
        if !content.outfit_is_starter(&key, state.sex)
            && unlocks.outfit(&key) != UnlockFact::Compatible
        {
            continue;
        }
        let mut mask = 0;
        for addon in 1..=content.outfit_addon_count(&key)? {
            if content.addon_is_starter(&key, state.sex, addon)
                || unlocks.addon(&key, addon) == UnlockFact::Compatible
            {
                mask |= 1 << (addon - 1);
            }
        }
        outfits.push(wire::OutfitOption {
            look_type: content.look_type(&key, state.sex)?,
            key,
            permitted_addons: mask,
        });
    }
    let mounts = content
        .mount_keys()
        .map(str::to_owned)
        .filter(|key| unlocks.mount(key) == UnlockFact::Compatible)
        .collect();
    let selected = state.selected.as_ref().map(|s| wire::Selected {
        selection: wire::Selection {
            outfit_key: s.selection.outfit_key.clone(),
            colours: s.selection.colours,
            addons: s.selection.addons,
            mount_key: s.selection.mount_key.clone(),
        },
        revision: s.committed_character_revision.get(),
        provenance: s.content_revision.clone(),
    });
    let catalogue = wire::Catalogue {
        context,
        root_revision,
        selected,
        palette: content.palette().to_vec(),
        outfits,
        mounts,
    };
    catalogue.encode().ok()?;
    Some(catalogue)
}
pub(super) fn base(
    content: &NativeCosmeticsView,
    state: &CurrentCharacterAppearance,
) -> (AppearanceSelection<String>, Option<u64>) {
    let unlocks = EarnedCosmetics {
        content,
        facts: &state.earned,
    };
    if let Some(selected) = &state.selected
        && crate::domain::appearance::validate_selection(
            &selected.selection,
            state.sex,
            content,
            &unlocks,
        )
        .is_ok()
    {
        return (
            selected.selection.clone(),
            Some(selected.committed_character_revision.get()),
        );
    }
    (content.fallback(state.sex), None)
}
