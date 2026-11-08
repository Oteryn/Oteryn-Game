use oteryn_client_runtime::spell_catalog::spellbook_details::Query;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpellbookFilter {
    pub search: String,
    pub vocation: u8,
    pub premium: u8,
    pub min_level: u32,
    pub max_level: u32,
    pub group: u8,
}
impl SpellbookFilter {
    pub fn query(&self, page: u16) -> Query {
        Query {
            page,
            search: self.search.clone(),
            vocation: self.vocation,
            premium: self.premium,
            min_level: self.min_level,
            max_level: self.max_level,
            group: self.group,
        }
    }
    pub fn valid(&self) -> bool {
        self.query(0).valid()
    }
}
