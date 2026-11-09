#[derive(Clone, Debug, Default)]
pub struct Panel {
    pub map_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub model: oteryn_client_ui::house::Panel,
    pub fields: [String; 7],
}

impl Panel {
    pub(in crate::play) fn apply(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        let acl_read = self.model.pending_acl_read && self.model.pending.as_ref() == Some(command);
        self.model.apply(command, result);
        if acl_read && let Some(snapshot) = &self.model.acl_snapshot {
            self.fields[4] = snapshot
                .members
                .iter()
                .map(|id| {
                    let mut text = String::with_capacity(36);
                    for (index, byte) in id.iter().enumerate() {
                        if [4, 6, 8, 10].contains(&index) {
                            text.push('-');
                        }
                        use std::fmt::Write;
                        let _ = write!(text, "{byte:02x}");
                    }
                    text
                })
                .collect::<Vec<_>>()
                .join("\r\n");
            self.fields[6].clear();
        }
    }
    pub(in crate::play) fn invalidate(&mut self) {
        self.model.pending = None;
        self.model.pending_acl_read = false;
        self.model.pending_acl = false;
        self.model.acl_snapshot = None;
        self.model.page = Default::default();
        self.model.index = 0;
        self.model.pending_index = 0;
        self.model.feedback = "Activity changed; reload houses and access lists".into();
        for field in [2, 4, 6] {
            self.fields[field].clear();
        }
    }
    pub(in crate::play) fn initialize(&mut self, selected: bool) {
        let world = std::mem::take(&mut self.fields[0]);
        *self = Self::default();
        self.model.selected = selected;
        self.fields[0] = world;
    }
}
