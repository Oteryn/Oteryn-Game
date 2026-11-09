use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub catalogue_open: bool,
    pub catalogue_loaded: bool,
    pub catalogue_next: Option<u32>,
    pub catalogue_drop_next: Option<u16>,
    pub catalogue_categories: Vec<String>,
    pub catalogue_market: Option<u32>,
    pub catalogue: std::collections::BTreeMap<u32, InspectedItem>,
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
    item.attributes.iter().filter_map(|(key, value)| {
        (key.eq_ignore_ascii_case("category") || key == "Weapon Type" || key == "Body Position")
            .then(|| value.split(',').next().unwrap_or("").trim())
            .filter(|value| !value.is_empty())
    })
}
impl Panel {
    pub fn invalidate_catalogue(&mut self) {
        let had_catalogue = self.catalogue_loaded || !self.catalogue.is_empty();
        self.catalogue.clear();
        self.catalogue_categories.clear();
        self.catalogue_next = None;
        self.catalogue_drop_next = None;
        self.catalogue_market = None;
        self.catalogue_loaded = false;
        self.catalogue_definition.clear();
        self.category.clear();
        if matches!(
            self.last_request,
            Some(Request::Catalogue(_) | Request::CatalogueQuery(_))
        ) {
            self.last_request = None;
            self.reply = None;
            self.pending = false;
        }
        if had_catalogue {
            self.feedback = "The catalogue scope changed; browse again".into();
        }
    }
    pub fn categories(&self) -> Vec<&str> {
        if !self.catalogue_categories.is_empty() {
            return self
                .catalogue_categories
                .iter()
                .map(String::as_str)
                .collect();
        }
        self.catalogue
            .values()
            .flat_map(item_categories)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
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
