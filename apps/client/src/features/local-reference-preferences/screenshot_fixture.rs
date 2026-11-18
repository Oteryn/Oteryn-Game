use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(crate) struct TestDir(tempfile::TempDir);

impl TestDir {
    pub(crate) fn new() -> io::Result<Self> {
        let parent = std::env::temp_dir().canonicalize()?;
        let mut builder = tempfile::Builder::new();
        builder.prefix("oteryn-test-");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        builder.tempdir_in(parent).map(Self)
    }

    #[cfg(bevy_shell)]
    pub(crate) fn keep(self) -> PathBuf {
        self.0.keep()
    }

    pub(crate) fn path(&self) -> &Path {
        self.0.path()
    }

    pub(crate) fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.path().join(name)
    }
}

pub(crate) fn one_test_passed(output: &Output) -> bool {
    output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed;"))
}

pub(crate) fn assert_one_test_passed(output: &Output) {
    assert!(
        one_test_passed(output),
        "Expected one successful child test. Status: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(crate) fn with_relative_capture_path(
    check: impl FnOnce(&str, &Path) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    const CHILD: &str = "OTERYN_TEST_RELATIVE_CAPTURE_CWD";
    let cwd = std::env::current_dir()?.canonicalize()?;
    if std::env::var_os(CHILD).as_deref() == Some(cwd.as_os_str()) {
        let fixture = tempfile::tempdir_in(&cwd)?;
        let target = fixture.path().join("captures");
        return check(target.strip_prefix(&cwd)?.to_str().ok_or("fixture path")?, fixture.path());
    }
    let fixture = TestDir::new()?;
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", std::thread::current().name().ok_or("test name")?])
        .current_dir(fixture.path())
        .env(CHILD, fixture.path())
        .output()?;
    assert_one_test_passed(&output);
    Ok(())
}

#[test]
fn fixture_reservation_leaves_existing_directory_alone() -> io::Result<()> {
    let parent = TestDir::new()?;
    let occupied = parent.join("occupied");
    std::fs::create_dir(&occupied)?;
    std::fs::write(occupied.join("sentinel"), b"untouched")?;
    let collision = tempfile::Builder::new()
        .prefix("occupied")
        .rand_bytes(0)
        .tempdir_in(parent.path());
    assert!(collision.is_err_and(|error| error.kind() == io::ErrorKind::AlreadyExists));
    let reserved = tempfile::tempdir_in(parent.path())?;
    let available = reserved.path().to_path_buf();
    assert_ne!(available, occupied);
    drop(reserved);
    assert!(!available.exists());
    assert_eq!(std::fs::read(occupied.join("sentinel"))?, b"untouched");
    Ok(())
}

#[test]
fn zero_test_subprocess_is_rejected() -> io::Result<()> {
    let fixture = TestDir::new()?;
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", "oteryn_missing_test_target"])
        .current_dir(fixture.path())
        .output()?;
    assert!(output.status.success());
    assert!(!one_test_passed(&output));
    Ok(())
}
