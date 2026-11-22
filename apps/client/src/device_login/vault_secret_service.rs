//! Store pending revocation credentials in the user's Secret Service collection.
use super::Vault;
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub(super) struct SecretService(pub(super) String);

impl SecretService {
    fn command(&self, operation: &str) -> Command {
        let mut command = Command::new("secret-tool");
        command.arg(operation);
        if operation == "store" {
            command.arg("--label=Oteryn pending device revocation");
        }
        command.args(["application", "Oteryn", "revocation", &self.0]);
        command.stderr(Stdio::null());
        command
    }
}

impl Vault for SecretService {
    fn available(&self) -> bool {
        true
    }

    fn read(&self) -> Result<Option<Vec<u8>>, String> {
        let mut child = self
            .command("lookup")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| error.to_string())?;
        wait(&mut child)?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or_else(|| "Secret Service stdout unavailable".to_owned())?
            .take(50)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.len() != 48 {
            return Err("Secret Service returned an invalid revocation credential".into());
        }
        Ok(Some(bytes))
    }

    fn write(&self, bytes: &[u8]) -> Result<(), String> {
        let mut child = self
            .command("store")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        let write = child
            .stdin
            .take()
            .ok_or_else(|| "Secret Service stdin unavailable".to_owned())
            .and_then(|mut input| input.write_all(bytes).map_err(|error| error.to_string()));
        let status = wait(&mut child);
        write?;
        status
    }

    fn write_process(&self, bytes: &[u8]) -> Result<bool, String> {
        Ok(self.write(bytes).is_ok())
    }

    fn clear(&self) -> Result<(), String> {
        let mut child = self
            .command("clear")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        wait(&mut child)
    }
}

fn wait(child: &mut Child) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err("Secret Service request failed".into())
            };
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Secret Service request timed out".into());
        }
        std::thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}
