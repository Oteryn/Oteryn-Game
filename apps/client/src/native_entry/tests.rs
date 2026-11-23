use super::test_support::Stack;
use super::*;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_session_tcp::connection::connect_any;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn temporarily_unavailable_entry_can_be_repeated_on_the_same_runtime() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start(vec![1115])?;
                let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = crate::reentry::Reentry::new(client);
                for _ in 0..2 {
                    assert!(entry.request());
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    let result = loop {
                        if let Some(result) = entry.poll() {
                            break result;
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err("entry timed out".into());
                        }
                        std::thread::yield_now();
                    };
                    assert_eq!(
                        result.err(),
                        Some(GameplayEntryError::Rejected(
                            PublicClass::TemporarilyUnavailable
                        ))
                    );
                }
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 4);
                assert_eq!(
                    super::test_support::count(&stack.requests(), "POST /api/v1/game-auth/tickets"),
                    2
                );
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "refused entry test thread failed")?
        .map_err(Into::into)
}

#[test]
fn same_client_enters_again_after_server_drops_the_session() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack =
                    Stack::start_with_frames(vec![], super::test_support::admitted_frames())?;
                let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = crate::reentry::Reentry::new(client);
                for _ in 0..2 {
                    assert!(entry.request());
                    assert!(!entry.request());
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    let (mut view, link) = loop {
                        if let Some(result) = entry.poll() {
                            break result?;
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err("entry timed out".into());
                        }
                        std::thread::yield_now();
                    };
                    assert_eq!(link.party_panel().own_character_name, "Ari");
                    assert_eq!(view.preference_character(), parse_uuid_v7(&stack.config().character_id), "preference scope comes from the admitted selected character, even without a spatial entity capability");
                    loop {
                        if let Err(end) = view.tick(&link) {
                            assert_eq!(
                                end,
                                crate::play::PlayEnd::Ended(PublicClass::SessionUnavailable)
                            );
                            break;
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err("session end timed out".into());
                        }
                        std::thread::yield_now();
                    }
                    drop(link);
                    entry.leave_play();
                }
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
                for target in [
                    "GET /v1/client/directory",
                    "POST /oauth/token",
                    "POST /api/v1/game-auth/tickets",
                    "POST /v1/login",
                ] {
                    assert_eq!(
                        super::test_support::count(&stack.requests(), target),
                        2,
                        "{target}"
                    );
                }
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "re-entry test thread failed")?
        .map_err(Into::into)
}

#[test]
fn an_unreachable_first_address_falls_through_to_the_next() -> TestResult {
    let stack = Stack::start(vec![1100])?;
    let root = CertificateDer::from(std::fs::read(&stack.root_path)?);
    let dead = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?;
    let live = SocketAddr::from(([127, 0, 0, 1], stack.game_port));
    let connected = stack.servers.block_on(async {
        (
            connect_any(&[dead], "localhost", &root).await.is_some(),
            connect_any(&[dead, live], "localhost", &root)
                .await
                .is_some(),
        )
    })?;
    stack.finish();
    assert_eq!(connected, (false, true));
    Ok(())
}

#[test]
fn configured_character_is_only_a_preselect() -> TestResult {
    let stack = Stack::start(vec![1105])?;
    let mut config = stack.config();
    config.world_ref.clear();
    config.character_id.clear();
    assert_eq!(
        stack.entry(config)?,
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
    );
    let requests = stack.requests();
    let owner = requests
        .iter()
        .position(|r| r.starts_with("GET /api/v1/game-auth/native-characters"))
        .ok_or("owner read")?;
    let ticket = requests
        .iter()
        .position(|r| r.starts_with("POST /api/v1/game-auth/tickets"))
        .ok_or("ticket")?;
    assert!(owner < ticket);
    let gateway = requests
        .iter()
        .find(|r| r.starts_with("POST /v1/login"))
        .ok_or("gateway")?;
    assert!(gateway.contains("01890a5d-ac96-774b-bcce-b302099a8058"));
    assert!(gateway.contains(r#""channel_id":"01890a5d-ac96-774b-bcce-b302099a805a""#));
    stack.finish();
    Ok(())
}

#[test]
fn terminal_refusal_is_its_public_class_without_retry() -> TestResult {
    // 1105 ADMISSION_GRANT_REPLAYED: SECURITY_TERMINAL, SESSION_UNAVAILABLE.
    let stack = Stack::start(vec![1105])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
    );
    let requests = stack.requests();
    for target in [
        "GET /v1/client/directory",
        "POST /oauth/token",
        "POST /api/v1/game-auth/tickets",
        "POST /v1/login",
    ] {
        assert_eq!(super::test_support::count(&requests, target), 1, "{target}");
    }
    let token = requests
        .iter()
        .find(|request| request.starts_with("POST /oauth/token"))
        .cloned()
        .unwrap_or_default();
    assert!(token.contains("code=code-1"));
    assert!(token.contains("code_verifier="));
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
    stack.finish();
    Ok(())
}

#[test]
fn not_yet_valid_retries_the_same_grant_three_times() -> TestResult {
    let stack = Stack::start(vec![1103])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable)
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 4);
    assert_eq!(
        super::test_support::count(&stack.requests(), "POST /v1/login"),
        1
    );
    stack.finish();
    Ok(())
}

