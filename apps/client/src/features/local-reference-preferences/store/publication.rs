use super::*;

impl PreferenceStore {
    pub fn apply(&mut self, preferences: ReferencePreferences) -> io::Result<()> {
        #[cfg(bevy_shell)]
        if self.saving() {
            return self.apply_in_background(preferences);
        }
        Self::write_edits(&self.path, &self.preferences, &preferences)?;
        self.preferences = preferences;
        Ok(())
    }

    pub(super) fn write_edits(
        path: &Path,
        before: &ReferencePreferences,
        after: &ReferencePreferences,
    ) -> io::Result<ReferencePreferences> {
        // Validate even unchanged snapshots, including the supported transfer size.
        Self::serialized(after)?;
        let _lock = crate::atomic_file::lock(path)?;
        let mut active = Self::read_or_default(path)?;
        active.rebase_edits(before, after)?;
        crate::atomic_file::replace(path, &Self::serialized(&active)?, true)?;
        Ok(active)
    }

    pub(super) fn read_or_default(path: &Path) -> io::Result<ReferencePreferences> {
        match Self::read_transfer(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(ReferencePreferences::default())
            }
            result => result,
        }
    }

    pub fn replace_with_backup(
        &mut self,
        preferences: ReferencePreferences,
        previous: &ReferencePreferences,
    ) -> io::Result<()> {
        #[cfg(bevy_shell)]
        if self.saving() {
            return self.replace_in_background(previous, move |_| Ok(preferences));
        }
        self.replace_from(previous, move |_| Ok(preferences))
    }

    pub(in super::super) fn replace_from(
        &mut self,
        previous: &ReferencePreferences,
        load: impl FnOnce(&Path) -> io::Result<ReferencePreferences>,
    ) -> io::Result<()> {
        self.preferences = Self::publish_replacement(&self.path, previous, load)?;
        Ok(())
    }

    pub(super) fn publish_replacement(
        path: &Path,
        previous: &ReferencePreferences,
        load: impl FnOnce(&Path) -> io::Result<ReferencePreferences>,
    ) -> io::Result<ReferencePreferences> {
        let _lock = crate::atomic_file::lock(path)?;
        let replacement = load(path)?;
        Self::serialized(previous)?;
        let bytes = Self::serialized(&replacement)?;
        match Self::read_transfer(path) {
            Ok(active) => Self::write_backup(path, &active)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Self::write_backup(path, previous)?
            }
            // Explicit recovery can replace a damaged document. Retain the prior backup.
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {}
            Err(error) => return Err(error),
        }
        crate::atomic_file::replace(path, &bytes, true)?;
        Ok(replacement)
    }

    fn write_backup(path: &Path, active: &ReferencePreferences) -> io::Result<()> {
        crate::atomic_file::replace(
            &path.with_extension("backup.json"),
            &Self::serialized(active)?,
            true,
        )
    }
}
