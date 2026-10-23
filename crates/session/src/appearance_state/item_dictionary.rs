//! Item transforms resolve only against the caller's pinned client artifact dictionary.
use super::*;

/// Item references exported by one verified client artifact. The loader supplies the artifact
/// generation and its complete sorted reference set; absence fails closed for Item looks.
#[derive(Debug, Clone, Copy)]
pub struct AppearanceItemDictionary<'a> {
    pub content_generation: [u8; 32],
    pub definition_refs: &'a [u32],
}

#[derive(Debug, Clone)]
pub(super) struct ItemDictionary {
    references: std::collections::BTreeSet<u32>,
}

pub(super) fn admit(
    items: Option<AppearanceItemDictionary<'_>>,
    generation: [u8; 32],
) -> Result<Option<ItemDictionary>, SessionError> {
    let Some(items) = items else { return Ok(None) };
    if items.content_generation != generation
        || items.definition_refs.first() == Some(&0)
        || items
            .definition_refs
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(invalid());
    }
    Ok(Some(ItemDictionary {
        references: items.definition_refs.iter().copied().collect(),
    }))
}

impl AppearanceState {
    pub(super) fn validate_item(&self, reference: u32, selected: bool) -> Result<(), SessionError> {
        if !selected
            || !self
                .items
                .as_ref()
                .is_some_and(|items| items.references.contains(&reference))
        {
            return Err(invalid());
        }
        Ok(())
    }
}
