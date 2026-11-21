use super::test_server::*;
use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct Vault {
    bytes: Mutex<Option<Vec<u8>>>,
    denied_deletes: AtomicUsize,
}

impl vault::Vault for Vault {
    fn available(&self) -> bool {
        true
    }

    fn read(&self) -> Result<Option<Vec<u8>>, String> {
        Ok(self.bytes.lock().map_err(|_| "vault")?.clone())
    }

    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        *self.bytes.lock().map_err(|_| "vault")? = Some(bytes.to_vec());
        Ok(())
    }

    fn clear(&self) -> Result<(), String> {
        if self
            .denied_deletes
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
            .is_ok()
        {
            return Err("delete refused".into());
        }
        *self.bytes.lock().map_err(|_| "vault")? = None;
        Ok(())
    }
}

#[test]
fn failed_vault_delete_still_revokes_and_never_reuses_the_session() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = Server::start(vec![(200, fixture("revoke-response.json")?)]).await?;
            let vault = Arc::new(Vault::default());
            vault::Vault::write(vault.as_ref(), &enrollment_response()?)
                .map_err(std::io::Error::other)?;
            vault.denied_deletes.store(1, Ordering::Relaxed);
            let login = DeviceLogin {
                enabled: true,
                forget_requested: Default::default(),
                vault,
                state: Default::default(),
                folder: None,
            };
            let attempt = login
                .begin(
                    &server.client,
                    MiscellaneousPreferences {
                        stay_logged_in: false,
                        remember_device: false,
                        ..Default::default()
                    },
                    CancellationToken::new(),
                )
                .await?;
            assert!(!attempt.is_some_and(|attempt| attempt.has_session()));
            assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 1);
            server.finish().await
        })
}

#[test]
fn sign_out_retries_every_pending_family() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = Server::start(vec![
                (500, Vec::new()),
                (200, fixture("revoke-response.json")?),
                (200, fixture("revoke-response.json")?),
            ])
            .await?;
            let vault = Arc::new(Vault::default());
            let first = enrollment_response()?;
            let mut second: serde_json::Value = serde_json::from_slice(&first)?;
            let second_credential = format!("otd1.{}E", "A".repeat(42));
            second["device_credential"] = second_credential.clone().into();
            vault::Vault::write(vault.as_ref(), &first).map_err(std::io::Error::other)?;
            let login = DeviceLogin {
                enabled: true,
                forget_requested: Default::default(),
                vault: vault.clone(),
                state: Default::default(),
                folder: None,
            };
            assert!(login.sign_out(&server.client).await.is_err());
            vault::Vault::write(vault.as_ref(), &serde_json::to_vec(&second)?)
                .map_err(std::io::Error::other)?;
            login.sign_out(&server.client).await?;
            let requests = server.requests.lock().map_err(|_| "requests")?.clone();
            assert_eq!(requests.len(), 3);
            let original = DeviceSession::from_vault(&first, now()?)?;
            assert!(
                requests[1..]
                    .iter()
                    .any(|request| request.contains(original.device_credential.expose()))
            );
            assert!(
                requests[1..]
                    .iter()
                    .any(|request| request.contains(&second_credential))
            );
            server.finish().await
        })
}

#[cfg(target_os = "linux")]
#[test]
fn credential_fifo_is_rejected_within_the_lock_deadline() -> TestResult {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("credential.lock");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()?
            .success()
    );
    let login = DeviceLogin {
        enabled: true,
        forget_requested: Default::default(),
        vault: Arc::new(Vault::default()),
        state: Default::default(),
        folder: Some(folder.path().into()),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(2), login.lock_credential()).await
    });
    // A regression can leave a blocking open in the pool. A temporary reader
    // releases it without an unbounded join during test teardown.
    let _unblock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;
    assert!(result?.is_err());
    Ok(())
}
