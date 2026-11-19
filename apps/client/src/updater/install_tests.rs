use super::install::*;
use std::{fs, io::Write, path::Path};
fn package(path: &Path, name: &str, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut zip = zip::ZipWriter::new(fs::File::create(path)?);
    zip.start_file(name, zip::write::SimpleFileOptions::default())?;
    zip.write_all(bytes)?;
    zip.finish()?;
    Ok(())
}
#[test]
fn repair_installs_fresh_slot_and_verification_detects_changed_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("oteryn-repair-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;
    let zip = root.join("release.zip");
    package(&zip, "oteryn-client.exe", b"signed executable")?;
    let first = install(&zip, &root, "rel-a", None)?;
    assert_eq!(first, "rel-a");
    verify_archive(&zip, &root.join("releases/rel-a"))?;
    fs::write(
        root.join("releases/rel-a/oteryn-client.exe"),
        b"altered executable",
    )?;
    assert!(verify_archive(&zip, &root.join("releases/rel-a")).is_err());
    let repaired = install(&zip, &root, "rel-a", None)?;
    assert_eq!(repaired, "rel-a~1");
    verify_archive(&zip, &root.join("releases/rel-a~1"))?;
    assert_eq!(
        fs::read_to_string(root.join("current.txt"))?.trim(),
        "rel-a~1"
    );
    package(&zip, "../escape.exe", b"escape")?;
    assert!(install(&zip, &root, "rel-b", None).is_err());
    assert!(!root.join("escape.exe").exists());
    assert_eq!(
        fs::read_to_string(root.join("current.txt"))?.trim(),
        "rel-a~1"
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn login_updater_requires_channel_trust_before_starting() {
    assert_eq!(
        super::login::require_update_root(&[]),
        Err(oteryn_platform_client::updates::UpdateError::Configuration),
    );
}

#[test]
fn activation_preserves_the_manual_installers_slot_format() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_platform_client::updates::UpdateError;
    let root = std::env::temp_dir().join(format!("oteryn-updater-slot-{}", std::process::id()));
    fs::create_dir_all(&root)?;
    let zip = root.join("release.zip");
    package(&zip, "oteryn-client.exe", b"signed executable")?;
    let max = "a".repeat(64);
    fs::create_dir_all(root.join("releases").join(&max))?;
    fs::write(root.join("current.txt"), &max)?;
    assert!(super::release_directory(&max));
    for release in [&max, &"b".repeat(65), ".rel", "rel..a"] {
        assert_eq!(
            install(&zip, &root, release, None),
            Err(UpdateError::Verification)
        );
        assert_eq!(fs::read_to_string(root.join("current.txt"))?.trim(), max);
    }
    fs::remove_dir_all(root)?;
    Ok(())
}

fn write_launcher_image(path: &Path, version: (u16, u16)) -> std::io::Result<()> {
    // DOS header with the PE signature at 0x80, then the COFF and PE32+ headers.
    let mut image = [0_u8; 0x188];
    image[..2].copy_from_slice(b"MZ");
    image[0x3c..0x40].copy_from_slice(&0x80_u32.to_le_bytes());
    image[0x80..0x84].copy_from_slice(b"PE\0\0");
    image[0x94..0x96].copy_from_slice(&240_u16.to_le_bytes());
    image[0x98..0x9a].copy_from_slice(&0x20b_u16.to_le_bytes());
    image[0xc4..0xc6].copy_from_slice(&version.0.to_le_bytes());
    image[0xc6..0xc8].copy_from_slice(&version.1.to_le_bytes());
    fs::write(path, image)
}

#[test]
fn legacy_launcher_cannot_activate_an_opaque_release() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_platform_client::updates::UpdateError;
    let root = std::env::temp_dir().join(format!("oteryn-legacy-launcher-{}", std::process::id()));
    fs::create_dir_all(root.join("releases"))?;
    let previous = crate::RELEASE_ID;
    fs::create_dir_all(root.join("releases").join(previous))?;
    let pointer = root.join("current.txt");
    fs::write(&pointer, previous)?;
    let launcher = root.join("oteryn-launcher.exe");
    write_launcher_image(&launcher, (0, 0))?;
    let zip = root.join("release.zip");
    package(&zip, "oteryn-client.exe", b"signed executable")?;

    assert_eq!(
        install(&zip, &root, "rel-a", Some(&launcher)),
        Err(UpdateError::Unavailable)
    );
    assert_eq!(fs::read_to_string(&pointer)?, previous);
    assert!(!root.join("releases/rel-a").exists());

    write_launcher_image(&launcher, super::launcher::PE_IMAGE_VERSION)?;
    assert_eq!(install(&zip, &root, "rel-a", Some(&launcher))?, "rel-a");
    assert_eq!(fs::read_to_string(&pointer)?.trim(), "rel-a");
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn new_release_slots_leave_room_for_repair() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_platform_client::updates::UpdateError;
    let root = std::env::temp_dir().join(format!("oteryn-repair-limit-{}", std::process::id()));
    fs::create_dir_all(&root)?;
    let zip = root.join("release.zip");
    package(&zip, "oteryn-client.exe", b"signed executable")?;
    let max = "a".repeat(62);
    assert_eq!(install(&zip, &root, &max, None)?, max);
    let repaired = format!("{max}~1");
    assert_eq!(install(&zip, &root, &max, None)?, repaired);
    let oversized = install(&zip, &root, &"b".repeat(63), None);
    let active = fs::read_to_string(root.join("current.txt"))?;
    fs::remove_dir_all(root)?;
    assert_eq!(oversized, Err(UpdateError::Verification));
    assert_eq!(active.trim(), repaired);
    Ok(())
}
