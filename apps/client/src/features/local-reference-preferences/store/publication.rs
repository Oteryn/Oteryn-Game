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
            return self.replace_in_background(previous, BackupOrder::BackupFirst, move |_| {
                Ok(preferences)
            });
        }
        self.replace_from(previous, BackupOrder::BackupFirst, move |_| Ok(preferences))
    }

    pub(in super::super) fn replace_from(
        &mut self,
        previous: &ReferencePreferences,
        order: BackupOrder,
        load: impl FnOnce(&Path) -> io::Result<ReferencePreferences>,
    ) -> io::Result<()> {
        self.preferences = Self::publish_replacement(&self.path, previous, order, load)?;
        Ok(())
    }

    pub(super) fn publish_replacement(
        path: &Path,
        previous: &ReferencePreferences,
        order: BackupOrder,
        load: impl FnOnce(&Path) -> io::Result<ReferencePreferences>,
    ) -> io::Result<ReferencePreferences> {
        let _lock = crate::atomic_file::lock(path)?;
        let replacement = load(path)?;
        Self::serialized(previous)?;
        let bytes = Self::serialized(&replacement)?;
        let backup = match Self::read_transfer(path) {
            Ok(active) => Some(Self::serialized(&active)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Some(Self::serialized(previous)?)
            }
            // Explicit recovery can replace a damaged document. Retain the prior backup.
            Err(error) if error.kind() == io::ErrorKind::InvalidData => None,
            Err(error) => return Err(error),
        };
        let rotate_backup = || {
            if let Some(backup) = &backup {
                crate::atomic_file::replace(&path.with_extension("backup.json"), backup, true)?;
            }
            Ok::<_, io::Error>(())
        };
        if order == BackupOrder::BackupFirst {
            rotate_backup()?;
        }
        crate::atomic_file::replace(path, &bytes, true)?;
        if order == BackupOrder::ActiveFirst {
            rotate_backup()?;
        }
        Ok(replacement)
    }
}

#[cfg(test)]
#[test]
fn review_3376_restore_keeps_backup_content_when_backup_write_fails() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let backup = path.with_extension("backup.json");
    let active = ReferencePreferences {
        ui_scale: 125,
        ..Default::default()
    };
    let saved = ReferencePreferences {
        ui_scale: 140,
        ..Default::default()
    };
    PreferenceStore::export_to(&active, &path)?;
    PreferenceStore::export_to(&saved, &backup)?;
    let result =
        PreferenceStore::publish_replacement(&path, &active, BackupOrder::ActiveFirst, |_| {
            let replacement = PreferenceStore::read_transfer(&backup)?;
            std::fs::remove_file(&backup)?;
            std::fs::create_dir(&backup)?;
            Ok(replacement)
        });
    assert!(result.is_err());
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, 140);
    Ok(())
}
