//! Oteryn-local options ZIP. Original archive interoperability is unqualified.
use oteryn_client::settings::ClientSettings;
use std::{
    io::{self, Read, Write},
    path::Path,
};

pub(crate) const OPTIONS_ENTRY: &str = "oteryn-options.json";
const MAX_OPTIONS_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = MAX_OPTIONS_BYTES + 64 * 1024;

pub(crate) fn load(source: &Path) -> io::Result<ClientSettings> {
    let mut bytes = Vec::new();
    std::fs::File::open(source)?
        .take(MAX_ARCHIVE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(io::Error::other("Options archive exceeds its size bound"));
    }
    let mut archive = zip::ZipArchive::new(io::Cursor::new(&bytes))?;
    if archive.len() != 1 || archive.offset() != 0 || !archive.comment().is_empty() {
        return Err(io::Error::other(
            "Only a single Oteryn options entry is supported",
        ));
    }
    let central_start = archive.central_directory_start();
    let mut entry = archive.by_index(0)?;
    if entry.name_raw() != OPTIONS_ENTRY.as_bytes()
        || !entry.is_file()
        || entry.encrypted()
        || entry.compression() != zip::CompressionMethod::Stored
        || entry.size() > MAX_OPTIONS_BYTES
        || entry.compressed_size() != entry.size()
        || !entry.comment().is_empty()
        || entry.extra_data_fields().next().is_some()
        || entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 != 0o100000)
    {
        return Err(io::Error::other("Unsupported options archive entry"));
    }
    // Accept only the flat ZIP32 Stored envelope our exporter writes. The
    // upstream stream visitor swallows malformed central-record parse errors;
    // callback counts alone cannot establish that it exhausted the directory.
    // Parsed offsets plus the fixed ZIP32 header/footer sizes exclude gaps,
    // descriptors, extra records and trailing data without a second ZIP parser.
    let name_bytes = OPTIONS_ENTRY.len() as u64;
    let data_start = 30 + name_bytes;
    let expected_central = data_start + entry.size();
    let expected_length = expected_central + 46 + name_bytes + 22;
    if entry.header_start() != 0
        || entry.data_start() != Some(data_start)
        || entry.central_header_start() != expected_central
        || central_start != expected_central
        || bytes.len() as u64 != expected_length
    {
        return Err(io::Error::other("Unsupported options archive layout"));
    }
    let mut physical = SingleOptionsEntry {
        crc: entry.crc32(),
        size: entry.size(),
        bytes: None,
        central_count: 0,
    };
    let mut options = Vec::new();
    (&mut entry)
        .take(MAX_OPTIONS_BYTES + 1)
        .read_to_end(&mut options)?;
    if options.len() as u64 != physical.size {
        return Err(io::Error::other("Truncated options entry"));
    }
    // ZipArchive deduplicates names. The upstream stream visitor independently
    // checks physical local/central record counts, names, sizes and both CRCs.
    zip::unstable::stream::ZipStreamReader::new(io::Cursor::new(&bytes)).visit(&mut physical)?;
    if physical.central_count != 1 || physical.bytes.as_deref() != Some(options.as_slice()) {
        return Err(io::Error::other("Ambiguous options archive"));
    }
    let staging = tempfile::tempdir()?;
    let preferences = staging.path().join(OPTIONS_ENTRY);
    std::fs::write(&preferences, options)?;
    ClientSettings::load(&preferences)
}

struct SingleOptionsEntry {
    crc: u32,
    size: u64,
    bytes: Option<Vec<u8>>,
    central_count: u8,
}

