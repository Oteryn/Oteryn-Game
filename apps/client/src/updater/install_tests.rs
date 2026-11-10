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
    let first = install(&zip, &root, "rel-a")?;
    assert_eq!(first, "rel-a");
    verify_archive(&zip, &root.join("releases/rel-a"))?;
    fs::write(
        root.join("releases/rel-a/oteryn-client.exe"),
        b"altered executable",
    )?;
    assert!(verify_archive(&zip, &root.join("releases/rel-a")).is_err());
    let repaired = install(&zip, &root, "rel-a")?;
    assert_eq!(repaired, "rel-a~1");
    verify_archive(&zip, &root.join("releases/rel-a~1"))?;
    assert_eq!(
        fs::read_to_string(root.join("current.txt"))?.trim(),
        "rel-a~1"
    );
    package(&zip, "../escape.exe", b"escape")?;
    assert!(install(&zip, &root, "rel-b").is_err());
    assert!(!root.join("escape.exe").exists());
    assert_eq!(
        fs::read_to_string(root.join("current.txt"))?.trim(),
        "rel-a~1"
    );
    fs::remove_dir_all(root)?;
    Ok(())
}
