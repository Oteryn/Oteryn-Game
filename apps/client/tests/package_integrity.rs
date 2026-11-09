use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "oteryn-package-check-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root)?;
        Ok(Self(root))
    }
    fn run_client(&self) -> Result<Output, Box<dyn std::error::Error>> {
        let path = self.0.join(if cfg!(windows) {
            "oteryn-client.exe"
        } else {
            "oteryn-client"
        });
        std::fs::copy(env!("CARGO_BIN_EXE_oteryn-client"), &path)?;
        let mut command = Command::new(path);
        command
            .arg("--check-packages")
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = std::env::var_os("LD_LIBRARY_PATH") {
            command.env("LD_LIBRARY_PATH", path);
        }
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let mut child = command.spawn()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err("package check started UI/network instead of returning".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(child.wait_with_output()?)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn package_check_rejects_malformed_manifest_before_any_login() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::write(fixture.0.join("packages.json"), b"invalid JSON")?;
    let output = fixture.run_client()?;
    assert!(
        !output.status.success(),
        "corrupt manifest was accepted before login"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("packages.json"));
    Ok(())
}

fn manifest() -> serde_json::Value {
    let identity = oteryn_client::package_integrity::Identity::compiled();
    serde_json::json!({"schema":"oteryn.client.packages.v1", "release": {
        "release_id":identity.release_id, "client_version":identity.client_version, "channel":identity.channel, "game_commit":identity.game_commit, "target":identity.target
    },"packages":[{"id":"scene","version":"1.2.3","path":"packages/scene.bin","size":3,"sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","provenance":{"source":"oteryn-generated","source_ref":"Game test fixture generator"}}]})
}
fn write_manifest(fixture: &Fixture, document: &serde_json::Value) -> TestResult {
    std::fs::write(
        fixture.0.join("packages.json"),
        serde_json::to_vec(document)?,
    )?;
    Ok(())
}
fn seeded() -> Result<Fixture, Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    std::fs::create_dir_all(fixture.0.join("packages"))?;
    std::fs::write(fixture.0.join("packages/scene.bin"), b"abc")?;
    write_manifest(&fixture, &manifest())?;
    Ok(fixture)
}
#[test]
fn executable_verifies_packages_then_rejects_same_size_corruption() -> TestResult {
    let fixture = seeded()?;
    let valid = fixture.run_client()?;
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    assert!(String::from_utf8_lossy(&valid.stdout).contains("Verified 1 presentation package"));
    std::fs::write(fixture.0.join("packages/scene.bin"), b"abd")?;
    let corrupt = fixture.run_client()?;
    assert!(!corrupt.status.success());
    assert!(String::from_utf8_lossy(&corrupt.stderr).contains("integrity mismatch"));
    Ok(())
}
#[test]
fn every_release_identity_field_is_checked_even_with_an_empty_package_list() -> TestResult {
    use oteryn_client::package_integrity::{Identity, Report, verify};
    let fixture = seeded()?;
    let mut document = manifest();
    document["packages"] = serde_json::json!([]);
    write_manifest(&fixture, &document)?;
    assert_eq!(
        verify(&fixture.0, Identity::compiled())?,
        Report::Verified {
            packages: 0,
            bytes: 0
        }
    );
    for key in [
        "release_id",
        "client_version",
        "channel",
        "game_commit",
        "target",
    ] {
        let mut wrong = document.clone();
        wrong["release"][key] = "different-build".into();
        write_manifest(&fixture, &wrong)?;
        let error = verify(&fixture.0, Identity::compiled())
            .err()
            .ok_or("mismatched identity accepted")?;
        assert!(error.contains(key), "{error}");
    }
    Ok(())
}
#[test]
fn invalid_schema_paths_size_provenance_and_missing_packages_fail_closed() -> TestResult {
    use oteryn_client::package_integrity::{Identity, verify};
    let fixture = seeded()?;
    for path in [
        "../outside",
        "packages/../outside",
        "/absolute",
        "C:\\absolute",
        "packages\\..\\outside",
    ] {
        let mut document = manifest();
        document["packages"][0]["path"] = path.into();
        write_manifest(&fixture, &document)?;
        assert!(verify(&fixture.0, Identity::compiled()).is_err());
    }
    for (key, value) in [
        ("size", serde_json::json!(4)),
        ("sha256", serde_json::json!("ABC")),
        ("version", serde_json::json!("invalid")),
        ("id", serde_json::json!("INVALID")),
        (
            "provenance",
            serde_json::json!({"source":"reference-assets","source_ref":"not redistributable"}),
        ),
    ] {
        let mut document = manifest();
        document["packages"][0][key] = value;
        write_manifest(&fixture, &document)?;
        assert!(verify(&fixture.0, Identity::compiled()).is_err());
    }
    let mut document = manifest();
    document["schema"] = "unknown".into();
    write_manifest(&fixture, &document)?;
    assert!(verify(&fixture.0, Identity::compiled()).is_err());
    let mut document = manifest();
    document["packages"] = serde_json::json!([
        document["packages"][0].clone(),
        document["packages"][0].clone()
    ]);
    write_manifest(&fixture, &document)?;
    assert!(verify(&fixture.0, Identity::compiled()).is_err());
    write_manifest(&fixture, &manifest())?;
    std::fs::remove_file(fixture.0.join("packages/scene.bin"))?;
    assert!(verify(&fixture.0, Identity::compiled()).is_err());
    Ok(())
}
#[cfg(unix)]
#[test]
fn package_symlinks_cannot_escape_the_release_packages_directory() -> TestResult {
    use oteryn_client::package_integrity::{Identity, verify};
    let fixture = seeded()?;
    let outside = Fixture::new()?;
    std::fs::write(outside.0.join("scene.bin"), b"abc")?;
    std::fs::remove_file(fixture.0.join("packages/scene.bin"))?;
    std::os::unix::fs::symlink(
        outside.0.join("scene.bin"),
        fixture.0.join("packages/scene.bin"),
    )?;
    assert!(verify(&fixture.0, Identity::compiled()).is_err());
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn launcher_diagnosis_runs_the_active_release_verifier() -> TestResult {
    let fixture = Fixture::new()?;
    let release = fixture.0.join("releases").join(oteryn_client::RELEASE_ID);
    std::fs::create_dir_all(release.join("packages"))?;
    std::fs::write(release.join("packages/scene.bin"), b"abc")?;
    std::fs::write(
        release.join("packages.json"),
        serde_json::to_vec(&manifest())?,
    )?;
    std::fs::write(fixture.0.join("current.txt"), oteryn_client::RELEASE_ID)?;
    std::fs::copy(
        env!("CARGO_BIN_EXE_oteryn-client"),
        release.join("oteryn-client.exe"),
    )?;
    let launcher = fixture.0.join("oteryn-launcher");
    std::fs::copy(env!("CARGO_BIN_EXE_oteryn-launcher"), &launcher)?;
    for (bytes, expected) in [
        (b"abc", "Verified 1 presentation package"),
        (b"abd", "integrity mismatch"),
    ] {
        std::fs::write(release.join("packages/scene.bin"), bytes)?;
        let output = Command::new(&launcher)
            .current_dir(&fixture.0)
            .arg("--check-install")
            .output()?;
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}
