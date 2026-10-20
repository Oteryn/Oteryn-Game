use crate::foundation::WorldId;
use crate::world_clock::{RetainedWorldConfig, WorldConfigError};
use oteryn_gameplay_node_files::secure_file::FileError;
use oteryn_simulation_determinism::GameplayDecisionRoot;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct SecretFixture(std::path::PathBuf);
impl SecretFixture {
    fn new(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "oteryn-world-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let fixture = Self(directory);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(fixture.seed())?;
        file.write_all(bytes)?;
        Ok(fixture)
    }
    fn seed(&self) -> std::path::PathBuf {
        self.0.join("world-seed")
    }
}
impl Drop for SecretFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn world(last: u8) -> Result<WorldId, Box<dyn std::error::Error>> {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    WorldId::decode(&bytes).map_err(|error| format!("{error:?}").into())
}
fn load(file: &std::path::Path) -> Result<RetainedWorldConfig, WorldConfigError> {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = 1;
    let world = WorldId::decode(&bytes).map_err(|_| WorldConfigError::ClockIdentity)?;
    RetainedWorldConfig::load_checked_secret(world, file, "clock-v1", 18_000_000, "policy-7")
}

#[test]
fn checked_secret_is_retained_and_only_the_bound_world_can_read_it() -> TestResult {
    let file = SecretFixture::new(&[0x39; 32])?;
    let retained = load(&file.seed()).map_err(|error| format!("{error:?}"))?;
    let read = retained.read(world(1)?).ok_or("same World binding")?;
    assert_eq!(read.clock_identity, "clock-v1");
    assert_eq!(read.policy_revision, "policy-7");
    assert_eq!(read.clock.absolute_elapsed_ms(18_900_000), Some(900_000));
    assert!(read.root == &GameplayDecisionRoot::from_bytes([0x39; 32]));
    assert!(retained.read(world(2)?).is_none());
    // Altering the source file after boot cannot reseed the retained World root.
    std::fs::write(file.seed(), [0x72; 32])?;
    assert!(retained.read(world(1)?).ok_or("retained binding")?.root == read.root);
    let debug = format!("{retained:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("GameplayDecisionRoot"));
    Ok(())
}

#[test]
fn secret_shape_permissions_and_symlink_never_admit_a_fallback_seed() -> TestResult {
    for length in [0, 31, 33] {
        let file = SecretFixture::new(&vec![7; length])?;
        assert!(load(&file.seed()).is_err());
    }
    let file = SecretFixture::new(&[7; 32])?;
    std::fs::set_permissions(file.seed(), std::fs::Permissions::from_mode(0o644))?;
    assert!(matches!(
        load(&file.seed()),
        Err(WorldConfigError::SeedFile(FileError::Mode))
    ));
    std::fs::set_permissions(file.seed(), std::fs::Permissions::from_mode(0o600))?;
    let link = file.0.join("link");
    std::os::unix::fs::symlink(file.seed(), &link)?;
    assert!(matches!(
        load(&link),
        Err(WorldConfigError::SeedFile(FileError::SymbolicLink))
    ));
    Ok(())
}

#[test]
fn explicit_clock_and_single_policy_metadata_are_required() -> TestResult {
    let file = SecretFixture::new(&[7; 32])?;
    for (identity, epoch, policy, error) in [
        ("", 18_000_000, "policy-7", WorldConfigError::ClockIdentity),
        (
            "clock-v1",
            18_000_001,
            "policy-7",
            WorldConfigError::ClockEpoch,
        ),
        ("clock-v1", 18_000_000, "", WorldConfigError::PolicyRevision),
    ] {
        assert!(
            matches!(RetainedWorldConfig::load_checked_secret(world(1)?,&file.seed(),identity,epoch,policy),Err(actual) if actual==error)
        );
    }
    Ok(())
}