impl zip::unstable::stream::ZipStreamVisitor for SingleOptionsEntry {
    fn visit_file<R: Read>(
        &mut self,
        file: &mut zip::read::ZipFile<'_, R>,
    ) -> zip::result::ZipResult<()> {
        if self.bytes.is_some()
            || file.name_raw() != OPTIONS_ENTRY.as_bytes()
            || file.encrypted()
            || file.compression() != zip::CompressionMethod::Stored
            || file.size() != self.size
            || file.compressed_size() != self.size
            || file.crc32() != self.crc
            || file.extra_data_fields().next().is_some()
        {
            return Err(zip::result::ZipError::UnsupportedArchive(
                "Unsupported physical options entry",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_OPTIONS_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != self.size {
            return Err(zip::result::ZipError::UnsupportedArchive(
                "Truncated physical options entry",
            ));
        }
        self.bytes = Some(bytes);
        Ok(())
    }

    fn visit_additional_metadata(
        &mut self,
        entry: &zip::read::ZipFileEntry<'_>,
    ) -> zip::result::ZipResult<()> {
        if self.central_count != 0
            || entry.name_raw() != OPTIONS_ENTRY.as_bytes()
            || !entry.is_file()
            || entry.encrypted()
            || entry.size() != self.size
            || entry.crc32() != self.crc
        {
            return Err(zip::result::ZipError::UnsupportedArchive(
                "Unsupported central options entry",
            ));
        }
        self.central_count = 1;
        Ok(())
    }
}

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

#[cfg(windows)]
pub(crate) fn choose_source(
    english: bool,
    parent: &winit::window::Window,
) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_parent(parent)
        .set_title(if english {
            "Import Oteryn options ZIP..."
        } else {
            "Importuj ZIP ustawień Oteryn..."
        })
        .add_filter("Oteryn options ZIP", &["zip"])
        .pick_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options_zip(name: &str, bytes: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut archive = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        archive.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )?;
        archive.write_all(bytes)?;
        Ok(archive.finish()?.into_inner())
    }

    #[test]
    fn options_zip_import_roundtrips_exports_and_migrates_legacy_settings()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("options.zip");
        let mut settings = ClientSettings {
            fullscreen: true,
            fps: 7,
            last_frame_limit: 7,
            ..Default::default()
        };
        save(&settings, &path)?;
        let exported = std::fs::read(&path)?;
        assert_eq!(load(&path)?, settings);
        assert_eq!(std::fs::read(&path)?, exported);
        settings.version = 1;
        let json = serde_json::to_vec(&settings)?;
        let legacy = directory.path().join("legacy.json");
        std::fs::write(&legacy, &json)?;
        std::fs::write(&path, options_zip(OPTIONS_ENTRY, &json)?)?;
        assert_eq!(load(&path)?, ClientSettings::load(&legacy)?);
        assert_eq!(load(&path)?.version, ClientSettings::default().version);
        Ok(())
    }

