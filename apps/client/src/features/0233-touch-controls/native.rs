use super::state::Phase;
use crate::play::{PlayLink, PlayView};
use oteryn_renderer::BatchError;
use winit::event::{TouchPhase, WindowEvent};

impl PlayView {
    pub fn touch_window_event(
        &mut self,
        link: &PlayLink,
        event: &WindowEvent,
    ) -> Result<bool, BatchError> {
        match event {
            WindowEvent::Touch(touch) => {
                let phase = match touch.phase {
                    TouchPhase::Started => Phase::Started,
                    TouchPhase::Moved => Phase::Moved,
                    TouchPhase::Ended => Phase::Ended,
                    TouchPhase::Cancelled => Phase::Cancelled,
                };
                self.touch_input(link, touch.id, phase, [touch.location.x, touch.location.y])?;
                Ok(true)
            }
            WindowEvent::Focused(false) | WindowEvent::Occluded(true) => {
                self.cancel_touch_controls();
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}
