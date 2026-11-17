//! Drag state retains the displayed identity until release and never predicts server state.
use oteryn_client_runtime::inventory::{ItemHandle, ItemMoveDestination, MoveRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragItem {
    pub equipped: bool,
    pub handle: ItemHandle,
    pub count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Movement {
    pub dragged: Option<DragItem>,
    pub prompt: Option<(DragItem, ItemMoveDestination)>,
    pub pending: bool,
    pub feedback: Option<&'static str>,
}
impl Movement {
    pub fn begin(&mut self, item: DragItem) -> bool {
        if self.pending || self.prompt.is_some() {
            return false;
        }
        self.dragged = Some(item);
        true
    }
    pub fn drop_on(
        &mut self,
        destination: Option<ItemMoveDestination>,
        counted: bool,
    ) -> Option<MoveRequest> {
        let item = self.dragged.take()?;
        self.feedback = None;
        let Some(destination) = destination else {
            self.feedback = Some(tr!(
                "Move cancelled: choose inventory, an equipment slot, an open container or map tile"
            ));
            return None;
        };
        if item.count > 1
            && counted
            && !item.equipped
            && !matches!(destination, ItemMoveDestination::Equipment(_))
        {
            self.prompt = Some((item, destination));
            return None;
        }
        self.pending = true;
        Some(MoveRequest {
            source: item.handle,
            destination,
            count: None,
        })
    }
    pub fn confirm(&mut self, count: u32) -> Option<MoveRequest> {
        let (item, destination) = self.prompt?;
        if count == 0 || count > item.count || count > 100 {
            self.feedback = Some(tr!("Enter a count from 1 to the stack size (maximum 100)"));
            return None;
        }
        self.prompt = None;
        self.pending = true;
        Some(MoveRequest {
            source: item.handle,
            destination,
            count: Some(count),
        })
    }
    pub fn abort(&mut self) {
        self.dragged = None;
    }
    pub fn cancel(&mut self) {
        self.dragged = None;
        self.prompt = None;
        self.feedback = Some(tr!("Move cancelled"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_requires_valid_count_and_retains_identity() -> Result<(), Box<dyn std::error::Error>> {
        let mut state = Movement::default();
        let item = DragItem {
            equipped: false,
            handle: 9.try_into()?,
            count: 20,
        };
        assert!(state.begin(item));
        assert!(
            state
                .drop_on(Some(ItemMoveDestination::MainBackpack), true)
                .is_none()
        );
        assert!(state.confirm(0).is_none());
        assert!(state.confirm(21).is_none());
        let request = state.confirm(7).ok_or("count")?;
        assert_eq!((request.source, request.count), (item.handle, Some(7)));
        assert!(state.pending);
        assert!(!state.begin(item));
        Ok(())
    }
    #[test]
    fn prompted_count_equal_to_observed_stack_stays_explicit()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut state = Movement::default();
        state.begin(DragItem {
            equipped: false,
            handle: 2.try_into()?,
            count: 10,
        });
        state.drop_on(Some(ItemMoveDestination::MainBackpack), true);
        assert_eq!(state.confirm(10).ok_or("move")?.count, Some(10));
        Ok(())
    }
    #[test]
    fn short_click_aborts_silently_and_preserves_feedback() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state = Movement {
            feedback: Some("Too far away"),
            ..Default::default()
        };
        assert!(state.begin(DragItem {
            equipped: false,
            handle: 2.try_into()?,
            count: 10
        }));
        state.abort();
        assert_eq!(
            state,
            Movement {
                feedback: Some("Too far away"),
                ..Default::default()
            }
        );
        state.cancel();
        assert_eq!(state.feedback, Some("Move cancelled"));
        Ok(())
    }
    #[test]
    fn whole_stack_fallback_and_cancel_do_not_send_counts() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state = Movement::default();
        let item = DragItem {
            equipped: false,
            handle: 2.try_into()?,
            count: 10,
        };
        state.begin(item);
        assert!(state.drop_on(None, true).is_none());
        state.begin(item);
        assert_eq!(
            state
                .drop_on(Some(ItemMoveDestination::MainBackpack), false)
                .ok_or("move")?
                .count,
            None
        );
        state.pending = false;
        state.begin(item);
        state.drop_on(Some(ItemMoveDestination::MainBackpack), true);
        state.cancel();
        assert!(state.confirm(1).is_none());
        Ok(())
    }
}
