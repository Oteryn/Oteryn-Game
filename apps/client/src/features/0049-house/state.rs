#[derive(Clone, Debug, Default)]
pub struct Panel {
    pub map_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub model: oteryn_client_ui::house::Panel,
    pub fields: [String; 7],
}

impl Panel {
    pub(in crate::play) fn initialize(&mut self, selected: bool) {
        let world = std::mem::take(&mut self.fields[0]);
        *self = Self::default();
        self.model.selected = selected;
        self.fields[0] = world;
    }
}
