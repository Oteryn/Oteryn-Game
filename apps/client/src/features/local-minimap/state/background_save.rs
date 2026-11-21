use super::*;

#[cfg(bevy_shell)]
#[derive(Debug)]
pub(super) struct OutgoingSave {
    cells: BTreeMap<CellPosition, u8>,
    markers: Vec<Marker>,
    task: Option<bevy_tasks::Task<io::Result<()>>>,
}

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
        self.queue_snapshot(path, saved)?;
        self.cells_dirty = false;
        self.marker_save_queued = false;
        Ok(())
    }

    #[cfg(bevy_shell)]
    pub(super) fn queue_snapshot(&mut self, path: PathBuf, saved: Saved) -> io::Result<()> {
        let Some(CacheAccess::Owner { _lock }) = self.cache_access.get(&path) else {
            return Err(io::Error::other("Minimap storage is unavailable"));
        };
        let lock = _lock.clone();
        let previous = self.save_task.take();
        self.save_task = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
            let _lock = lock;
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            saved.validate()?;
            saved.write_to(&path)
        }));
        Ok(())
    }

    pub(super) fn save_before_rebind(&mut self) -> io::Result<()> {
        #[cfg(bevy_shell)]
        if self.background_save && let Some(path) = self.path() {
            if self.cells_dirty && !self.is_read_only() && !self.background_io_owns_cache() {
                self.start_save()?;
            }
            if let Some(task) = self.save_task.take() {
                self.outgoing_saves.insert(path, OutgoingSave {
                    cells: std::mem::take(&mut self.cells),
                    markers: std::mem::take(&mut self.markers),
                    task: Some(task),
                });
            }
            return Ok(());
        }
        self.save()
    }

    #[cfg(bevy_shell)]
    pub(super) fn load_outgoing_save(&mut self, path: &std::path::Path) -> bool {
        let Some(saved) = self.outgoing_saves.remove(path) else {
            return false;
        };
        self.cells = saved.cells;
        self.markers = saved.markers;
        self.cells_dirty = saved.task.is_none();
        self.save_task = saved.task;
        true
    }

    #[cfg(bevy_shell)]
    pub(super) fn detach_outgoing_saves(&mut self) {
        for save in std::mem::take(&mut self.outgoing_saves).into_values() {
            if let Some(task) = save.task {
                bevy_tasks::IoTaskPool::get().spawn(async move {
                    if let Err(error) = task.await {
                        eprintln!("Could not save minimap: {error}");
                    }
                }).detach();
            }
        }
    }

    #[cfg(bevy_shell)]
    pub(in crate::play) fn poll_background_save(&mut self) -> bool {
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
        let mut outgoing_error = None;
        self.outgoing_saves.retain(|_, save| {
            match save.task.as_mut().and_then(bevy_tasks::futures::check_ready) {
                Some(Ok(())) => false,
                Some(Err(error)) => {
                    save.task = None;
                    outgoing_error = Some(error);
                    true
                }
                None => true,
            }
        });
        if let Some(error) = outgoing_error {
            self.feedback = lfmt!("Could not save minimap: {error}", error = error);
        }
        let changed = self.background_save && self.poll_cache_load();
        if self.background_save {
            self.save_if_due();
        }
        changed
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

    #[cfg(bevy_shell)]
    pub(in crate::play) fn flush_background(&mut self) -> bool {
        self.poll_background_save();
        if let Some(Err(error)) = self.poll_background_import() {
            eprintln!("Minimap import failed: {error}");
        }
        if self.background_save && self.cells_dirty && !self.background_io_owns_cache() && !self.is_read_only()
            && let Err(error) = self.start_save()
        {
            self.save_failed(error);
        }
        self.save_task.is_some() || self.outgoing_saves.values().any(|save| save.task.is_some())
            || self.background_io_pending()
    }

    #[cfg(all(test, bevy_shell))]
    pub(super) fn finish_pending_save(&mut self) {
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
