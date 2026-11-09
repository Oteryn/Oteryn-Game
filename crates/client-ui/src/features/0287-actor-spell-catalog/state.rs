use oteryn_client_runtime::spell_catalog::CatalogState;
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub archive_open: bool,
    pub catalog: CatalogState,
    pub filter: String,
    pub page: usize,
    pub selection: usize,
    pub max_level: Option<u32>,
    pub group: Option<u32>,
    pub vocation: Option<String>,
    pub learned_only: bool,
    pub filter_error: bool,
}

impl Panel {
    #[must_use]
    pub fn selected_spell(
        &self,
    ) -> Option<&oteryn_client_runtime::spell_catalog::SpellCatalogEntry> {
        if !self.selected || self.filter_error {
            return None;
        }
        self.catalog
            .entries()
            .iter()
            .filter(|entry| self.matches(entry))
            .nth(self.selection)
    }

    pub(super) fn matches(
        &self,
        entry: &oteryn_client_runtime::spell_catalog::SpellCatalogEntry,
    ) -> bool {
        let search = self.filter.to_lowercase();
        (entry.name.to_lowercase().contains(&search)
            || entry.words.to_lowercase().contains(&search))
            && self.max_level.is_none_or(|level| entry.level <= level)
            && self
                .group
                .is_none_or(|group| entry.cooldown_groups.contains(&group))
            && (!self.learned_only || entry.learned)
            && self
                .vocation
                .as_ref()
                .is_none_or(|vocation| super::professions(entry).contains(&vocation.as_str()))
    }
}
