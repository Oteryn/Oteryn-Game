use oteryn_client_runtime::account_achievements::AccountAchievementsResult;
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AchievementsPanel {
    pub selected: bool,
    pub catalog: super::catalog::CatalogState,
    pub open: bool,
    pub pending: Option<u32>,
    pub page: Option<AccountAchievementsResult>,
    pub error: Option<&'static str>,
}
impl AchievementsPanel {
    pub fn has_previous(&self) -> bool {
        self.page.as_ref().is_some_and(|p| p.page > 0)
    }
    pub fn has_next(&self) -> bool {
        self.page
            .as_ref()
            .is_some_and(|p| p.has_more && p.page < u32::MAX)
    }
}
