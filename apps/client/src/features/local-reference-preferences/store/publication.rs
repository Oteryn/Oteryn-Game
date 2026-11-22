use super::*;

impl PreferenceStore {
    pub fn apply(&mut self, preferences: ReferencePreferences) -> io::Result<()> {
        #[cfg(bevy_shell)]
        if self.saving() {
            return self.apply_then(preferences, true, |_| ());
        }
        Self::write_edits(&self.path, &self.preferences, &preferences, true)?;
        self.preferences = preferences;
        Ok(())
    }

    pub(super) fn write_edits(
        path: &Path,
        before: &ReferencePreferences,
        after: &ReferencePreferences,
        recover_malformed: bool,
    ) -> io::Result<ReferencePreferences> {
        // Validate even unchanged snapshots, including the supported transfer size.
        Self::serialized(after)?;
        let _lock = crate::atomic_file::lock(path)?;
        Self::finish_pending_backup(path)?;
        let mut active = match Self::read_or_default(path) {
            Err(error) if recover_malformed && error.kind() == io::ErrorKind::InvalidData => {
                crate::atomic_file::preserve_malformed(path)?;
                ReferencePreferences::default()
            }
            result => result?,
        };
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
        Self::finish_pending_backup(path)?;
        let replacement = load(path)?;
        Self::serialized(previous)?;
        let bytes = Self::serialized(&replacement)?;
        let backup = match Self::transfer_bytes(path).and_then(|bytes| {
            Self::decode_transfer(&bytes)?;
            Ok(bytes)
        }) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Some(Self::serialized(previous)?)
            }
            // Explicit recovery can replace a damaged document. Retain the prior backup.
            Err(error) if error.kind() == io::ErrorKind::InvalidData => None,
            Err(error) => return Err(error),
        };
        let slot = path.with_extension("backup.json");
        if let Some(backup) = backup {
            match order {
                BackupOrder::BackupFirst => crate::atomic_file::replace(&slot, &backup, true)?,
                BackupOrder::ActiveFirst => {
                    let pending = path.with_added_extension("pending-backup");
                    crate::atomic_file::replace(&pending, &backup, true)?;
                    sync_parent(path)?;
                    crate::atomic_file::replace(path, &bytes, true)?;
                    sync_parent(path)?;
                    #[cfg(test)]
                    check_backup_write()?;
                    std::fs::rename(pending, slot)?;
                    return Ok(replacement);
                }
            }
            sync_parent(path)?;
        }
        crate::atomic_file::replace(path, &bytes, true)?;
        Ok(replacement)
    }

    // Resolve an interrupted swap under the same lock as every subsequent write.
    fn finish_pending_backup(path: &Path) -> io::Result<()> {
        let pending = path.with_added_extension("pending-backup");
        let backup = match Self::transfer_bytes(&pending) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            result => result?,
        };
        match Self::transfer_bytes(path) {
            // Publication stopped before replacing active. Keep the original backup slot.
            Ok(active) if active == backup => std::fs::remove_file(pending)?,
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => std::fs::rename(pending, path.with_extension("backup.json"))?,
        }
        sync_parent(path)
    }
}

fn sync_parent(_path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let directory = _path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::File::open(directory)?.sync_all()?;
    }
    Ok(())
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

#[cfg(test)]
thread_local! {
    static FAIL_BACKUP_WRITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn check_backup_write() -> io::Result<()> {
    if FAIL_BACKUP_WRITE.replace(false) {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn restore_recovers_original_bytes_after_backup_write_failure() -> io::Result<()> {
    use super::super::SettingsState;
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let active = ReferencePreferences {
        ui_scale: 125,
        ..Default::default()
    };
    let original = serde_json::to_vec(&active)?;
    std::fs::write(&path, &original)?;
    PreferenceStore::export_to(
        &ReferencePreferences::default(),
        &path.with_extension("backup.json"),
    )?;
    let mut state = SettingsState::new(Some(path.clone()));
    FAIL_BACKUP_WRITE.set(true);
    assert_eq!(
        state.restore_backup().err().map(|error| error.kind()),
        Some(io::ErrorKind::PermissionDenied)
    );
    let store = PreferenceStore::load(path.clone())?;
    assert!(
        std::fs::read(store.backup_path())? == original,
        "missing pre-restore bytes"
    );
    let mut restarted = SettingsState::new(Some(path));
    restarted.restore_backup()?;
    assert_eq!(restarted.preferences.ui_scale, 125);
    Ok(())
}

#[cfg(test)]
#[test]
fn c2d_apply_recovers_damaged_preferences() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    std::fs::write(&path, b"{")?;
    let mut state = super::super::SettingsState::new(Some(path.clone()));
    let mut edited = state.preferences.clone();
    edited.ui_scale = 125;
    state.apply(edited);
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, 125);
    assert_eq!(std::fs::read(path.with_added_extension("malformed"))?, b"{");
    Ok(())
}
