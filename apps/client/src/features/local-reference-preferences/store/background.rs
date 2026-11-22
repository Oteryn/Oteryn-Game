use super::*;

impl PreferenceStore {
    pub(in super::super) fn apply_in_background(
        &mut self,
        preferences: ReferencePreferences,
    ) -> io::Result<()> {
        self.apply_then(preferences, false, |_| ())
    }

    pub(in super::super) fn apply_then(
        &mut self,
        preferences: ReferencePreferences,
        recover_malformed: bool,
        complete: impl FnOnce(bool) + Send + 'static,
    ) -> io::Result<()> {
        preferences.validate().map_err(io::Error::other)?;
        let baseline = self
            .io_baseline
            .get_or_insert_with(|| {
                std::sync::Arc::new(std::sync::Mutex::new(self.preferences.clone()))
            })
            .clone();
        let edited = preferences.clone();
        let replacing = self.replacing;
        self.enqueue(move |path| {
            let result = (|| {
                let mut previous = baseline
                    .lock()
                    .map_err(|error| io::Error::other(error.to_string()))?;
                let active = Self::write_edits(&path, &previous, &edited, recover_malformed)?;
                *previous = edited;
                Ok(replacing.then_some(active))
            })();
            complete(result.is_ok());
            result
        })?;
        self.preferences = preferences;
        Ok(())
    }

    pub(in super::super) fn replace_in_background(
        &mut self,
        previous: &ReferencePreferences,
        order: BackupOrder,
        load: impl FnOnce(&Path) -> io::Result<ReferencePreferences> + Send + 'static,
    ) -> io::Result<()> {
        let previous = previous.clone();
        self.enqueue(move |path| {
            Self::publish_replacement(&path, &previous, order, load).map(Some)
        })?;
        self.replacing = true;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn restore_in_background(
        &mut self,
        previous: &ReferencePreferences,
    ) -> io::Result<()> {
        self.replace_in_background(previous, BackupOrder::ActiveFirst, |path| {
            Self::read_transfer(&path.with_extension("backup.json"))
        })
    }

    pub(in super::super) fn load_in_background(path: PathBuf, template: Option<PathBuf>) -> Self {
        let mut store = Self::defaults(path.clone());
        store.replacing = true;
        let task = bevy_tasks::IoTaskPool::get().spawn(async move {
            let result = (|| {
                let _lock = crate::atomic_file::lock(&path)?;
                match Self::read_transfer(&path) {
                    Ok(preferences) => Ok(preferences),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        let Some(template) = template else {
                            return Ok(ReferencePreferences::default());
                        };
                        let preferences = Self::read_or_default(&template)?;
                        crate::atomic_file::replace(&path, &Self::serialized(&preferences)?, true)?;
                        Ok(preferences)
                    }
                    Err(error) => Err(error),
                }
            })();
            vec![result.map(Some)]
        });
        store.save_task = Some(task);
        store.queued = 1;
        store
    }

    pub(in super::super) fn replacing(&self) -> bool {
        self.replacing
    }

    fn enqueue(
        &mut self,
        save: impl FnOnce(PathBuf) -> SaveResult + Send + 'static,
    ) -> io::Result<()> {
        const MAX_PENDING_WRITES: usize = 8;
        if self.queued >= MAX_PENDING_WRITES {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Preference writer is busy",
            ));
        }
        self.queued += 1;
        let path = self.path.clone();
        let previous = self.save_task.take();
        self.save_task = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
            let mut results = match previous {
                Some(previous) => previous.await,
                None => Vec::new(),
            };
            results.push(save(path));
            results
        }));
        Ok(())
    }

    pub(in super::super) fn saving(&self) -> bool {
        self.save_task.is_some()
    }

    pub(in super::super) fn at_path(&self, path: &Path) -> bool {
        self.path == path
    }

    pub(in super::super) fn poll_background_save(&mut self) -> Option<Vec<SaveResult>> {
        let results = bevy_tasks::futures::check_ready(self.save_task.as_mut()?)?;
        self.save_task = None;
        self.queued = 0;
        self.replacing = false;
        if let Some(baseline) = self.io_baseline.take()
            && let Ok(saved) = baseline.lock()
        {
            self.preferences = saved.clone();
        }
        if let Some(restored) = results
            .iter()
            .rev()
            .find_map(|item| item.as_ref().ok()?.as_ref())
        {
            self.preferences = restored.clone();
        }
        Some(results)
    }
}
