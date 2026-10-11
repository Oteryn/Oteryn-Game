//! Oteryn-local options ZIP. Original archive interoperability is unqualified.
use oteryn_client::settings::ClientSettings;
use std::{
    io::{self, Write},
    path::Path,
};

pub(crate) const OPTIONS_ENTRY: &str = "oteryn-options.json";

fn resolved_destination(path: &Path) -> io::Result<std::path::PathBuf> {
    if let Ok(existing) = std::fs::canonicalize(path) {
        return Ok(existing);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Missing filename"))?;
    Ok(std::fs::canonicalize(parent)?.join(name))
}

pub(crate) fn save(settings: &ClientSettings, destination: &Path) -> io::Result<()> {
    save_guarded(settings, destination, ClientSettings::path().as_deref())
}

fn save_guarded(
    settings: &ClientSettings,
    destination: &Path,
    active: Option<&Path>,
) -> io::Result<()> {
    // Reuse exactly the ordinary validated/migrated persistence representation,
    // not a second serializer or an account-directory export.
    let staging = tempfile::tempdir()?;
    let preferences = staging.path().join(OPTIONS_ENTRY);
    settings.save(&preferences)?;
    let bytes = std::fs::read(preferences)?;
    if let Some(active) = active
        && let (Ok(active), Ok(selected)) = (
            resolved_destination(active),
            resolved_destination(destination),
        )
        && active == selected
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot replace active preferences with an archive",
        ));
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // Write and sync the entire archive in its destination filesystem before
    // replacement. Failed validation/encoding/writing preserves the old file.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut archive = zip::ZipWriter::new(temporary.as_file_mut());
        archive.start_file(
            OPTIONS_ENTRY,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )?;
        archive.write_all(&bytes)?;
        archive.finish()?;
    }
    temporary.as_file().sync_all()?;
    temporary
        .persist(destination)
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(windows)]
pub(crate) fn choose_destination(
    english: bool,
    parent: &winit::window::Window,
) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_parent(parent)
        .set_title(if english {
            "Select export file..."
        } else {
            "Wybierz plik eksportu..."
        })
        .add_filter(
            if english {
                "ZIP archives"
            } else {
                "Archiwa ZIP"
            },
            &["zip"],
        )
        .add_filter(
            if english {
                "All files"
            } else {
                "Wszystkie pliki"
            },
            &["*"],
        )
        .set_file_name("oteryn-options.zip")
        .save_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn archive_contains_only_validated_oteryn_preferences() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let destination = directory.path().join("selected.zip");
        let settings = ClientSettings {
            fullscreen: true,
            fps: 7,
            last_frame_limit: 7,
            ..Default::default()
        };
        let before = settings.clone();
        std::fs::write(&destination, b"previous archive")?;
        save(&settings, &destination)?;
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&destination)?)?;
        assert_eq!(archive.len(), 1);
        let mut entry = archive.by_name(OPTIONS_ENTRY)?;
        assert_eq!(entry.compression(), zip::CompressionMethod::Stored);
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let round_trip = directory.path().join("round-trip.json");
        std::fs::write(&round_trip, bytes)?;
        assert_eq!(ClientSettings::load(&round_trip)?, before);
        assert_eq!(settings, before);
        Ok(())
    }

    #[test]
    fn invalid_options_and_missing_parent_preserve_existing_export()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let destination = directory.path().join("selected.zip");
        std::fs::write(&destination, b"previous archive")?;
        let invalid = ClientSettings {
            ui_scale: f32::NAN,
            ..Default::default()
        };
        assert!(save(&invalid, &destination).is_err());
        assert_eq!(std::fs::read(&destination)?, b"previous archive");
        assert!(
            save(
                &ClientSettings::default(),
                &directory.path().join("missing/export.zip")
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&destination)?, b"previous archive");
        assert!(save(&ClientSettings::default(), directory.path()).is_err());
        assert!(directory.path().is_dir());
        Ok(())
    }

    #[test]
    fn active_destination_resolution_also_covers_a_missing_preferences_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let active = directory.path().join("settings.json");
        let alternate = directory.path().join(".").join("settings.json");
        assert_eq!(
            resolved_destination(&active)?,
            resolved_destination(&alternate)?
        );
        assert!(save_guarded(&ClientSettings::default(), &alternate, Some(&active)).is_err());
        assert!(!active.exists());
        std::fs::write(&active, b"existing preferences")?;
        assert_eq!(
            resolved_destination(&active)?,
            resolved_destination(&alternate)?
        );
        assert!(save_guarded(&ClientSettings::default(), &alternate, Some(&active)).is_err());
        assert_eq!(std::fs::read(&active)?, b"existing preferences");
        Ok(())
    }
}
