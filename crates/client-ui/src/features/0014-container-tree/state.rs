use oteryn_client_runtime::container_tree::{
    ContainerView, ContainerViewIntent, ContainerViews, ItemEntry, ItemHandle,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContainerPanel {
    pub bulk: super::BulkMove,
    pub open: bool,
    pub house_capacity_v2: bool,
    pub views: Option<ContainerViews>,
    pub active: Option<u8>,
    pub pending: bool,
    pub feedback: Option<&'static str>,
    selected: Option<ItemHandle>,
    focus: Option<ItemHandle>,
    focus_accepted: bool,
    ended: bool,
}

impl ContainerPanel {
    pub fn apply(&mut self, views: Option<ContainerViews>) {
        if self.ended {
            return;
        }
        self.views = views;
        self.focus_requested();
        if self.current().is_none() && (self.focus.is_none() || self.focus_accepted) {
            self.active = self
                .views
                .as_ref()
                .and_then(|v| v.views.first())
                .map(|v| v.view_id);
        }
        if self.selected_item().is_none() {
            self.selected = None;
        }
    }
    fn focus_requested(&mut self) {
        if !self.focus_accepted {
            return;
        }
        if let Some(view) = self.views.as_ref().and_then(|v| {
            v.views
                .iter()
                .find(|v| Some(v.container_handle) == self.focus)
        }) {
            self.active = Some(view.view_id);
            self.focus = None;
        }
    }
    pub fn current(&self) -> Option<&ContainerView> {
        self.views
            .as_ref()?
            .views
            .iter()
            .find(|v| Some(v.view_id) == self.active)
    }
    pub fn rows(&self) -> &[ItemEntry] {
        self.current().map_or(&[], |v| &v.entries)
    }
    pub fn activate(&mut self, id: u8) -> bool {
        if !self
            .views
            .as_ref()
            .is_some_and(|v| v.views.iter().any(|v| v.view_id == id))
        {
            return false;
        }
        if self.active != Some(id) {
            self.active = Some(id);
            self.selected = None;
        }
        true
    }
    pub fn select(&mut self, handle: ItemHandle) -> bool {
        if !self.rows().iter().any(|e| e.handle == handle) {
            return false;
        }
        self.selected = Some(handle);
        true
    }
    pub fn selected_item(&self) -> Option<ItemEntry> {
        self.rows()
            .iter()
            .find(|e| Some(e.handle) == self.selected)
            .copied()
    }
    pub fn open(
        &mut self,
        handle: ItemHandle,
        replace_view: Option<u8>,
    ) -> Option<ContainerViewIntent> {
        let intent = self.request(ContainerViewIntent::Open {
            handle,
            replace_view,
        })?;
        self.focus = Some(handle);
        Some(intent)
    }
    pub fn open_selected(&mut self) -> Option<ContainerViewIntent> {
        self.open(self.selected_item()?.handle, self.active)
    }
    pub fn up(&mut self) -> Option<ContainerViewIntent> {
        self.current()?.parent_handle?;
        self.request(ContainerViewIntent::Up {
            view_id: self.active?,
        })
    }
    pub fn close(&mut self) -> Option<ContainerViewIntent> {
        self.request(ContainerViewIntent::Close {
            view_id: self.active?,
        })
    }
    fn request(&mut self, intent: ContainerViewIntent) -> Option<ContainerViewIntent> {
        if self.ended || self.views.is_none() || self.pending || self.bulk.pending {
            return None;
        }
        self.clear_focus();
        self.pending = true;
        self.feedback = None;
        Some(intent)
    }
    pub fn accept_focus(&mut self) {
        self.focus_accepted = true;
        self.focus_requested();
    }
    pub fn clear_focus(&mut self) {
        self.focus_accepted = false;
        self.focus = None;
    }
    pub fn finish(&mut self, feedback: Option<&'static str>) {
        self.pending = false;
        self.feedback = feedback;
    }
    pub fn end(&mut self) {
        *self = Self {
            ended: true,
            ..Default::default()
        };
    }
}
