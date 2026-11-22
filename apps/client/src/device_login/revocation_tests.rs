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
                journal: super::test_server::journal(None),
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
                journal: super::test_server::journal(None),
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
        journal: super::test_server::journal(Some(folder.path().into())),
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

#[test]
fn queued_revocation_survives_shutdown_before_the_worker_runs() -> TestResult {
    let folder = tempfile::tempdir()?;
    let vault = Arc::new(Vault::default());
    vault::Vault::write(vault.as_ref(), &enrollment_response()?).map_err(std::io::Error::other)?;
    let login = || DeviceLogin {
        enabled: true,
        forget_requested: Default::default(),
        journal: super::test_server::journal(Some(folder.path().into())),
        vault: vault.clone(),
        state: Default::default(),
        folder: Some(folder.path().into()),
    };
    let first = Arc::new(login());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let held = runtime.block_on(first.state.clone().lock_owned());
    let attempted = runtime.block_on(async {
        let server = Server::start(Vec::new()).await?;
        first.block_remembered()?;
        let first = first.clone();
        Ok::<_, Box<dyn std::error::Error>>(tokio::spawn(async move {
            first.finish_sign_out(&server.client).await
        }))
    })?;
    let marker = folder.path().join("revoke.pending");
    assert!(marker.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&marker)?.permissions().mode() & 0o777,
            0o600
        );
    }
    drop(runtime);
    assert!(attempted.is_finished());
    drop(held);
    assert!(
        vault::Vault::read(vault.as_ref())
            .map_err(std::io::Error::other)?
            .is_some()
    );

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = Server::start(vec![(200, fixture("revoke-response.json")?)]).await?;
            let restarted = login();
            let attempt = restarted
                .begin(
                    &server.client,
                    MiscellaneousPreferences {
                        stay_logged_in: true,
                        remember_device: true,
                        ..Default::default()
                    },
                    CancellationToken::new(),
                )
                .await?
                .ok_or("device attempt")?;
            assert!(
                !attempt.has_session(),
                "an unprocessed sign-out must fence remembered admission"
            );
            assert!(
                vault::Vault::read(vault.as_ref())
                    .map_err(std::io::Error::other)?
                    .is_none()
            );
            assert!(!marker.exists());
            assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 1);
            server.finish().await
        })
}

#[test]
fn rejected_platform_revocation_is_retried_after_restart() -> TestResult {
    let folder = tempfile::tempdir()?;
    let vault = Arc::new(Vault::default());
    let enrolled = enrollment_response()?;
    vault::Vault::write(vault.as_ref(), &enrolled).map_err(std::io::Error::other)?;
    let login = || DeviceLogin {
        enabled: true,
        forget_requested: Default::default(),
        journal: super::test_server::journal(Some(folder.path().into())),
        vault: vault.clone(),
        state: Default::default(),
        folder: Some(folder.path().into()),
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let failed = Server::start(vec![(500, Vec::new())]).await?;
            assert!(login().sign_out(&failed.client).await.is_err());
            failed.finish().await?;
            assert!(folder.path().join("revoke.pending").is_file());

            let restored = Server::start(vec![(200, fixture("revoke-response.json")?)]).await?;
            let attempt = login()
                .begin(
                    &restored.client,
                    MiscellaneousPreferences {
                        stay_logged_in: true,
                        remember_device: true,
                        ..Default::default()
                    },
                    CancellationToken::new(),
                )
                .await?;
            assert!(!attempt.is_some_and(|attempt| attempt.has_session()));
            let credential = DeviceSession::from_vault(&enrolled, now()?)?.device_credential;
            assert!(
                restored
                    .requests
                    .lock()
                    .map_err(|_| "requests")?
                    .iter()
                    .any(|request| request.contains(credential.expose()))
            );
            assert!(!folder.path().join("revoke.pending").exists());
            restored.finish().await
        })
}

#[test]
fn review_3376_exit_after_active_vault_clear_replays_without_token_in_marker() -> TestResult {
    let folder = tempfile::tempdir()?;
    let vault = Arc::new(Vault::default());
    let enrolled = enrollment_response()?;
    vault::Vault::write(vault.as_ref(), &enrolled).map_err(std::io::Error::other)?;
    let login = || DeviceLogin {
        enabled: true,
        forget_requested: Default::default(),
        journal: super::test_server::journal(Some(folder.path().into())),
        vault: vault.clone(),
        state: Default::default(),
        folder: Some(folder.path().into()),
    };
    let credential = DeviceSession::from_vault(&enrolled, now()?)?.device_credential;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (_release, blocked) = tokio::sync::oneshot::channel();
    let attempt = runtime.block_on(async {
        let server =
            Server::start_with_blocked_ack(fixture("revoke-response.json")?, blocked).await?;
        let requests = server.requests.clone();
        let first = login();
        let attempt = tokio::spawn(async move { first.sign_out(&server.client).await });
        tokio::time::timeout(Duration::from_secs(2), async {
            while requests.lock().map_err(|_| "requests")?.is_empty() {
                tokio::task::yield_now().await;
            }
            Ok::<_, &str>(())
        })
        .await??;
        assert!(
            vault::Vault::read(vault.as_ref())
                .map_err(std::io::Error::other)?
                .is_none()
        );
        assert!(folder.path().join("revoke.pending").exists());
        for file in std::fs::read_dir(folder.path())? {
            let bytes = std::fs::read(file?.path())?;
            assert!(
                !bytes
                    .windows(credential.expose().len())
                    .any(|window| window == credential.expose().as_bytes())
            );
        }
        Ok::<_, Box<dyn std::error::Error>>(attempt)
    })?;
    drop(runtime);
    assert!(attempt.is_finished());
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = Server::start(vec![(200, fixture("revoke-response.json")?)]).await?;
            drop(
                login()
                    .begin(
                        &server.client,
                        MiscellaneousPreferences::default(),
                        CancellationToken::new(),
                    )
                    .await?,
            );
            assert!(
                server
                    .requests
                    .lock()
                    .map_err(|_| "requests")?
                    .iter()
                    .any(|request| request.contains(credential.expose()))
            );
            assert!(!folder.path().join("revoke.pending").exists());
            server.finish().await
        })
}
