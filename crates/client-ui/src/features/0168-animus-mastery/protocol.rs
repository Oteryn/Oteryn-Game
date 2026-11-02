use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Previous,
    Next,
}

pub fn request(panel: &mut AnimusPanel, intent: Intent) -> Option<Vec<u8>> {
    if !panel.selected || panel.pending.is_some() {
        return None;
    }
    let offset = match intent {
        Intent::Refresh => 0,
        Intent::Previous if panel.has_previous() => {
            panel.offset.saturating_sub(u16::from(MAX_ANIMUS_PAGE_KEYS))
        }
        Intent::Next if panel.has_next() => panel
            .offset
            .checked_add(u16::try_from(panel.keys.len()).ok()?)?,
        Intent::Previous | Intent::Next => return None,
    };
    let page = AnimusPage {
        offset,
        limit: MAX_ANIMUS_PAGE_KEYS,
    };
    let payload = page.encode()?.to_vec();
    panel.pending = Some(page);
    panel.error = None;
    Some(payload)
}