#[test]
fn capacity_exceeded_retries_once() -> TestResult {
    let stack = Stack::start(vec![1115])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable)
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
    stack.finish();
    Ok(())
}

#[test]
fn reconciliation_repeats_the_gateway_login_with_the_same_attempt() -> TestResult {
    let stack = Stack::start(vec![1106, 1105])?;
    assert_eq!(
        stack.entry(stack.config())?,
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
    );
    let logins = stack
        .requests()
        .into_iter()
        .filter(|request| request.starts_with("POST /v1/login"))
        .collect::<Vec<_>>();
    assert_eq!(logins.len(), 2);
    assert_eq!(logins.first(), logins.get(1));
    assert_eq!(
        super::test_support::count(&stack.requests(), "POST /api/v1/game-auth/tickets"),
        1
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
    stack.finish();
    Ok(())
}

#[test]
fn configuration_is_read_only_when_the_platform_is_set() -> TestResult {
    assert!(NativeLoginConfig::from_env(|_name| None)?.is_none());
    let partial = NativeLoginConfig::from_env(|name| {
        (name == "OTERYN_PLATFORM_URL").then(|| "https://platform.test/".to_owned())
    });
    assert_eq!(
        partial.err(),
        Some(NativeLoginSettingError("OTERYN_GATEWAY_URL"))
    );
    let config = NativeLoginConfig::from_env(|name| {
        (name != "OTERYN_DEV_OAUTH_CALLBACK_PORT").then(|| format!("{name}-value"))
    })?
    .ok_or("configured")?;
    assert_eq!(config.character_id, "OTERYN_CHARACTER_ID-value");
    assert!(!format!("{config:?}").contains("OTERYN_OAUTH_CLIENT_ID"));
    Ok(())
}

#[test]
fn registered_oauth_callback_port_is_opt_in_and_loopback_only() -> TestResult {
    let config = |platform: &str, port: Option<&str>| {
        NativeLoginConfig::from_env(|name| match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(platform.to_owned()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("qualification".into()),
            "OTERYN_DEV_OAUTH_CALLBACK_PORT" => port.map(str::to_owned),
            _ => None,
        })
    };
    let loopback = "http://127.0.0.1:28280";
    assert_eq!(
        config(loopback, None)?.ok_or("configured")?.callback_port,
        0
    );
    assert_eq!(
        config(loopback, Some("28299"))?
            .ok_or("configured")?
            .callback_port,
        28299
    );
    assert_eq!(
        config(loopback, Some(""))?
            .ok_or("configured")?
            .callback_port,
        0
    );
    for (origin, port) in [
        (loopback, "0"),
        (loopback, "65536"),
        ("https://example.invalid", "28299"),
    ] {
        assert_eq!(
            config(origin, Some(port)).err(),
            Some(NativeLoginSettingError("OTERYN_DEV_OAUTH_CALLBACK_PORT"))
        );
    }
    Ok(())
}

#[test]
fn login_form_drives_existing_browser_gateway_and_admission() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                use crate::native_login::{LoginForm, LoginSettings};
                let stack = Stack::start_with_frames(Vec::new(), test_support::admitted_frames())?;
                let config = stack.config();
                let mut form = LoginForm::new(LoginSettings::from_values([
                    config.platform_url.clone(),
                    config.gateway_url.clone(),
                    config.oauth_client_id.clone(),
                    config.world_ref.clone(),
                    config.character_id.clone(),
                    config.dev_root.to_string_lossy().into_owned(),
                ]));
                let mut submitted = form.submit()?;
                // The fixture browser follows the real loopback OAuth callback, without launching a desktop browser.
                submitted.open_browser = config.open_browser;
                submitted.select_character = config.select_character;
                submitted.retry_unit = config.retry_unit;
                let mut entry = crate::reentry::Reentry::new(crate::ClientBootstrap::new()?);
                assert!(entry.configure(submitted.clone()));
                assert!(entry.request());
                assert!(!entry.configure(submitted));
                assert!(!entry.request());
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                loop {
                    if let Some(result) = entry.poll() {
                        form.finish(result.as_ref().map(|_| ()).map_err(Clone::clone));
                        assert!(result.is_ok());
                        break;
                    }
                    if std::time::Instant::now() >= deadline {
                        return Err("form admission timed out".into());
                    }
                    std::thread::yield_now();
                }
                assert_eq!(form.message(), "Connected.");
                assert_eq!(
                    test_support::count(&stack.requests(), "POST /api/v1/game-auth/tickets"),
                    1
                );
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "login form scenario thread failed")?
        .map_err(Into::into)
}

