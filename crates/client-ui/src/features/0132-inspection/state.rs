use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub catalogue_open: bool,
    pub catalogue_loaded: bool,
    pub catalogue_next: Option<u32>,
    pub catalogue_drop_next: Option<u16>,
    pub catalogue_query_categories: Vec<String>,
    pub catalogue_market: Option<u32>,
    pub catalogue: std::collections::BTreeMap<u32, InspectedItem>,
    pub catalogue_query: Option<catalogue::PageQuery>,
    pub catalogue_market_requested: Option<u32>,
    pub catalogue_sources: Vec<String>,
    pub catalogue_source_next: Option<u32>,
    pub catalogue_source_definition: u32,
    pub catalogue_page: Option<catalogue::Page>,
    pub catalogue_categories: std::collections::BTreeSet<String>,
    pub catalogue_definition: String,
    pub search: String,
    pub category: String,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub last_request: Option<Request>,
    pub reply: Option<Reply>,
    pub feedback: String,
}

pub(super) fn item_categories(item: &InspectedItem) -> impl Iterator<Item = &str> {
    catalogue::categories(item)
}
impl Panel {
    pub fn categories(&self) -> Vec<&str> {
        if !self.catalogue_query_categories.is_empty() {
            return self
                .catalogue_query_categories
                .iter()
                .map(String::as_str)
                .collect();
        }
        let mut categories = std::collections::BTreeMap::new();
        for category in self
            .catalogue_categories
            .iter()
            .map(String::as_str)
            .chain(self.catalogue.values().flat_map(item_categories))
        {
            categories
                .entry(category.to_lowercase())
                .or_insert(category);
        }
        categories.into_values().collect()
    }
    pub fn select_category(&mut self, previous: bool) {
        let categories = self.categories();
        let count = categories.len() + 1;
        let current = categories
            .iter()
            .position(|category| category.eq_ignore_ascii_case(&self.category))
            .map_or(0, |index| index + 1);
        let index = if previous {
            (current + count - 1) % count
        } else {
            (current + 1) % count
        };
        self.category = if index == 0 {
            String::new()
        } else {
            categories[index - 1].to_owned()
        };
    }
}

impl Panel {
    pub(super) fn clear_sources(&mut self) {
        self.catalogue_market_requested = None;
        self.catalogue_sources.clear();
        self.catalogue_source_next = None;
        self.catalogue_source_definition = 0;
    }
    pub fn select_item(&mut self, previous: bool) -> bool {
        if !self.selected || self.pending || self.catalogue.is_empty() {
            return false;
        }
        let ids: Vec<_> = self.catalogue.keys().copied().collect();
        let selected = self.catalogue_definition.parse::<u32>().ok();
        let index = ids
            .iter()
            .position(|id| Some(*id) == selected)
            .map_or(0, |index| {
                if previous {
                    (index + ids.len() - 1) % ids.len()
                } else {
                    (index + 1) % ids.len()
                }
            });
        if selected != Some(ids[index]) {
            self.clear_sources();
        }
        self.catalogue_definition = ids[index].to_string();
        true
    }
}

impl Panel {
    pub fn invalidate_catalogue(&mut self) {
        let had_catalogue = self.catalogue_loaded || !self.catalogue.is_empty();
        self.catalogue.clear();
        self.catalogue_query_categories.clear();
        self.catalogue_next = None;
        self.catalogue_drop_next = None;
        self.catalogue_market = None;
        self.catalogue_loaded = false;
        self.catalogue_query = None;
        self.catalogue_page = None;
        self.catalogue_categories.clear();
        self.catalogue_definition.clear();
        self.category.clear();
        self.clear_sources();
        if matches!(
            self.last_request,
            Some(
                Request::Catalogue(_)
                    | Request::CatalogueQuery(_)
                    | Request::CataloguePage(_)
                    | Request::CatalogueSources { .. }
            )
        ) {
            self.pending = false;
            self.last_request = None;
            self.reply = None;
        }
        if had_catalogue {
            self.feedback = "The catalogue scope changed; browse again".into();
        }
    }
}
