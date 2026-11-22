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
                before.clone()
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
        let source = Replacement::Set(Box::new(preferences));
        #[cfg(bevy_shell)]
        if self.saving() {
            return self.replace_in_background(previous, source);
        }
        self.replace_from(previous, source)
    }

    pub(in super::super) fn replace_from(
        &mut self,
        previous: &ReferencePreferences,
        source: Replacement,
    ) -> io::Result<()> {
        self.preferences = Self::publish_replacement(&self.path, previous, source)?;
        Ok(())
    }

    pub(super) fn publish_replacement(
        path: &Path,
        previous: &ReferencePreferences,
        source: Replacement,
    ) -> io::Result<ReferencePreferences> {
        let _lock = crate::atomic_file::lock(path)?;
        let retained = Self::finish_pending_backup(path)?;
        let ready = path.with_added_extension("ready-backup");
        let slot = path.with_extension("backup.json");
        let snapshots = Self::ordered_snapshots(path)?;
        let restoring = matches!(source, Replacement::Restore);
        let (replacement, bytes) = match source {
            Replacement::Restore => {
                let mut selected = None;
                for (_, candidate) in &snapshots {
                    if let Ok(bytes) = Self::transfer_bytes(candidate)
                        && let Ok(preferences) = Self::decode_transfer(&bytes)
                    {
                        #[cfg(test)]
                        if let Some(change) = AFTER_SELECTION.take() {
                            change(candidate)?;
                        }
                        // Settings and the active file use the same validated snapshot.
                        selected = Some((preferences, bytes));
                        break;
                    }
                }
                selected.ok_or_else(|| io::Error::other("No valid preference backup"))?
            }
            Replacement::Import(path) => {
                let replacement = Self::read_transfer(&path)?;
                let bytes = Self::serialized(&replacement)?;
                (replacement, bytes)
            }
            Replacement::Set(replacement) => {
                let bytes = Self::serialized(&replacement)?;
                (*replacement, bytes)
            }
        };
        Self::serialized(previous)?;
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
        let staging = path.with_added_extension("pending-backup");
        let mut staged = false;
        if let Some(backup) = backup.filter(|backup| *backup != bytes) {
            if !restoring && !retained && slot.is_dir() {
                return Err(io::ErrorKind::IsADirectory.into());
            }
            // Encode a logical generation in the file time. Renames preserve it and
            // legacy snapshots already carry their publication time. Advance by two
            // seconds to remain distinct even on FAT with a coarse timestamp.
            let generation = snapshots
                .first()
                .map(|(latest, _)| {
                    latest
                        .checked_add(std::time::Duration::from_secs(2))
                        .ok_or_else(|| io::Error::other("Backup generation exhausted"))
                })
                .transpose()?
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .max(std::time::SystemTime::now());
            crate::atomic_file::publish_durable_at(&staging, &backup, Some(generation))?;
            staged = true;
        }
        if let Err(error) = crate::atomic_file::publish_durable(path, &bytes)
            && !Self::transfer_bytes(path).is_ok_and(|active| active == bytes)
        {
            if staged {
                let _ = std::fs::remove_file(&staging);
            }
            return Err(error);
        }
        if staged {
            // The active rename commits the replacement. A failed promotion leaves
            // the durable rollback snapshot in staging or ready for the next writer.
            let destination = if retained && !restoring {
                path.with_added_extension("recovery")
            } else {
                ready.clone()
            };
            if crate::atomic_file::durable_rename(&staging, &destination).is_ok() && !retained {
                #[cfg(test)]
                if check_backup_write().is_err() {
                    return Ok(replacement);
                }
                let _ = crate::atomic_file::durable_rename(&ready, &slot);
            }
        }
        Ok(replacement)
    }

    // Stable sorting breaks equal legacy timestamps in publication order.
    pub(super) fn ordered_snapshots(
        path: &Path,
    ) -> io::Result<Vec<(std::time::SystemTime, PathBuf)>> {
        let mut snapshots = Vec::new();
        for candidate in [
            path.with_added_extension("recovery"),
            path.with_added_extension("ready-backup"),
            path.with_extension("backup.json"),
        ] {
            match std::fs::metadata(&candidate) {
                Ok(metadata) if metadata.is_file() => {
                    snapshots.push((metadata.modified()?, candidate))
                }
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
        }
        snapshots.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
        Ok(snapshots)
    }

    // Resolve an interrupted swap under the same lock as every subsequent write.
    fn finish_pending_backup(path: &Path) -> io::Result<bool> {
        let pending = path.with_added_extension("pending-backup");
        let ready = path.with_added_extension("ready-backup");
        if pending.try_exists()? {
            if Self::pending_backup_is_committed(path, &pending)? {
                crate::atomic_file::durable_rename(&pending, &ready)?;
            } else {
                std::fs::remove_file(pending)?;
                sync_parent(path)?;
            }
        }
        if !ready.is_file() {
            return ready.try_exists();
        }
        // Never promote an invalid or superseded ready file over a good slot.
        let slot = path.with_extension("backup.json");
        if Self::read_transfer(&ready).is_err()
            || (slot.is_file()
                && slot.metadata()?.modified()? > ready.metadata()?.modified()?
                && Self::read_transfer(&slot).is_ok())
        {
            return Ok(true);
        }
        Ok(crate::atomic_file::durable_rename(&ready, &slot).is_err())
    }

    pub(super) fn pending_backup_is_committed(path: &Path, pending: &Path) -> io::Result<bool> {
        let backup = Self::transfer_bytes(pending)?;
        match Self::transfer_bytes(path) {
            Ok(active) => Ok(active != backup),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
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
type SnapshotChange = fn(&Path) -> io::Result<()>;

#[cfg(test)]
thread_local! {
    static AFTER_SELECTION: std::cell::Cell<Option<SnapshotChange>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
#[test]
fn restore_displays_the_snapshot_it_publishes() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let slot = path.with_extension("backup.json");
    let selected = ReferencePreferences {
        ui_scale: 125,
        ..Default::default()
    };
    PreferenceStore::export_to(&selected, &slot)?;
    let bytes = std::fs::read(&slot)?;
    let mut state = super::super::SettingsState::new(Some(path.clone()));
    AFTER_SELECTION.set(Some(|candidate| {
        std::fs::write(
            candidate,
            PreferenceStore::serialized(&ReferencePreferences {
                ui_scale: 140,
                ..Default::default()
            })?,
        )
    }));
    state.restore_backup()?;
    assert!(AFTER_SELECTION.take().is_none());
    assert_eq!(std::fs::read(&path)?, bytes);
    assert_eq!(state.preferences.ui_scale, selected.ui_scale);
    Ok(())
}

#[cfg(test)]
#[test]
fn restore_keeps_the_selected_snapshot_when_the_backup_slot_becomes_unwritable() -> io::Result<()> {
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
    AFTER_SELECTION.set(Some(|backup| {
        std::fs::remove_file(backup)?;
        std::fs::create_dir(backup)
    }));
    let restored = PreferenceStore::publish_replacement(&path, &active, Replacement::Restore)?;
    assert_eq!(restored.ui_scale, 140);
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, 140);
    let retained = PreferenceStore::load(path)?.backup_path();
    assert_eq!(PreferenceStore::read_transfer(&retained)?.ui_scale, 125);
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
    state.restore_backup()?;
    assert_eq!(state.preferences.ui_scale, ReferencePreferences::default().ui_scale);
    assert_eq!(std::fs::read(&path)?, PreferenceStore::serialized(&state.preferences)?);
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
fn restore_rejects_precommit_rename_failures_without_changing_either_snapshot() -> io::Result<()> {
    use super::super::SettingsState;
    for extension in ["json.pending-backup", "json"] {
        let root = tempfile::tempdir()?;
        let path = root.path().join("preferences.json");
        let active = ReferencePreferences {
            ui_scale: 125,
            ..Default::default()
        };
        PreferenceStore::export_to(&active, &path)?;
        let backup = path.with_extension("backup.json");
        PreferenceStore::export_to(&ReferencePreferences::default(), &backup)?;
        let before = std::fs::read(&path)?;
        let saved = std::fs::read(&backup)?;
        let mut state = SettingsState::new(Some(path.clone()));
        crate::atomic_file::fail_durable_rename_to(&path.with_extension(extension));
        assert_eq!(
            state.restore_backup().err().map(|e| e.kind()),
            Some(io::ErrorKind::PermissionDenied),
            "{extension}"
        );
        assert_eq!(std::fs::read(&path)?, before);
        assert_eq!(std::fs::read(&backup)?, saved);
    }
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

#[cfg(test)]
#[test]
fn pending_promotion_allows_later_settings_writes() -> io::Result<()> {
    use super::super::SettingsState;
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let active = ReferencePreferences {
        ui_scale: 125,
        ..Default::default()
    };
    let original = serde_json::to_vec(&active)?;
    std::fs::write(&path, &original)?;
    let slot = path.with_extension("backup.json");
    PreferenceStore::export_to(&ReferencePreferences::default(), &slot)?;
    let mut state = SettingsState::new(Some(path.clone()));
    FAIL_BACKUP_WRITE.set(true);
    state.restore_backup()?;
    std::fs::remove_file(&slot)?;
    std::fs::create_dir(&slot)?;
    let edited = ReferencePreferences {
        ui_scale: 140,
        ..Default::default()
    };
    state.apply(edited.clone());
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, 140);
    let import = root.path().join("import.json");
    PreferenceStore::export_to(&active, &import)?;
    let recovery = path.with_added_extension("recovery");
    let before_import = std::fs::read(&path)?;
    state.import_from(&import)?;
    assert_eq!(std::fs::read(&recovery)?, before_import);
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, 125);
    let before_reset = std::fs::read(&path)?;
    state.reset_defaults()?;
    assert_eq!(std::fs::read(&recovery)?, before_reset);
    assert_eq!(
        PreferenceStore::read_transfer(&path)?.ui_scale,
        ReferencePreferences::default().ui_scale
    );
    assert_eq!(
        std::fs::read(path.with_added_extension("ready-backup"))?,
        original
    );
    let before_restore = std::fs::read(&path)?;
    state.restore_backup()?;
    assert_eq!(std::fs::read(&path)?, before_reset);
    assert_eq!(
        std::fs::read(PreferenceStore::load(path)?.backup_path())?,
        before_restore
    );
    Ok(())
}

#[cfg(test)]
#[test]
fn apply_after_corruption_preserves_loaded_fields() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let loaded = ReferencePreferences {
        ui_scale: 125,
        ..Default::default()
    };
    PreferenceStore::export_to(&loaded, &path)?;
    let mut state = super::super::SettingsState::new(Some(path.clone()));
    std::fs::write(&path, b"{")?;
    let mut edited = state.preferences.clone();
    edited.hud_mana = false;
    state.apply(edited);
    let saved = PreferenceStore::read_transfer(&path)?;
    assert_eq!(saved.ui_scale, 125);
    assert!(!saved.hud_mana);
    assert_eq!(std::fs::read(path.with_added_extension("malformed"))?, b"{");
    Ok(())
}

#[cfg(test)]
fn blocked_import_restore(scales: &[u16]) -> io::Result<()> {
    use super::super::SettingsState;
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    PreferenceStore::export_to(&ReferencePreferences::default(), &path)?;
    PreferenceStore::export_to(
        &ReferencePreferences {
            ui_scale: 110,
            ..Default::default()
        },
        &path.with_added_extension("ready-backup"),
    )?;
    std::fs::create_dir(path.with_extension("backup.json"))?;
    let mut state = SettingsState::new(Some(path.clone()));
    let mut expected = std::fs::read(&path)?;
    for &scale in scales {
        let source = root.path().join(format!("{scale}.json"));
        PreferenceStore::export_to(
            &ReferencePreferences {
                ui_scale: scale,
                ..Default::default()
            },
            &source,
        )?;
        expected = std::fs::read(&path)?;
        state.import_from(&source)?;
    }
    let imported = std::fs::read(&path)?;
    state.restore_backup()?;
    assert!(std::fs::read(&path)? == expected, "wrong rollback snapshot");
    state.restore_backup()?;
    assert!(std::fs::read(&path)? == imported, "lost pre-restore bytes");
    Ok(())
}

#[cfg(test)]
#[test]
fn blocked_import_once_restores_latest() -> io::Result<()> {
    blocked_import_restore(&[125])
}

#[cfg(test)]
#[test]
fn blocked_import_twice_restores_latest() -> io::Result<()> {
    blocked_import_restore(&[125, 140])
}

#[cfg(test)]
#[test]
fn restore_skips_invalid_legacy_snapshots_and_breaks_generation_ties() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let recovery = path.with_added_extension("recovery");
    let ready = path.with_added_extension("ready-backup");
    let slot = path.with_extension("backup.json");
    let generation = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
    for (candidate, scale) in [(&slot, 120), (&ready, 130), (&recovery, 140)] {
        PreferenceStore::export_to(
            &ReferencePreferences {
                ui_scale: scale,
                ..Default::default()
            },
            candidate,
        )?;
        std::fs::File::options()
            .write(true)
            .open(candidate)?
            .set_modified(generation)?;
    }
    let mut state = super::super::SettingsState::new(Some(path.clone()));
    state.restore_backup()?;
    assert_eq!(state.preferences.ui_scale, 140);
    // The new rollback outranks even the future-dated legacy files.
    state.restore_backup()?;
    assert_eq!(
        state.preferences.ui_scale,
        ReferencePreferences::default().ui_scale
    );
    std::fs::write(&slot, b"{")?;
    std::fs::File::options()
        .write(true)
        .open(&slot)?
        .set_modified(generation + std::time::Duration::from_secs(60))?;
    state.restore_backup()?;
    assert_eq!(state.preferences.ui_scale, 140);
    Ok(())
}

