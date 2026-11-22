use super::*;

#[cfg(unix)]
#[test]
fn shutdown_before_spawn_prevents_helper_side_effect() -> Result<(), Box<dyn std::error::Error>> {
    let (ready, started) = std::sync::mpsc::channel();
    let (resume, wait) = std::sync::mpsc::channel();
    let wait = Mutex::new(wait);
    let workers = Arc::new(BrowserEpoch {
        prepare_for_test: Some(Arc::new(move || {
            ready.send(()).map_err(std::io::Error::other)?;
            wait.lock()
                .map_err(|_| std::io::Error::other("lock"))?
                .recv_timeout(Duration::from_secs(5))
                .map_err(std::io::Error::other)?;
            Ok(std::process::Command::new("true"))
        })),
        ..Default::default()
    });
    let mut entry = admission_with_workers(&workers)?;
    assert!(entry.request());
    started.recv_timeout(Duration::from_secs(5))?;
    assert!(entry.cancel_entry());
    resume.send(())?;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = entry.poll() {
            assert!(result.is_err());
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "admission did not finish"
        );
        std::thread::yield_now();
    }
    assert_eq!(
        workers.launches.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    Ok(())
}

#[cfg(unix)]
fn admission_with_workers(
    workers: &Arc<BrowserEpoch>,
) -> Result<crate::reentry::Reentry, Box<dyn std::error::Error>> {
    let owner = workers.clone();
    let system = SystemBrowser {
        opener: Arc::new(move |url| deliver_system_url(url, &owner, None, None)),
        workers: workers.clone(),
    };
    let mut config = NativeLoginConfig::from_env(|key| match key {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some("http://127.0.0.1:1".into()),
        "OTERYN_OAUTH_CLIENT_ID" => Some("native".into()),
        _ => None,
    })?
    .ok_or("config")?;
    config.bind_system_browser(system);
    let mut client = crate::ClientBootstrap::new()?;
    client.native_login = Some(config);
    Ok(crate::reentry::Reentry::new(client))
}

#[cfg(target_os = "linux")]
#[test]
fn admission_cancellation_during_spawn_returns_and_reaps_helper()
-> Result<(), Box<dyn std::error::Error>> {
    let (ready, started) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let wait = Mutex::new(wait);
    let workers = Arc::new(BrowserEpoch {
        prepare_for_test: Some(Arc::new(|| {
            let mut command = std::process::Command::new("sleep");
            command.arg("30");
            Ok(command)
        })),
        spawned: Some(Arc::new(move |pid| {
            let _ = ready.send(pid);
            if let Ok(wait) = wait.lock() {
                let _ = wait.recv_timeout(Duration::from_secs(5));
            }
        })),
        ..Default::default()
    });
    let mut entry = admission_with_workers(&workers)?;
    assert!(entry.request());
    let pid = started.recv_timeout(Duration::from_secs(5))?;
    let process = PathBuf::from("/proc").join(pid.to_string());
    assert!(process.exists());
    let start = std::time::Instant::now();
    assert!(entry.cancel_entry());
    let elapsed = start.elapsed();
    let _ = release.send(());
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while process.exists() && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(!process.exists(), "cancelled helper was not reaped");
    assert!(elapsed < Duration::from_secs(1), "close waited for spawn");
    Ok(())
}
