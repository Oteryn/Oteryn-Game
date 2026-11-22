use super::*;

#[test]
fn review_3376_second_client_cannot_consume_a_process_session_revocation() -> TestResult {
    process_revocation(false)
}

#[cfg(target_os = "linux")]
#[test]
fn c2a_process_revocation_without_secret_service() -> TestResult {
    process_revocation(true)
}

fn process_revocation(unavailable: bool) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut enrollment: serde_json::Value =
                serde_json::from_slice(&enrollment_response()?)?;
            let deadline = now()? + 3600;
            enrollment["absolute_expires_at"] = deadline.into();
            enrollment["idle_expires_at"] = deadline.into();
            let server = Server::start(vec![
                (
                    200,
                    br#"{"token_type":"Bearer","access_token":"synthetic-access"}"#.to_vec(),
                ),
                (200, serde_json::to_vec(&enrollment)?),
                (200, fixture("revoke-response.json")?),
            ])
            .await?;
            let root = tempfile::tempdir()?;
            let mut a = manager(Arc::new(MemoryVault::default()), root.path().into());
            if unavailable {
                a.journal = Arc::new(revocation_journal::Journal::with_vault(
                    Some(root.path().into()),
                    Arc::new(|reference| vault::system(reference.into())),
                ));
            }
            let b = manager(Arc::new(MemoryVault::default()), root.path().into());
            let mode = MiscellaneousPreferences {
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
                let mut attempt = a
                    .begin(&server.client, mode, CancellationToken::new())
                    .await?
                    .ok_or("attempt")?;
                attempt
                    .enroll(&server.client, &token, CancellationToken::new())
                    .await?;
            }
            a.block_remembered()?;
            drop(
                b.begin(&server.client, mode, CancellationToken::new())
                    .await?,
            );
            a.finish_sign_out(&server.client).await?;
            assert_eq!(
                server.requests.lock().map_err(|_| "requests")?.len(),
                3,
                "the initiating credential must be sent"
            );
            server.finish().await
        })
}

#[test]
fn c2a_picker_dismissal_preserves_remembered_family() -> TestResult {
    picker_dismissal(false)
}

#[test]
fn c2a_explicit_picker_cancel_leaves_revoke_to_sign_out_worker() -> TestResult {
    picker_dismissal(true)
}

fn picker_dismissal(explicit: bool) -> TestResult {
    on_large_stack(move || {
        use crate::native_entry::test_support::{Stack, authorize_selection};
        let stack = Stack::start_two_channels()?;
        tokio::runtime::Runtime::new()?.block_on(async {
            let rotated = changed_response(&credential('A'), false)?;
            let server = Server::start(vec![
                (200, rotated),
                (200, fixture("revoke-response.json")?),
            ])
            .await?;
            let root = tempfile::tempdir()?;
            let vault = Arc::new(MemoryVault::default());
            vault::Vault::write(vault.as_ref(), &enrollment_response()?)
                .map_err(std::io::Error::other)?;
            let mut config = stack.config();
            config.preference_path = Some(root.path().join("preferences.json"));
            let preferences = root.path().join("preferences.session-options.json");
            options().save(&preferences)?;
            config.device_sessions = Arc::new(manager(vault.clone(), root.path().into()));
            config.session_options = Some(options());
            config.require_character_picker = true;
            let device = config.device_sessions.clone();
            config.select_character = Arc::new(move |_, _, _| {
                if explicit {
                    assert!(device.block_remembered().is_ok());
                }
                crate::character_selection::SelectionAction::Cancel
            });
            assert!(authorize_selection(&server.client, &config).await.is_err());
            assert_eq!(server.requests.lock().map_err(|_| "requests")?.len(), 1);
            if explicit {
                config
                    .device_sessions
                    .finish_sign_out(&server.client)
                    .await?;
                server.finish().await?;
            } else {
                assert!(MiscellaneousPreferences::read(&preferences)?.remember_device);
                assert!(
                    vault::Vault::read(vault.as_ref())
                        .map_err(std::io::Error::other)?
                        .is_some()
                );
            }
            Ok(())
        })
    })
}