#[cfg(test)]
#[test]
fn failed_active_publication_preserves_the_previous_backup() -> io::Result<()> {
    for reset in [false, true] {
        let root = tempfile::tempdir()?;
        let path = root.path().join("preferences.json");
        let slot = path.with_extension("backup.json");
        let active = ReferencePreferences { ui_scale: 125, ..Default::default() };
        let saved = ReferencePreferences { ui_scale: 140, ..Default::default() };
        PreferenceStore::export_to(&active, &path)?;
        PreferenceStore::export_to(&saved, &slot)?;
        let bytes = std::fs::read(&slot)?;
        let mut state = super::super::SettingsState::new(Some(path.clone()));
        crate::atomic_file::fail_durable_rename_to(&path);
        if reset {
            assert!(state.reset_defaults().is_err());
        } else {
            let import = root.path().join("import.json");
            PreferenceStore::export_to(&ReferencePreferences::default(), &import)?;
            assert!(state.import_from(&import).is_err());
        }
        assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, active.ui_scale);
        assert_eq!(std::fs::read(PreferenceStore::load(path.clone())?.backup_path())?, bytes);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn restore_with_deferred_backup_promotion_reports_the_committed_settings() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let slot = path.with_extension("backup.json");
    let active = ReferencePreferences { ui_scale: 125, ..Default::default() };
    let saved = ReferencePreferences { ui_scale: 140, ..Default::default() };
    PreferenceStore::export_to(&active, &path)?;
    PreferenceStore::export_to(&saved, &slot)?;
    let mut state = super::super::SettingsState::new(Some(path.clone()));
    crate::atomic_file::fail_durable_rename_to(&slot);
    let restored = state.restore_backup();
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, saved.ui_scale);
    restored?;
    assert_eq!(state.preferences.ui_scale, saved.ui_scale);
    assert_eq!(PreferenceStore::read_transfer(&PreferenceStore::load(path)?.backup_path())?.ui_scale, active.ui_scale);
    Ok(())
}
