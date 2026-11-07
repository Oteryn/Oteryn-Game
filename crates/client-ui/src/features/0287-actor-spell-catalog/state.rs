use oteryn_client_runtime::spell_catalog::CatalogState;
#[derive(Debug, Clone, Default)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub catalog: CatalogState,
    pub filter: String,
    pub page: usize,
}
