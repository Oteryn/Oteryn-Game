use oteryn_client_runtime::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult,
};
use std::collections::BTreeSet;
const MAX_ROWS: usize = 16_384;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AchievementFilter {
    #[default]
    All,
    Regular,
    Secret,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AchievementSort {
    #[default]
    Name,
    Points,
    Grade,
    Earned,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogState {
    pub enabled: bool,
    pub collecting: bool,
    pub ready: bool,
    pub rows: Vec<AccountAchievementRow>,
    pub points: u32,
    pub filter: AchievementFilter,
    pub sort: AchievementSort,
    pub search: String,
    pub page: usize,
    pub revision: u64,
    building: Vec<AccountAchievementRow>,
    keys: BTreeSet<String>,
    totals: Option<(u32, u32)>,
    next_page: u32,
}
impl CatalogState {
    pub fn set_options(&mut self, search: &str, filter: AchievementFilter, sort: AchievementSort) {
        if search.len() > 512 || search.contains('\0') {
            return;
        }
        if self.search != search || self.filter != filter || self.sort != sort {
            self.page = 0;
            self.search = search.to_owned();
            self.filter = filter;
            self.sort = sort;
        }
    }

    pub(super) fn begin(&mut self) {
        self.collecting = true;
        self.building.clear();
        self.keys.clear();
        self.totals = None;
        self.next_page = 0;
    }
    /// Publish only a complete account list. A changed count/points or duplicate requires refresh.
    pub(super) fn accept(&mut self, page: &AccountAchievementsResult) -> Result<(), &'static str> {
        if page.page == 0 {
            self.begin();
        }
        let totals = (page.fact_count, page.total_points);
        if !self.collecting
            || page.page != self.next_page
            || page.fact_count as usize > MAX_ROWS
            || self.totals.is_some_and(|v| v != totals)
            || self.building.len().saturating_add(page.rows.len()) > page.fact_count as usize
        {
            self.collecting = false;
            return Err(tr!(
                "The account list changed or exceeded its limit. Refresh to load it again"
            ));
        }
        self.totals = Some(totals);
        for row in &page.rows {
            if !self.keys.insert(row.key.clone()) {
                self.collecting = false;
                return Err(tr!("The account list changed. Refresh to load it again"));
            }
            self.building.push(row.clone());
        }
        self.revision = self.revision.saturating_add(1);
        self.next_page = self.next_page.saturating_add(1);
        if !page.has_more {
            if self.building.len() != page.fact_count as usize {
                self.collecting = false;
                return Err(tr!("The server returned an incomplete account list"));
            }
            self.rows = std::mem::take(&mut self.building);
            self.keys.clear();
            self.points = page.total_points;
            self.ready = true;
            self.collecting = false;
            self.page = 0;
        }
        Ok(())
    }
}
pub fn catalog_rows(catalog: &CatalogState) -> Vec<&AccountAchievementRow> {
    let search = catalog.search.trim().to_lowercase();
    let mut rows: Vec<_> = catalog
        .rows
        .iter()
        .filter(|r| match catalog.filter {
            AchievementFilter::All => true,
            AchievementFilter::Regular => !r.secret,
            AchievementFilter::Secret => r.secret,
        })
        .filter(|r| {
            r.name.to_lowercase().contains(&search)
                || r.description.to_lowercase().contains(&search)
                || r.key.to_lowercase().contains(&search)
        })
        .collect();
    rows.sort_by(|a, b| {
        let order = match catalog.sort {
            AchievementSort::Name => a.name.cmp(&b.name),
            AchievementSort::Points => b.points.cmp(&a.points),
            AchievementSort::Grade => b.grade.cmp(&a.grade),
            AchievementSort::Earned => b.earned_at.cmp(&a.earned_at),
        };
        order.then_with(|| a.key.cmp(&b.key))
    });
    rows
}
