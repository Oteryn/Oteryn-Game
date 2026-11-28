//! Live C16d admission capture and client replay.

use super::*;
use std::io::Write;

const REPLAY_TEST: &str = "tests::e2e_c16d::e2e_c16d_item_appearance_snapshot_replay";

pub(super) async fn run_packet(clients: &SeamClients<'_>, _world: &BundleWorld) {
    match verdict::observe(capture_and_replay(clients)).await {
        Ok(()) => evidence(
            "e2e_all scenario=C16d.client status=PASS evidence=live-rune-effects-relog-and-client-replay",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C16d.client status=FAIL evidence={error}"
        )),
    }
}

async fn capture_and_replay(clients: &SeamClients<'_>) -> TestResult {
    let directory = CaptureDirectory::open()?;
    let files = full_e2ef5::tile_item_client_capture(clients).await?;
    for (name, bytes) in files {
        std::fs::File::create_new(directory.path().join(name))?.write_all(&bytes)?;
    }

    let replay = tokio::task::spawn_blocking(move || {
        std::process::Command::new("timeout")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .env("E2E_ALL_CAPTURE_DIR", directory.path())
            .args([
                "80s",
                env!("CARGO"),
                "test",
                "--locked",
                "-p",
                "oteryn-session",
                "--lib",
                REPLAY_TEST,
                "--",
                "--exact",
                "--format",
                "pretty",
            ])
            .output()
    })
    .await??;
    let passed = format!("test {REPLAY_TEST} ... ok");
    if !replay.status.success()
        || !String::from_utf8_lossy(&replay.stdout)
            .lines()
            .any(|line| line == passed)
    {
        return Err(format!("C16d exact Session replay failed: {}", replay.status).into());
    }
    Ok(())
}

enum CaptureDirectory {
    Supplied(std::path::PathBuf),
    Temporary(std::path::PathBuf),
}

impl CaptureDirectory {
    fn open() -> TestResult<Self> {
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            return Ok(Self::Supplied(Path::new(&path).canonicalize()?));
        }
        let output = std::process::Command::new("mktemp")
            .args(["-d", "/tmp/oteryn-c16d-capture.XXXXXXXXXX"])
            .output()?;
        if !output.status.success() {
            return Err(
                format!("C16d capture directory creation failed: {}", output.status).into(),
            );
        }
        let path = std::str::from_utf8(&output.stdout)?.trim_end();
        Ok(Self::Temporary(path.into()))
    }

    fn path(&self) -> &Path {
        match self {
            Self::Supplied(path) | Self::Temporary(path) => path,
        }
    }
}

impl Drop for CaptureDirectory {
    fn drop(&mut self) {
        if let Self::Temporary(path) = self {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
