use super::*;

impl PreferenceStore {
    pub(in super::super) fn apply_in_background(&mut self, preferences: ReferencePreferences) {
        let previous = std::mem::replace(&mut self.preferences, preferences.clone());
        if self.replacing {
            self.enqueue(move |path| {
                let mut current = Self::load(path.clone())?.preferences().clone();
                current.rebase_edits(&previous, &preferences)?;
                Self::write_existing(&path, &current)?;
                Ok(Some(current))
            });
        } else {
            self.enqueue(move |path| Self::write_existing(&path, &preferences).map(|()| None));
        }
    }

    pub(in super::super) fn replace_in_background(
        &mut self,
        previous: &ReferencePreferences,
        load: impl FnOnce(&Path) -> io::Result<ReferencePreferences> + Send + 'static,
    ) {
        let previous = previous.clone();
        self.replacing = true;
        self.enqueue(move |path| {
            let mut replacement = load(&path)?;
            let active = match Self::read_transfer(&path) {
                Ok(active) => active,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::InvalidData
                    ) =>
                {
                    previous
                }
                Err(error) => return Err(error),
            };
            replacement.miscellaneous = active.miscellaneous;
            Self::write_with_backup(
                &path,
                &Self::serialized(&replacement)?,
                &Self::serialized(&active)?,
            )?;
            Ok(Some(replacement))
        });
    }

    pub(in super::super) fn restore_in_background(
        &mut self,
        previous: &ReferencePreferences,
    ) -> io::Result<()> {
        self.replace_in_background(previous, |path| {
            Self::read_transfer(&path.with_extension("backup.json"))
        });
        Ok(())
    }

    pub(in super::super) fn load_in_background(path: PathBuf, template: Option<PathBuf>) -> Self {
        let mut store = Self::defaults(path);
        store.replacing = true;
        store.enqueue(move |path| match Self::read_transfer(&path) {
            Ok(preferences) => Ok(Some(preferences)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let preferences = match template {
                    Some(template) => {
                        let preferences = Self::load(template)?.preferences().clone();
                        Self::write_existing(&path, &preferences)?;
                        preferences
                    }
                    None => ReferencePreferences::default(),
                };
                Ok(Some(preferences))
            }
            Err(error) => Err(error),
        });
        store
    }

    pub(in super::super) fn replacing(&self) -> bool {
        self.replacing
    }

    fn enqueue(&mut self, save: impl FnOnce(PathBuf) -> SaveResult + Send + 'static) {
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
        self.replacing = false;
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
