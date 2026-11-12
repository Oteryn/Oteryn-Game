use std::path::Path;
use std::process::Command;

pub(super) fn status(client: &Path) -> String {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    // A retained older release may predate this diagnostic flag. Its own identity remains authoritative.
    let release = client
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    if release.and_then(|name| name.split('~').next()) != Some(oteryn_client::RELEASE_ID) {
        return "Use the active release's trusted installer to check or repair its packages."
            .into();
    }
    let Ok(mut child) = Command::new(client)
        .arg("--check-packages")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return "Cannot start the active client's package checker. Repair with the trusted installer.".into();
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return "Package check did not finish. Close the client and use the trusted installer to repair it.".into();
            }
        }
    }
    match child.wait_with_output() {
        Ok(output) => {
            let report = if output.status.success() {
                &output.stdout
            } else {
                &output.stderr
            };
            String::from_utf8_lossy(report).trim().to_owned()
        }
        Err(_) => "Cannot read the package check result. Repair with the trusted installer.".into(),
    }
}
