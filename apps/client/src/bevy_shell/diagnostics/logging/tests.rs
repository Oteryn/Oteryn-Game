use super::*;
use bevy::log::tracing;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn names(directory: &Path) -> io::Result<Vec<String>> {
    let mut names: Vec<_> = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name != ".lock")
        .collect();
    names.sort();
    Ok(names)
}

#[test]
fn the_log_file_is_created_per_utc_day_under_logs() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    let file = RollingFile::new(directory.clone(), KEEP_FILES, TOTAL_CAP);
    file.write_text(SystemTime::now(), "first line\n")?;
    file.write_dated("2030-01-02", "next day\n")?;
    let today = format!("client-{}.log", Utc::at(SystemTime::now()).date());
    assert_eq!(
        names(&directory)?,
        vec![today.clone(), "client-2030-01-02.log".into()]
    );
    assert_eq!(fs::read_to_string(directory.join(today))?, "first line\n");
    fs::remove_dir_all(&directory)?;
    Ok(())
}

#[test]
fn rotation_keeps_the_newest_seven_files() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    fs::create_dir_all(&directory)?;
    for day in 1..=9 {
        fs::write(
            directory.join(format!("client-2030-01-0{day}.log")),
            "old\n",
        )?;
    }
    fs::write(directory.join("unrelated.txt"), "keep me")?;
    let file = RollingFile::new(directory.clone(), KEEP_FILES, TOTAL_CAP);
    file.write_dated("2030-01-10", "new\n")?;
    let expected: Vec<String> = (4..=9)
        .map(|day| format!("client-2030-01-0{day}.log"))
        .chain(["client-2030-01-10.log".into(), "unrelated.txt".into()])
        .collect();
    assert_eq!(names(&directory)?, expected);
    fs::remove_dir_all(&directory)?;
    assert!(!directory.with_added_extension("lock").exists());
    Ok(())
}

#[test]
fn two_clients_share_the_daily_budget() -> TestResult {
    // The crash ring is process-wide and other tests can evict this warning.
    const CHILD: &str = "OTERYN_TEST_LOG_CONTENTION";
    if std::env::var_os(CHILD).is_none() {
        let fixture = crate::play::screenshot::TestDir::new()?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .env("TMPDIR", fixture.path())
            .args([
                "--exact",
                "bevy_shell::diagnostics::logging::tests::two_clients_share_the_daily_budget",
            ])
            .env(CHILD, "1")
            .output()?;
        crate::play::screenshot::assert_one_test_passed(&output);
        return Ok(());
    }
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    let writers = [(); 2].map(|()| Arc::new(RollingFile::new(directory.clone(), 7, 1000)));
    let date = "2030-01-01";
    writers[0].write_dated(date, "first line\n")?;
    let lock = fs::File::options()
        .read(true)
        .write(true)
        .open(directory.join(".lock"))?;
    lock.lock()?;
    let (finished, wait) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let result = wait.recv_timeout(std::time::Duration::from_secs(1));
        drop(lock);
        result
    });
    tracing::subscriber::with_default(test_subscriber(&writers[1], "info"), || {
        tracing::warn!("contended foreground log");
    });
    assert!(
        crate::crash_report::recent_log_lines(crate::crash_report::RECENT_LINES)
            .iter()
            .any(|line| line.contains("contended foreground log"))
    );
    let log = format!("client-{date}.log");
    assert_eq!(fs::read_to_string(directory.join(&log))?, "first line\n");
    assert_eq!(names(&directory)?, vec![log]);
    let result = writers[1].write_dated(date, "contended line\n");
    finished.send(())?;
    holder.join().map_err(|_| "lock holder panicked")??;
    assert_eq!(
        result.err().map(|error| error.kind()),
        Some(io::ErrorKind::WouldBlock)
    );
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| -> io::Result<()> {
        let mut workers = Vec::new();
        for writer in &writers {
            let barrier = &barrier;
            workers.push(scope.spawn(move || -> io::Result<()> {
                let mut failure = None;
                for _ in 0..100 {
                    barrier.wait();
                    if let Err(error) = writer.write_dated(date, "a shared line of log output\n")
                        && error.kind() != io::ErrorKind::WouldBlock
                    {
                        failure = Some(error);
                    }
                }
                failure.map_or(Ok(()), Err)
            }));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|_| io::Error::other("writer panicked"))??;
        }
        Ok(())
    })?;
    let text = fs::read_to_string(directory.join("client-2030-01-01.log"))?;
    assert!(text.len() <= 1000);
    assert!(text.ends_with(CAPPED));
    fs::remove_dir_all(&directory)?;
    assert!(!directory.with_added_extension("lock").exists());
    Ok(())
}

#[test]
fn writable_log_directory_works_when_sibling_lock_cannot_be_created() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    let logs = directory.join("logs");
    fs::create_dir_all(&logs)?;
    #[cfg(unix)]
    let original = {
        use std::os::unix::fs::PermissionsExt;
        let original = fs::metadata(&directory)?.permissions();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o555))?;
        original
    };
    #[cfg(not(unix))]
    fs::create_dir(logs.with_added_extension("lock"))?;
    let file = Arc::new(RollingFile::new(logs.clone(), KEEP_FILES, TOTAL_CAP));
    tracing::subscriber::with_default(test_subscriber(&file, "info"), || {
        tracing::warn!("daily output");
    });
    let text =
        fs::read_to_string(logs.join(format!("client-{}.log", Utc::at(SystemTime::now()).date())));
    #[cfg(unix)]
    fs::set_permissions(&directory, original)?;
    fs::remove_dir_all(&directory)?;
    assert!(text?.contains("daily output"));
    Ok(())
}

