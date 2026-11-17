use super::AchievementsPanel;
use oteryn_client_runtime::account_achievements::Response;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Previous,
    Next,
}
pub fn request(p: &mut AchievementsPanel, intent: Intent) -> Option<u32> {
    if !p.selected {
        p.error = Some(tr!("Session ended"));
        return None;
    }
    if p.pending.is_some() {
        return None;
    }
    let page = match intent {
        Intent::Refresh => 0,
        Intent::Previous if p.has_previous() => p.page.as_ref()?.page.checked_sub(1)?,
        Intent::Next if p.has_next() => p.page.as_ref()?.page.checked_add(1)?,
        _ => return None,
    };
    if matches!(intent, Intent::Refresh) && p.catalog.enabled {
        p.catalog.begin();
    }
    p.pending = Some(page);
    p.error = None;
    Some(page)
}
pub fn apply_result(p: &mut AchievementsPanel, response: &Response) {
    let Some(wanted) = p.pending.take() else {
        return;
    };
    match response {
        Response::Page(page) if page.page == wanted => {
            p.catalog.revision = p.catalog.revision.saturating_add(1);
            p.error = if p.catalog.enabled {
                p.catalog.accept(page).err()
            } else {
                None
            };
            p.page = Some(page.clone());
        }
        Response::Page(_) => {
            p.catalog.collecting = false;
            p.error = Some(tr!("Invalid achievements page"));
        }
        Response::Refused => {
            p.catalog.collecting = false;
            p.error = Some(tr!("The server refused the achievements query"));
        }
    }
}

pub fn catalog_next_request(p: &mut AchievementsPanel) -> Option<u32> {
    if p.catalog.enabled
        && p.catalog.collecting
        && p.error.is_none()
        && p.pending.is_none()
        && p.has_next()
    {
        request(p, Intent::Next)
    } else {
        None
    }
}
