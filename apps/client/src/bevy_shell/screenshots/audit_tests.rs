use super::tests::{TestResult, app, capture, capture_frame, colored_frame};
use super::*;
use crate::play::screenshot::TestDir;

#[cfg(unix)]
#[test]
fn audit_f12_replaces_png_with_private_capture() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let output = root.join("new").join("audit.json");
    let mut app = app(&root, false)?;
    app.insert_resource(super::super::audit::AuditOutput::new(Some(output.clone())));
    capture(&mut app)?;
    let path = output.with_extension("png");
    let first = std::fs::read(&path)?;
    assert_eq!(&first[..8], b"\x89PNG\r\n\x1a\n");
    let second_frame = colored_frame([80, 100, 120, 255]);
    capture_frame(&mut app, second_frame.clone())?;
    let second = std::fs::read(&path)?;
    assert_eq!(&second[..8], b"\x89PNG\r\n\x1a\n");
    assert_ne!(second, first);
    let mut expected = std::io::Cursor::new(Vec::new());
    let format = bevy::image::ImageFormat::Png
        .as_image_crate_format()
        .ok_or("PNG format")?;
    second_frame
        .try_into_dynamic()?
        .into_rgb8()
        .write_to(&mut expected, format)?;
    assert_eq!(second, expected.into_inner());
    assert_eq!(path.metadata()?.permissions().mode() & 0o777, 0o600);
    let folder = path.parent().ok_or("capture directory")?;
    assert_eq!(folder.metadata()?.permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::read_dir(folder)?.count(), 1);
    Ok(())
}

#[cfg(unix)]
#[test]
fn f12_preserves_legacy_capture_in_existing_directory() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))?;
    let legacy = root.join("oteryn-1-2.bmp");
    std::fs::write(&legacy, b"BM legacy capture")?;
    std::fs::set_permissions(&legacy, std::fs::Permissions::from_mode(0o644))?;
    let original = root.join("oteryn-3-4.bmp");
    let backup = root.join("hard-linked-backup.bmp");
    std::fs::write(&original, b"BM linked capture")?;
    std::fs::set_permissions(&original, std::fs::Permissions::from_mode(0o644))?;
    std::fs::hard_link(&original, &backup)?;
    capture(&mut app(&root, false)?)?;
    for linked in [&original, &backup] {
        assert_eq!(linked.metadata()?.permissions().mode() & 0o777, 0o644);
        assert_eq!(std::fs::read(linked)?, b"BM linked capture");
    }
    let mode = root.metadata()?.permissions().mode() & 0o777;
    let legacy_mode = legacy.metadata()?.permissions().mode() & 0o777;
    let data = std::fs::read(&legacy);
    let entries = std::fs::read_dir(&root)?.count();
    assert_eq!(entries, 4);
    assert_eq!(mode, 0o755);
    assert_eq!(legacy_mode, 0o644);
    assert_eq!(data?, b"BM legacy capture");
    Ok(())
}

#[cfg(unix)]
fn audit_capture_after_json(basename: &str, parent_mode: u32, expected_mode: u32) -> TestResult {
    use super::super::audit;
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let folder = root.join("audit");
    let output = folder.join(basename).with_extension("json");
    let mut app = app(&root, false)?;
    app.insert_resource(audit::AuditOutput::new(Some(output.clone())))
        .add_systems(PostUpdate, audit::write);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    let json = std::fs::read(&output)?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&json)?["schema"],
        "oteryn.bevy-ui-audit.v1"
    );
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(parent_mode))?;
    let legacy = folder.join("oteryn-1-2-3.bmp");
    let lookalike = folder.join("oteryn-1-2.bmp");
    let unrelated = folder.join("unrelated.bmp");
    if parent_mode & 0o020 != 0 {
        for file in [&legacy, &lookalike, &unrelated] {
            std::fs::write(file, b"BM previous file")?;
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644))?;
        }
    }
    capture(&mut app)?;
    if parent_mode & 0o020 != 0 {
        assert_eq!(legacy.metadata()?.permissions().mode() & 0o777, 0o644);
        assert_eq!(std::fs::read(legacy)?, b"BM previous file");
        for file in [&lookalike, &unrelated] {
            assert_eq!(file.metadata()?.permissions().mode() & 0o777, 0o644);
            assert_eq!(std::fs::read(file)?, b"BM previous file");
        }
    }
    let path = output.with_extension("png");
    let saved = std::fs::read(&path);
    let png_mode = path
        .metadata()
        .map(|entry| entry.permissions().mode() & 0o777);
    let mode = folder.metadata()?.permissions().mode() & 0o1777;
    let entries = std::fs::read_dir(&folder)?.count();
    assert_eq!(&saved?[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(png_mode?, 0o600);
    assert_eq!(mode, expected_mode);
    assert_eq!(entries, if parent_mode & 0o020 != 0 { 5 } else { 2 });
    Ok(())
}

#[cfg(unix)]
#[test]
fn audit_f12_after_json_preserves_existing_directory() -> TestResult {
    audit_capture_after_json("audit", 0o755, 0o755)
}

#[cfg(unix)]
#[test]
fn audit_f12_accepts_maximum_audit_staging_basename() -> TestResult {
    audit_capture_after_json(&"a".repeat(244), 0o755, 0o755)
}

#[cfg(unix)]
#[test]
fn audit_f12_writes_private_png_in_shared_sticky_directory() -> TestResult {
    audit_capture_after_json("audit", 0o1777, 0o1777)
}
#[cfg(unix)]
#[test]
fn audit_f12_preserves_group_writable_directory() -> TestResult {
    audit_capture_after_json("audit", 0o775, 0o775)
}