#[test]
fn the_total_size_stays_under_the_cap() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    fs::create_dir_all(&directory)?;
    fs::write(directory.join("client-2030-01-01.log"), vec![b'a'; 300])?;
    fs::write(directory.join("client-2030-01-02.log"), vec![b'b'; 300])?;
    let file = RollingFile::new(directory.clone(), KEEP_FILES, 1_000);
    // A parallel subprocess can briefly inherit the lock before it execs.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    for index in 0..100 {
        let line = format!("line {index:04}\n");
        loop {
            match file.write_dated("2030-01-03", &line) {
                Err(error)
                    if error.kind() == io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::yield_now();
                }
                result => {
                    result?;
                    break;
                }
            }
        }
    }
    // The older file goes so the others use at most half; today stops at the budget.
    assert_eq!(
        names(&directory)?,
        vec!["client-2030-01-02.log", "client-2030-01-03.log"]
    );
    let total: u64 = fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum();
    assert!(total <= 1_000, "{total}");
    let today = fs::read_to_string(directory.join("client-2030-01-03.log"))?;
    assert!(today.starts_with("line 0000\n"));
    assert!(today.ends_with(CAPPED));
    fs::remove_dir_all(&directory)?;
    assert!(!directory.with_added_extension("lock").exists());
    Ok(())
}

#[test]
fn formatted_lines_carry_time_level_target_thread_and_are_redacted() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    let file = Arc::new(RollingFile::new(directory.clone(), KEEP_FILES, TOTAL_CAP));
    let subscriber = test_subscriber(&file, "info");
    std::thread::Builder::new()
        .name("format-test".into())
        .spawn(move || {
            tracing::subscriber::with_default(subscriber, || {
                tracing::warn!(target: "oteryn_test", "window creation failed");
                tracing::warn!(target: "oteryn_test", "login failed password=hunter2\nsecond line");
                tracing::info!(target: "oteryn_test", "recovered");
                tracing::debug!(target: "oteryn_test", "hidden at info");
            });
        })?
        .join()
        .map_err(|_| "logging thread")?;
    let text = fs::read_to_string(
        directory.join(format!("client-{}.log", Utc::at(SystemTime::now()).date())),
    )?;
    let mut lines = text.lines();
    let line = lines.next().ok_or("formatted line")?;
    let stamp = line.split_whitespace().next().ok_or("stamp")?;
    assert!(stamp.ends_with('Z') && stamp.contains('T'), "{line}");
    assert!(line.contains(" WARN "), "{line}");
    assert!(line.contains("format-test"), "{line}");
    assert!(line.contains("oteryn_test:"), "{line}");
    assert_eq!(lines.next(), Some("[record redacted: possible credential]"));
    assert!(lines.next().is_some_and(|line| line.ends_with("recovered")));
    assert_eq!(lines.next(), None);
    assert!(!text.contains("hunter2"));
    assert!(!text.contains("hidden at info"));
    fs::remove_dir_all(&directory)?;
    assert!(!directory.with_added_extension("lock").exists());
    Ok(())
}

#[test]
fn third_party_targets_stay_at_warn_unless_named() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    let file = Arc::new(RollingFile::new(directory.clone(), KEEP_FILES, TOTAL_CAP));
    for (directives, shown) in [("debug", false), ("debug,hyper=debug", true)] {
        tracing::subscriber::with_default(test_subscriber(&file, directives), || {
            tracing::debug!(target: "hyper::client", "hyper detail {directives}");
            tracing::warn!(target: "hyper::client", "hyper warning {directives}");
            tracing::debug!(target: "oteryn_client::play", "client detail {directives}");
        });
        let text = fs::read_to_string(
            directory.join(format!("client-{}.log", Utc::at(SystemTime::now()).date())),
        )?;
        assert_eq!(text.contains(&format!("hyper detail {directives}")), shown);
        assert!(text.contains(&format!("hyper warning {directives}")));
        assert!(text.contains(&format!("client detail {directives}")));
    }
    fs::remove_dir_all(&directory)?;
    assert!(!directory.with_added_extension("lock").exists());
    Ok(())
}

#[test]
fn a_full_log_writer_queue_does_not_block_the_formatter() -> TestResult {
    let (sender, receiver) = mpsc::sync_channel(1);
    let sink = Sink { file: Some(sender), synchronous_file: None, stderr: false, ring: false };
    let (done, finished) = mpsc::channel();
    let _worker = thread::spawn(move || {
        for text in [b"first".as_slice(), b"second"] {
            let mut writer = fmt::MakeWriter::make_writer(&sink);
            writer.write_all(text)?;
        }
        done.send(()).map_err(io::Error::other)
    });
    finished.recv_timeout(std::time::Duration::from_secs(2))?;
    assert_eq!(receiver.try_recv()?.1, "first");
    assert!(receiver.try_recv().is_err());
    Ok(())
}