#[test]
fn confirmed_gateway_update_refusal_reaches_the_login_form() -> TestResult {
    use crate::native_login::{LoginForm, LoginSettings};
    use oteryn_platform_client::{native_login::ReleaseIdentity, updates::UpdateChannel};

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start(vec![])?;
                let config = stack.config();
                let release = ReleaseIdentity::new(UpdateChannel::Stable, "fixture-release", 4)
                    .ok_or("fixture release identity")?;
                let url = "https://oteryn.com/download";
                stack.refuse_login(409, r#"{"protocol_version":2,"attempt_ref":null,"error":{"code":"NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED","public_class":"CLIENT_UPDATE_REQUIRED"},"update":{"update_channel":"stable","minimum_release_sequence":5,"current_release_sequence":6,"manual_download_url":"https://oteryn.com/download"}}"#)?;
                let mut form = LoginForm::new(LoginSettings::from_values([
                    config.platform_url.clone(),
                    config.gateway_url.clone(),
                    config.oauth_client_id.clone(),
                    config.world_ref.clone(),
                    config.character_id.clone(),
                    config.dev_root.to_string_lossy().into_owned(),
                ]));
                let mut submitted = form.submit()?;
                submitted.open_browser = config.open_browser;
                submitted.select_character = config.select_character;
                submitted.retry_unit = config.retry_unit;
                submitted.preference_path = config.preference_path;
                submitted.device_sessions = config.device_sessions;
                let result = stack.servers.block_on(async {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        crate::build_info::LOGIN_RELEASE_FIXTURE.scope(
                            (submitted.gateway_url.clone(), release),
                            super::enter(&submitted, CancellationToken::new()),
                        ),
                    ).await
                })??;
                let error = result.err().ok_or("build gate admitted an old client")?;
                assert_eq!(error, GameplayEntryError::ClientUpdateRequired {
                    manual_download_url: url.to_owned(),
                    updater_started: false,
                });
                form.finish(Err(error));
                assert!(form.message().contains(url));
                assert!(form.submit().is_err());
                let requests = stack.requests();
                let mut logins = requests.iter().filter_map(|request| request.strip_prefix("POST /v1/login "));
                let request: serde_json::Value = serde_json::from_str(logins.next().ok_or("missing gateway login")?)?;
                assert_eq!(request["offer"]["release"]["release_id"], "fixture-release");
                assert!(logins.next().is_none());
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 0);
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "build gate scenario thread failed")?
        .map_err(Into::into)
}

#[test]
fn empty_account_create_and_refresh_never_issue_a_ticket() -> TestResult {
    for suffix in ["", "?source=game#login"] {
        empty_account_create_and_refresh_case(suffix)?;
    }
    Ok(())
}

