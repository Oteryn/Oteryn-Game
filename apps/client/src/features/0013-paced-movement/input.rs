use crate::input::StepDir;
use crate::play::{Input, PlayView};
use oteryn_renderer::BatchError;

impl PlayView {
    /// An arrow key: the latest press wins over an arrow still waiting at the end of the queue.
    pub fn arrow(&mut self, direction: StepDir) {
        if matches!(self.inputs.back(), Some(Input::Arrow(_))) {
            self.inputs.pop_back();
        }
        self.queue(Input::Arrow(direction));
    }

    /// The step to send now: an arrow key first, else the next step toward the click goal.
    pub fn next_step(&mut self) -> Option<StepDir> {
        if self.in_flight {
            return None;
        }
        let direction = self.pending_step();
        self.in_flight = direction.is_some();
        if let Some(facing) = direction {
            self.facing = facing;
        }
        direction
    }

    pub(in crate::play) fn pending_step(&mut self) -> Option<StepDir> {
        match self.inputs.front() {
            Some(&Input::Arrow(direction)) => {
                self.inputs.pop_front();
                return Some(direction);
            }
            // A hotkey or click goes out first, in `tick`.
            Some(_) => return None,
            None => {}
        }
        self.walk.next_step(self.own)
    }

    /// [`Self::next_step`], redrawing at once when the player turns, so the new facing shows
    /// before the outcome arrives.
    pub(in crate::play) fn send_step(&mut self) -> Result<Option<StepDir>, BatchError> {
        let before = self.facing;
        let direction = self.next_step();
        if self.facing != before {
            self.scene.set_facing(self.facing)?;
        }
        Ok(direction)
    }
}
