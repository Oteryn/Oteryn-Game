use super::test_server::*;
use super::*;
use std::sync::Mutex;
#[derive(Default)]
struct MemoryVault {
    bytes: Mutex<Option<Vec<u8>>>,
    fail: std::sync::atomic::AtomicBool,
}
impl vault::Vault for MemoryVault {
    fn available(&self) -> bool {
        true
    }
    fn read(&self) -> Result<Option<Vec<u8>>, String> {
        Ok(self.bytes.lock().map_err(|_| "vault")?.clone())
    }
    fn write(&self, b: &[u8]) -> Result<(), String> {
        if self.fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("vault unavailable".into());
        }
        *self.bytes.lock().map_err(|_| "vault")? = Some(b.to_vec());
        Ok(())
    }
    fn clear(&self) -> Result<(), String> {
        *self.bytes.lock().map_err(|_| "vault")? = None;
        Ok(())
    }
}
fn manager(vault: Arc<MemoryVault>, folder: PathBuf) -> DeviceLogin {
    DeviceLogin {
        enabled: true,
        vault,
        state: Default::default(),
        folder: Some(folder),
    }
}
fn folder(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("oteryn-t067-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    Ok(root)
}
fn options() -> MiscellaneousPreferences {
    MiscellaneousPreferences {
        stay_logged_in: true,
        remember_device: true,
        quick_login: true,
        ..Default::default()
    }
}
fn credential(last: char) -> String {
    format!("otd1.{}{last}", "A".repeat(42))
}
#[test]
fn remembered_login_enrolls_restarts_rotates_before_results_and_revokes_over_https() -> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
  let second=credential('A');let third=credential('E');
  let server=Server::start(vec![
   (200,br#"{"token_type":"Bearer","access_token":"synthetic-access","refresh_token":"never-persist-this"}"#.to_vec()),
   (200,enrollment_response()?),
   (200,changed_response(&second,false)?),
   (200,changed_response(&third,true)?),
   (200,fixture("revoke-response.json")?),
  ]).await?;
  let path=folder("restart")?;let vault=Arc::new(MemoryVault::default());
  let first=manager(vault.clone(),path.clone());
  let token=server.client.exchange_code("code","verifier","http://127.0.0.1/callback",CancellationToken::new()).await?;
  {let mut attempt=first.begin(&server.client,options(),CancellationToken::new()).await?.ok_or("attempt")?;
   attempt.enroll(&server.client,&token,CancellationToken::new()).await?;assert!(attempt.has_session());}
  let saved=vault::Vault::read(vault.as_ref()).map_err(std::io::Error::other)?.ok_or("saved")?;
  assert!(!String::from_utf8_lossy(&saved).contains("synthetic-access"));assert!(!String::from_utf8_lossy(&saved).contains("never-persist-this"));
  drop(first);
  let restarted=manager(vault.clone(),path.clone());
  {let mut attempt=restarted.begin(&server.client,options(),CancellationToken::new()).await?.ok_or("restart")?;
   let characters=attempt.characters(&server.client,CancellationToken::new()).await?;
   assert_eq!(characters[0].name,"Fixture Knight");
   let saved=DeviceSession::from_vault(&vault::Vault::read(vault.as_ref()).map_err(std::io::Error::other)?.ok_or("rotation saved")?,now()?)?;
   assert_eq!(saved.device_credential.expose(),second);
   let _ticket=attempt.ticket(&server.client,CancellationToken::new()).await?;
   let saved=DeviceSession::from_vault(&vault::Vault::read(vault.as_ref()).map_err(std::io::Error::other)?.ok_or("ticket saved")?,now()?)?;
   assert_eq!(saved.device_credential.expose(),third);
   attempt.sign_out(&server.client).await;
  }
  assert!(vault::Vault::read(vault.as_ref()).map_err(std::io::Error::other)?.is_none());
  let requests=server.requests.lock().map_err(|_|"requests")?.clone();
  assert_eq!(requests.len(),5);assert!(requests[1].ends_with(std::str::from_utf8(&fixture("enroll.json")?)?));
  assert!(requests[2].contains("/device-sessions/native-characters"));assert!(requests[3].contains(&format!("OterynDevice {second}")));
  assert!(requests[4].contains(&format!("OterynDevice {third}")));
  server.finish().await?;std::fs::remove_dir_all(path)?;Ok(())
 })
}
#[test]
fn failed_rotation_forgets_predecessor_without_retry_and_session_options_are_not_exported()
-> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
        let server = Server::start(vec![
            (401, fixture("denied.json")?),
            (200, fixture("revoke-response.json")?),
        ])
        .await?;
        let path = folder("failure")?;
        let vault = Arc::new(MemoryVault::default());
        vault::Vault::write(vault.as_ref(), &enrollment_response()?)
            .map_err(std::io::Error::other)?;
        let login = manager(vault.clone(), path.clone());
        {
            let mut attempt = login
                .begin(&server.client, options(), CancellationToken::new())
                .await?
                .ok_or("attempt")?;
            assert!(
                attempt
                    .characters(&server.client, CancellationToken::new())
                    .await
                    .is_err()
            );
            assert!(!attempt.has_session());
        }
        assert!(
            vault::Vault::read(vault.as_ref())
                .map_err(std::io::Error::other)?
                .is_none()
        );
        assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 2);
        let prefs = options();
        prefs.save(&path.join("options.json"))?;
        assert_eq!(
            MiscellaneousPreferences::read(&path.join("options.json"))?,
            prefs
        );
        let settings = crate::play::ReferencePreferences {
            miscellaneous: prefs,
            ..Default::default()
        };
        assert!(
            serde_json::to_value(settings)?
                .get("miscellaneous")
                .is_none()
        );
        server.finish().await?;
        std::fs::remove_dir_all(path)?;
        Ok(())
    })
}