fn empty_account_create_and_refresh_case(suffix: &str) -> TestResult {
    use crate::character_selection::SelectionAction;
    use std::sync::{Arc, atomic::AtomicUsize};
    let stack = Stack::start_with_account(vec![], vec![], true)?;
    let mut config = stack.config();
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    config.select_character = Arc::new(move |rows, _, _| {
        assert!(rows.is_empty());
        match seen.fetch_add(1, Ordering::SeqCst) {
            0 => SelectionAction::Create,
            1 => SelectionAction::Refresh,
            _ => SelectionAction::Cancel,
        }
    });
    let opener = config.open_browser.clone();
    let create_url = format!("{}account/characters/create", config.platform_url);
    config.platform_url.push_str(suffix);
    let opened = Arc::new(AtomicUsize::new(0));
    let created = opened.clone();
    config.open_browser = Arc::new(move |url| {
        if url == create_url {
            created.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            opener(url)
        }
    });
    assert_eq!(
        stack.entry(config)?,
        GameplayEntryError::Rejected(PublicClass::RetryLogin)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    let requests = stack.requests();
    assert_eq!(
        test_support::count(&requests, "GET /api/v1/game-auth/native-characters"),
        3
    );
    assert_eq!(test_support::count(&requests, "POST /oauth/token"), 1);
    assert_eq!(
        test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
        0
    );
    stack.finish();
    Ok(())
}

#[test]
fn selection_cannot_enter_an_unowned_character_or_channel() -> TestResult {
    use crate::character_selection::SelectionAction;
    use std::sync::Arc;
    for channel_id in [
        None,
        Some("01890a5d-ac96-774b-bcce-b302099a8000".to_owned()),
    ] {
        let stack = Stack::start(vec![])?;
        let mut config = stack.config();
        config.select_character = Arc::new(move |rows, _, _| SelectionAction::Play {
            character_id: if channel_id.is_none() {
                "01890a5d-ac96-774b-bcce-b302099a8000".into()
            } else {
                rows[0].character.character_id.clone()
            },
            channel_id: channel_id.clone(),
        });
        assert_eq!(
            stack.entry(config)?,
            GameplayEntryError::Rejected(PublicClass::RetryLogin)
        );
        assert_eq!(
            test_support::count(&stack.requests(), "POST /api/v1/game-auth/tickets"),
            0
        );
        stack.finish();
    }
    Ok(())
}

#[test]
fn missing_gameplay_root_does_not_preempt_gateway_refusal() -> TestResult {
    let stack = Stack::start(vec![])?;
    let config = stack.config();
    std::fs::remove_file(&config.dev_root)?;
    // An old build gets the existing refusal code when the gate is enforcing.
    stack.refuse_login(409, r#"{"protocol_version":2,"attempt_ref":null,"error":{"code":"NATIVE_LOGIN_OFFER_UNSUPPORTED","public_class":"CLIENT_UPDATE_REQUIRED"}}"#)?;
    let result = stack.entry(config)?;
    let requests = stack.requests();
    assert_eq!(test_support::count(&requests, "POST /v1/login"), 1);
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 0);
    assert_eq!(
        result,
        GameplayEntryError::Rejected(PublicClass::ClientUpdateRequired)
    );
    stack.finish();
    Ok(())
}

#[test]
fn c2d_picker_cancel_persists_opt_out_after_lock_timeout() -> TestResult {
    use crate::{character_selection::SelectionAction, play::MiscellaneousPreferences};
    use std::{sync::Arc, time::Duration};
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("preferences.json");
    let sidecar = path.with_extension("session-options.json");
    MiscellaneousPreferences {
        remember_device: true,
        ..Default::default()
    }
    .save(&sidecar)?;
    let stack = Stack::start_with_account(vec![], vec![], true)?;
    let mut config = stack.config();
    config.preference_path = Some(path);
    let held = crate::atomic_file::lock(&sidecar)?;
    let (cancel, release) = std::sync::mpsc::channel();
    let unlock = std::thread::spawn(move || {
        if release.recv_timeout(Duration::from_secs(5)).is_ok() {
            std::thread::sleep(Duration::from_millis(1300));
        }
        drop(held);
    });
    let picker_config = config.clone();
    config.select_character = Arc::new(move |_, _, _| {
        assert!(cancel.send(()).is_ok());
        assert!(picker_config.record_picker_opt_out().is_ok());
        SelectionAction::Cancel
    });
    assert_eq!(
        stack.entry(config)?,
        GameplayEntryError::Rejected(PublicClass::RetryLogin)
    );
    unlock.join().map_err(|_| "lock holder panicked")?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while MiscellaneousPreferences::read(&sidecar)?.remember_device {
        assert!(
            std::time::Instant::now() < deadline,
            "opt-out retry timed out"
        );
        std::thread::yield_now();
    }
    assert!(!MiscellaneousPreferences::read(&sidecar)?.remember_device);
    stack.finish();
    Ok(())
}
