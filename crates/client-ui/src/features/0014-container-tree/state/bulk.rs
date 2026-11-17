use super::ContainerPanel;
use oteryn_client_runtime::inventory::{
    ItemMoveDestination, ItemMoveOutcome, MoveRequest, MoveResponse,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BulkMove {
    pub pending: bool,
    pub completed: usize,
    pub total: usize,
    pub feedback: Option<&'static str>,
}
impl ContainerPanel {
    /// Snapshot only direct entries. The server revalidates custody for every move.
    pub fn move_all(&mut self, destination_view: u8) -> Option<Vec<MoveRequest>> {
        if self.pending || self.bulk.pending || self.views.is_none() {
            return None;
        }
        let source = self.current()?;
        let destination = self
            .views
            .as_ref()?
            .views
            .iter()
            .find(|v| v.view_id == destination_view)?;
        if source.container_handle == destination.container_handle || destination.browse.is_some() {
            return None;
        }
        // Never try to move the destination into itself.
        if source
            .entries
            .iter()
            .any(|item| item.handle == destination.container_handle)
        {
            return None;
        }
        let requests: Vec<_> = source
            .entries
            .iter()
            .map(|item| MoveRequest {
                source: item.handle,
                destination: ItemMoveDestination::Container(destination.container_handle),
                count: None,
            })
            .collect();
        if requests.is_empty() {
            return None;
        }
        self.bulk = BulkMove {
            pending: true,
            completed: 0,
            total: requests.len(),
            feedback: None,
        };
        Some(requests)
    }
    pub fn bulk_result(&mut self, response: Option<MoveResponse>) -> bool {
        if !self.bulk.pending {
            return false;
        }
        self.bulk.feedback = Some(oteryn_client_runtime::inventory::feedback(response));
        if response.is_some_and(|r| r.outcome == ItemMoveOutcome::Moved) {
            self.bulk.completed += 1;
            self.bulk.pending = self.bulk.completed < self.bulk.total;
        } else {
            self.bulk.pending = false;
        }
        self.bulk.pending
    }
}
