use super::*;

impl Minimap {
    #[cfg(bevy_shell)]
    pub(in crate::play) fn enable_background_save(&mut self) {
        self.background_save = true;
    }

    #[cfg(bevy_shell)]
    pub(super) fn start_save(&mut self) -> io::Result<()> {
        let Some(path) = self.path() else {
            return self.save();
        };
        if self.is_read_only() {
            return self.save();
        }
        self.require_cache_owner(&path)?;
        if self.load_failed {
            return Err(io::Error::other(tr!("Invalid minimap file")));
        }
        let saved = self.snapshot(self.markers.clone());
        self.save_task = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
            saved.validate()?;
            saved.write_to(&path)
        }));
        self.cells_dirty = false;
        self.marker_save_queued = false;
        Ok(())
    }

    #[cfg(bevy_shell)]
    pub(in crate::play) fn poll_background_save(&mut self) {
        if let Some(result) = self
            .save_task
            .as_mut()
            .and_then(bevy_tasks::futures::check_ready)
        {
            self.save_task = None;
            self.finish_background_save(result);
            if self.marker_save_queued {
                self.start_marker_save();
            }
        }
        if self.background_save {
            self.save_if_due();
        }
    }

    #[cfg(bevy_shell)]
    fn finish_background_save(&mut self, result: io::Result<()>) {
        match result {
            Ok(()) => {
                self.saved_at = Instant::now();
                self.retry_delay = Duration::from_secs(5);
            }
            Err(error) => self.save_failed(error),
        }
    }

    // Explicit saves, cache switches and shutdown serialize with the in-flight snapshot.
    pub(super) fn finish_pending_save(&mut self) {
        #[cfg(bevy_shell)]
        if let Some(task) = self.save_task.take() {
            self.finish_background_save(bevy_tasks::block_on(task));
        }
    }

    #[cfg(bevy_shell)]
    pub(super) fn start_marker_save(&mut self) {
        if self.save_task.is_none()
            && let Err(error) = self.start_save()
        {
            self.marker_save_queued = false;
            self.save_failed(error);
        }
    }
}