#[test]
fn a_successor_that_cannot_be_saved_is_revoked_before_any_result_is_used() -> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
        let successor = credential('A');
        let server = Server::start(vec![
            (200, changed_response(&successor, false)?),
            (200, fixture("revoke-response.json")?),
        ])
        .await?;
        let path = folder("save-failure")?;
        let vault = Arc::new(MemoryVault::default());
        vault::Vault::write(vault.as_ref(), &enrollment_response()?)
            .map_err(std::io::Error::other)?;
        let login = manager(vault.clone(), path.clone());
        {
            let mut attempt = login
                .begin(&server.client, options(), CancellationToken::new())
                .await?
                .ok_or("attempt")?;
            vault.fail.store(true, std::sync::atomic::Ordering::Relaxed);
            assert!(
                attempt
                    .characters(&server.client, CancellationToken::new())
                    .await
                    .is_err()
            );
            assert!(!attempt.has_session());
        }
        assert!(
            vault::Vault::read(vault.as_ref())
                .map_err(std::io::Error::other)?
                .is_none()
        );
        let requests = server.requests.lock().map_err(|_| "requests")?.clone();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].contains(&format!("OterynDevice {successor}")));
        server.finish().await?;
        std::fs::remove_dir_all(path)?;
        Ok(())
    })
}

#[test]
fn process_session_uses_bound_enrollment_keeps_no_vault_secret_and_quick_login_rechecks_availability()
-> TestResult {
    tokio::runtime::Runtime::new()?.block_on(async {
        let deadline = now()? + 43_200;
        let mut enrollment: serde_json::Value = serde_json::from_slice(&enrollment_response()?)?;
        enrollment["absolute_expires_at"] = deadline.into();
        enrollment["idle_expires_at"] = deadline.into();
        let mut rotated: serde_json::Value =
            serde_json::from_slice(&changed_response(&credential('A'), false)?)?;
        rotated["absolute_expires_at"] = deadline.into();
        rotated["idle_expires_at"] = deadline.into();
        let server = Server::start(vec![
            (
                200,
                br#"{"token_type":"Bearer","access_token":"synthetic-access"}"#.to_vec(),
            ),
            (200, serde_json::to_vec(&enrollment)?),
            (200, serde_json::to_vec(&rotated)?),
            (200, fixture("revoke-response.json")?),
        ])
        .await?;
        let path = folder("session")?;
        let vault = Arc::new(MemoryVault::default());
        let login = manager(vault.clone(), path.clone());
        let options = MiscellaneousPreferences {
            remember_device: false,
            ..options()
        };
        let token = server
            .client
            .exchange_code(
                "code",
                "verifier",
                "http://127.0.0.1/callback",
                CancellationToken::new(),
            )
            .await?;
        {
            let mut attempt = login
                .begin(&server.client, options, CancellationToken::new())
                .await?
                .ok_or("attempt")?;
            attempt
                .enroll(&server.client, &token, CancellationToken::new())
                .await?;
        }
        assert!(
            vault::Vault::read(vault.as_ref())
                .map_err(std::io::Error::other)?
                .is_none()
        );
        {
            let mut attempt = login
                .begin(&server.client, options, CancellationToken::new())
                .await?
                .ok_or("same process")?;
            assert!(attempt.has_session());
            let characters = attempt
                .characters(&server.client, CancellationToken::new())
                .await?;
            let mut rows = vec![crate::character_selection::CharacterRow {
                character: characters[0].clone(),
                world_name: "Fixture World".into(),
                channels: vec![],
                available: true,
            }];
            attempt.remember_selection(&characters[0].character_id, None);
            assert!(attempt.quick_selection(&rows).is_some());
            rows[0].available = false;
            assert!(attempt.quick_selection(&rows).is_none());
            attempt.sign_out(&server.client).await;
        }
        let requests = server.requests.lock().map_err(|_| "requests")?.clone();
        let body: serde_json::Value =
            serde_json::from_str(requests[1].split_once("\r\n\r\n").ok_or("body")?.1)?;
        assert_eq!(body["remember_device"], false);
        assert_eq!(body.as_object().ok_or("object")?.len(), 4);
        assert_eq!(body["device_public_key"].as_str().ok_or("key")?.len(), 43);
        for entry in std::fs::read_dir(&path)? {
            let bytes = std::fs::read(entry?.path())?;
            assert!(!String::from_utf8_lossy(&bytes).contains("otd1."));
        }
        server.finish().await?;
        std::fs::remove_dir_all(path)?;
        Ok(())
    })
}
