//! Capability selection is retained independently of the inventory snapshot.
impl crate::play::Outbox {
    pub(in crate::play) fn equipment_initialize(&self, selected: bool) {
        crate::play::mailbox::lock(&self.0).equipment_selected = selected;
    }
}
