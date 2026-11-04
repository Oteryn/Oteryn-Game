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
        p.error = Some("Session ended");
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
            p.page = Some(page.clone());
            p.error = None;
        }
        Response::Page(_) => p.error = Some("Invalid achievements page"),
        Response::Refused => p.error = Some("The server refused the achievements query"),
    }
}
