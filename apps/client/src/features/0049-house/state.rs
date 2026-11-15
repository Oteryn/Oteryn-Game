#[path = "state/ownership.rs"]
pub mod ownership;
#[derive(Clone, Debug, Default)]
pub struct Panel {
    pub(in crate::play) map_open: bool,
    pub map_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub model: oteryn_client_ui::house::Panel,
    pub fields: [String; 7],
    pub ownership: ownership::Ownership,
}

impl Panel {
    pub(in crate::play) fn apply(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        if command.kind == oteryn_client_runtime::house::house_sale::COMMAND {
            self.ownership.apply(command, result);
            return;
        }
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
        self.map_running
            .store(false, std::sync::atomic::Ordering::Release);
        self.map_running = Default::default();
        self.map_open = false;
        self.ownership.form_review = None;
        self.ownership
            .model
            .admitted(if self.model.selected { &[49] } else { &[] });
        self.ownership.navigation = None;
        self.ownership.fields = Default::default();
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
        let context = self.ownership.context.clone();
        *self = Self::default();
        self.model.selected = selected;
        self.ownership.context = context;
        self.ownership
            .model
            .admitted(if selected { &[49] } else { &[] });
        self.fields[0] = world;
    }
}

#[cfg(test)]
#[path = "tests/e2e_n01.rs"]
mod e2e_n01;

#[cfg(test)]
mod map_lifecycle_tests {
    use super::*;
    #[test]
    fn house_activity_invalidation_closes_the_existing_map_generation() {
        use std::sync::atomic::Ordering;
        let mut panel = Panel::default();
        let shown = panel.map_running.clone();
        shown.store(true, Ordering::Release);
        panel.invalidate();
        assert!(!shown.load(Ordering::Acquire));
        assert!(!std::sync::Arc::ptr_eq(&shown, &panel.map_running));
    }
}

#[cfg(any(windows, test, feature = "bevy-shell"))]
pub(in crate::play) fn map_projection(
    mailbox: &crate::play::mailbox::Mailbox,
) -> Vec<oteryn_client_ui::house::MapHouse> {
    oteryn_client_ui::house::map_houses(&mailbox.properties.house.model)
}
#[cfg(test)]
#[test]
fn open_house_map_projection_reads_changed_selection_and_page() {
    let mut mailbox = crate::play::mailbox::Mailbox::default();
    let row = |name: &str, x| oteryn_client_runtime::house::Row {
        house: name.into(),
        name: name.into(),
        entrance: Some((x, 100, -7)),
        ..Default::default()
    };
    mailbox.properties.house.model.page.rows = vec![row("A", 100), row("B", 200)];
    assert!(map_projection(&mailbox)[0].selected);
    let (link, _commands, _outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    {
        let mut shared = crate::play::mailbox::lock(&link.mailbox);
        shared.properties.house = mailbox.properties.house.clone();
        shared.properties.house.model.selected = true;
    }
    link.house_edit(2, "2".into());
    link.house_request(10);
    assert_eq!(link.house_panel().model.index, 1);
    assert!(map_projection(&crate::play::mailbox::lock(&link.mailbox))[1].selected);
    mailbox.properties.house.model.index = 1;
    assert!(map_projection(&mailbox)[1].selected);
    mailbox.properties.house.model.page.rows = vec![row("C", 300)];
    mailbox.properties.house.model.index = 0;
    let fresh = map_projection(&mailbox);
    assert_eq!(fresh.len(), 1);
    assert!(fresh[0].label.contains("C"));
    assert_eq!(fresh[0].position.0, 300);
}