    #[test]
    fn options_zip_import_rejects_other_entries_paths_and_symlinks()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("options.zip");
        let json = serde_json::to_vec(&ClientSettings::default())?;
        for name in [
            "clientoptions.json",
            "minimap.bin",
            "../oteryn-options.json",
            "/oteryn-options.json",
            "nested/oteryn-options.json",
            "oteryn-options.json/",
        ] {
            std::fs::write(&path, options_zip(name, &json)?)?;
            assert!(load(&path).is_err(), "{name}");
        }
        let mut archive = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        archive.add_symlink(
            OPTIONS_ENTRY,
            "outside.json",
            zip::write::SimpleFileOptions::default(),
        )?;
        std::fs::write(&path, archive.finish()?.into_inner())?;
        assert!(load(&path).is_err());
        let mut archive = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        for name in [OPTIONS_ENTRY, "minimap.bin"] {
            archive.start_file(name, zip::write::SimpleFileOptions::default())?;
            archive.write_all(&json)?;
        }
        std::fs::write(&path, archive.finish()?.into_inner())?;
        assert!(load(&path).is_err());
        assert!(!directory.path().join("outside.json").exists());
        assert!(!directory.path().join(OPTIONS_ENTRY).exists());
        Ok(())
    }

    #[test]
    fn options_zip_import_rejects_corruption_encryption_and_physical_duplicates()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("options.zip");
        let original = options_zip(
            OPTIONS_ENTRY,
            &serde_json::to_vec(&ClientSettings::default())?,
        )?;
        let mut parsed = zip::ZipArchive::new(io::Cursor::new(&original))?;
        let central = usize::try_from(parsed.central_directory_start())?;
        let data = usize::try_from(parsed.by_index(0)?.data_start().ok_or("data start")?)?;
        let eocd = original.len() - 22; // This test's writer emits an uncommented ZIP32 footer.
        let mut damaged = Vec::new();
        damaged.push(original[..original.len() - 1].to_vec());
        for (offset, byte) in [
            (data + 5, original[data + 5] ^ 1),
            (30, b'x'),
            (6, 1),
            (8, 8),
            (14, original[14] ^ 1), // Local CRC differs from valid indexed metadata.
            (18, original[18] ^ 1), // Local compressed size differs.
            (22, original[22] ^ 1), // Local uncompressed size differs.
            (central + 8, 1),
            (central + 10, 8),
        ] {
            let mut bytes = original.clone();
            bytes[offset] = byte;
            damaged.push(bytes);
        }
        let mut duplicate_central = original[..eocd].to_vec();
        duplicate_central.extend_from_slice(&original[central..eocd]);
        duplicate_central.extend_from_slice(&original[eocd..]);
        let end = duplicate_central.len() - 22;
        duplicate_central[end + 8..end + 10].copy_from_slice(&2_u16.to_le_bytes());
        duplicate_central[end + 10..end + 12].copy_from_slice(&2_u16.to_le_bytes());
        duplicate_central[end + 12..end + 16]
            .copy_from_slice(&u32::try_from(2 * (eocd - central))?.to_le_bytes());
        // Prove the mature indexed reader hides this duplicate; our stream count must refuse it.
        assert_eq!(
            zip::ZipArchive::new(io::Cursor::new(&duplicate_central))?.len(),
            1
        );
        damaged.push(duplicate_central);
        let mut duplicate_local = original[..central].to_vec();
        duplicate_local.extend_from_slice(&original);
        let end = duplicate_local.len() - 22;
        duplicate_local[end + 16..end + 20]
            .copy_from_slice(&u32::try_from(2 * central)?.to_le_bytes());
        assert_eq!(
            zip::ZipArchive::new(io::Cursor::new(&duplicate_local))?.len(),
            1
        );
        damaged.push(duplicate_local);
        // Indexed reading ignores an incomplete extra central record, while
        // ZipStreamReader silently treats its parse error as normal termination.
        let mut interrupted_central = original[..eocd].to_vec();
        interrupted_central.extend_from_slice(b"PK\x01\x02\x00");
        interrupted_central.extend_from_slice(&original[eocd..]);
        let end = interrupted_central.len() - 22;
        interrupted_central[end + 12..end + 16]
            .copy_from_slice(&u32::try_from(eocd - central + 5)?.to_le_bytes());
        assert_eq!(
            zip::ZipArchive::new(io::Cursor::new(&interrupted_central))?.len(),
            1
        );
        damaged.push(interrupted_central);
        for (index, bytes) in damaged.into_iter().enumerate() {
            std::fs::write(&path, &bytes)?;
            assert!(load(&path).is_err(), "corruption case {index}");
            assert_eq!(std::fs::read(&path)?, bytes);
        }
        Ok(())
    }

    #[test]
    fn options_zip_import_honours_eight_mib_bounds_and_validates_payloads()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("options.zip");
        let settings = ClientSettings::default();
        let mut json = serde_json::to_vec(&settings)?;
        json.resize(usize::try_from(MAX_OPTIONS_BYTES)?, b' ');
        std::fs::write(&path, options_zip(OPTIONS_ENTRY, &json)?)?;
        assert_eq!(load(&path)?, settings);
        json.push(b' ');
        std::fs::write(&path, options_zip(OPTIONS_ENTRY, &json)?)?;
        assert!(load(&path).is_err());
        std::fs::File::create(&path)?.set_len(MAX_ARCHIVE_BYTES + 1)?;
        assert!(load(&path).is_err());
        for payload in [b"not json".as_slice(), b"{\"unsupported\":true}".as_slice()] {
            std::fs::write(&path, options_zip(OPTIONS_ENTRY, payload)?)?;
            assert!(load(&path).is_err());
        }
        let mut invalid = settings;
        invalid.version = 255;
        std::fs::write(
            &path,
            options_zip(OPTIONS_ENTRY, &serde_json::to_vec(&invalid)?)?,
        )?;
        assert!(load(&path).is_err());
        Ok(())
    }

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
