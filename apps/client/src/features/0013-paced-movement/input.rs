use crate::input::StepDir;
use crate::play::{Input, PlayView};
use oteryn_renderer::{BatchError, TileCoord};

impl PlayView {
    fn own_forecast(&self) -> Option<&oteryn_session::WorldSpatialEntity> {
        self.appearances
            .iter()
            .find(|entry| {
                Some(entry.entity.entity.identity) == self.scene.own_identity()
                    && entry.entity.position.x == self.own.x
                    && entry.entity.position.y == self.own.y
                    && entry.entity.position.floor == self.floor
            })
            .map(|entry| &entry.entity)
    }

    fn begin_prewalk(&mut self, direction: StepDir) -> bool {
        let Some(([x, y], ms)) = self
            .own_forecast()
            .and_then(|entity| oteryn_client_ui::actor_motion::forecast(entity, direction))
        else {
            return false;
        };
        let from = self.own;
        let origin = oteryn_session::ActorPosition {
            x: from.x,
            y: from.y,
            floor: self.floor,
        };
        let destination = TileCoord::new(x, y);
        self.place(oteryn_session::ActorPosition {
            x,
            y,
            floor: self.floor,
        });
        self.prewalk = Some((origin, destination));
        self.walk = oteryn_client_simulation::walk::Glide::new(
            from,
            destination,
            std::time::Duration::from_millis(u64::from(ms)),
        )
        .map(|glide| (glide, std::time::Instant::now()));
        true
    }

    /// An arrow key: the latest press wins over an arrow still waiting at the end of the queue.
    pub fn arrow(&mut self, direction: StepDir) {
        if matches!(self.inputs.back(), Some(Input::Arrow(_))) {
            self.inputs.pop_back();
        }
        self.queue(Input::Arrow(direction));
    }

    /// The arrow-key step to send now. Ground clicks use server target walking.
    pub fn next_step(&mut self) -> Option<StepDir> {
        if self.in_flight
            || (self.appearances.iter().any(|entry| {
                Some(entry.entity.entity.identity) == self.scene.own_identity()
                    && entry.entity.motion.is_some()
            }) && self
                .walk
                .is_some_and(|(glide, start)| !glide.finished(start.elapsed())))
        {
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
        None
    }

    /// [`Self::next_step`], redrawing at once when the player turns, so the new facing shows
    /// before the outcome arrives.
    pub(in crate::play) fn send_step(&mut self) -> Result<Option<StepDir>, BatchError> {
        let before = self.facing;
        let direction = self.next_step();
        let predicted = direction.is_some_and(|direction| self.begin_prewalk(direction));
        if self.facing != before || predicted {
            self.rebuild()?;
        }
        Ok(direction)
    }
}
