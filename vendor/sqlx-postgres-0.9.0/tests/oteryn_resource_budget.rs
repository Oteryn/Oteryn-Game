#![cfg(target_os = "linux")]

use std::error::Error;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

const SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 4, 210, 22, 47];
const DENIAL_MARKER: &str = "OTERYN_WP3_TLS_DENIAL";
const POSITIVE_MARKER: &str = "OTERYN_WP3_TLS_POSITIVE";
const SERVER_MARKER: &str = "OTERYN_WP3_TLS13_SERVER_OK";

const HELPER_LOCK_ENTRY: &str = r#"
[[package]]
name = "oteryn-wp3-aws-lc-qualification"
version = "0.0.0"
dependencies = [
 "sqlx",
 "tokio",
]
"#;

const HELPER_SOURCE: &str = r#"
use sqlx::postgres::{BudgetError, PgConnectOptions, PgConnection, PgSslMode, ResourceBudget};
use std::error::Error;
use std::sync::{Arc, Mutex};

const SLOT_LIMIT: usize = 4_194_304;
const ROOT_LIMIT: usize = 12_582_912;
const DENIAL_ROOT_LIMIT: usize = 4_096;

#[derive(Clone, Copy, Default)]
struct Snapshot {
    ordinary: usize,
    root: usize,
    peak_ordinary: usize,
    peak_root: usize,
    provider_shared: usize,
}

#[derive(Default)]
struct State {
    ordinary: usize,
    root: usize,
    peak_ordinary: usize,
    peak_root: usize,
    provider_shared: usize,
}

struct Ledger {
    ordinary_limit: usize,
    root_limit: usize,
    state: Mutex<State>,
}

impl Ledger {
    fn new(ordinary_limit: usize, root_limit: usize) -> Self {
        Self {
            ordinary_limit,
            root_limit,
            state: Mutex::new(State::default()),
        }
    }

    fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().unwrap();
        Snapshot {
            ordinary: state.ordinary,
            root: state.root,
            peak_ordinary: state.peak_ordinary,
            peak_root: state.peak_root,
            provider_shared: state.provider_shared,
        }
    }
}

impl ResourceBudget for Ledger {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        let Ok(mut state) = self.state.lock() else {
            return Err(BudgetError::Unavailable);
        };
        let ordinary = state
            .ordinary
            .checked_add(bytes)
            .ok_or(BudgetError::Overflow)?;
        let root = state.root.checked_add(bytes).ok_or(BudgetError::Overflow)?;
        if ordinary > self.ordinary_limit || root > self.root_limit {
            return Err(BudgetError::Unavailable);
        }
        state.ordinary = ordinary;
        state.root = root;
        state.peak_ordinary = state.peak_ordinary.max(ordinary);
        state.peak_root = state.peak_root.max(root);
        Ok(())
    }

    fn release(&self, bytes: usize) {
        let Ok(mut state) = self.state.lock() else {
            std::process::abort();
        };
        let Some(ordinary) = state.ordinary.checked_sub(bytes) else {
            std::process::abort();
        };
        let Some(root) = state.root.checked_sub(bytes) else {
            std::process::abort();
        };
        state.ordinary = ordinary;
        state.root = root;
    }

    fn try_reserve_provider_shared(&self, bytes: usize) -> Result<(), BudgetError> {
        let Ok(mut state) = self.state.lock() else {
            return Err(BudgetError::Unavailable);
        };
        let root = state.root.checked_add(bytes).ok_or(BudgetError::Overflow)?;
        if root > self.root_limit {
            return Err(BudgetError::Unavailable);
        }
        let provider_shared = state
            .provider_shared
            .checked_add(bytes)
            .ok_or(BudgetError::Overflow)?;
        state.root = root;
        state.provider_shared = provider_shared;
        state.peak_root = state.peak_root.max(root);
        Ok(())
    }
}

fn options(port: u16) -> PgConnectOptions {
    PgConnectOptions::new()
        .host("127.0.0.1")
        .port(port)
        .username("owner-test")
        .ssl_mode(PgSslMode::Require)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::var("OTERYN_WP3_MODE")?;
    let port: u16 = std::env::var("OTERYN_WP3_PORT")?.parse()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    match mode.as_str() {
        "deny" => {
            let ledger = Arc::new(Ledger::new(SLOT_LIMIT, DENIAL_ROOT_LIMIT));
            let owner: Arc<dyn ResourceBudget> = ledger.clone();
            let result = runtime.block_on(PgConnection::establish_with_resource_budget(
                &options(port),
                owner,
            ));
            if result.is_ok() {
                return Err("underfunded owner unexpectedly established TLS".into());
            }
            let snapshot = ledger.snapshot();
            if snapshot.peak_ordinary > SLOT_LIMIT || snapshot.peak_root > DENIAL_ROOT_LIMIT {
                return Err("denial exceeded the caller-supplied budget".into());
            }
            println!(
                "{DENIAL_MARKER} peak_ordinary={} peak_root={} provider_shared={}",
                snapshot.peak_ordinary, snapshot.peak_root, snapshot.provider_shared
            );
        }
        "positive" => {
            let ledger = Arc::new(Ledger::new(SLOT_LIMIT, ROOT_LIMIT));
            let owner: Arc<dyn ResourceBudget> = ledger.clone();
            let connection = runtime.block_on(PgConnection::establish_with_resource_budget(
                &options(port),
                owner,
            ))?;
            drop(connection);
            let snapshot = ledger.snapshot();
            if snapshot.peak_ordinary == 0 || snapshot.peak_ordinary > SLOT_LIMIT {
                return Err("ordinary TLS custody was not bounded by the slot ledger".into());
            }
            if snapshot.provider_shared == 0 || snapshot.peak_root > ROOT_LIMIT {
                return Err("AWS-LC provider residency was not charged to the same root ledger".into());
            }
            if snapshot.ordinary != 0 || snapshot.root < snapshot.provider_shared {
                return Err("connection teardown released custody before or after the wrong lifetime".into());
            }
            println!(
                "{POSITIVE_MARKER} peak_ordinary={} peak_root={} provider_shared={} retained_root={}",
                snapshot.peak_ordinary,
                snapshot.peak_root,
                snapshot.provider_shared,
                snapshot.root
            );
        }
        other => return Err(format!("unknown qualification mode: {other}").into()),
    }

    Ok(())
}
"#;

const TLS_SERVER_SOURCE: &str = r#"
import socket
import ssl
import struct
import sys

cert_path, key_path = sys.argv[1], sys.argv[2]
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.minimum_version = ssl.TLSVersion.TLSv1_3
context.maximum_version = ssl.TLSVersion.TLSv1_3
context.load_cert_chain(cert_path, key_path)

listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind(("127.0.0.1", 0))
listener.listen(1)
print(listener.getsockname()[1], flush=True)


def recv_exact(sock, size):
    data = bytearray()
    while len(data) < size:
        chunk = sock.recv(size - len(data))
        if not chunk:
            raise RuntimeError("unexpected EOF")
        data.extend(chunk)
    return bytes(data)


plain, _ = listener.accept()
request = recv_exact(plain, 8)
if request != bytes([0, 0, 0, 8, 4, 210, 22, 47]):
    raise RuntimeError(f"unexpected PostgreSQL SSLRequest: {request!r}")
plain.sendall(b"S")
with context.wrap_socket(plain, server_side=True) as tls:
    if tls.version() != "TLSv1.3":
        raise RuntimeError(f"expected TLSv1.3, got {tls.version()!r}")
    length = struct.unpack("!I", recv_exact(tls, 4))[0]
    if length < 8 or length > 65536:
        raise RuntimeError(f"invalid StartupMessage length: {length}")
    startup = recv_exact(tls, length - 4)
    if b"user\x00owner-test\x00" not in startup:
        raise RuntimeError("StartupMessage did not contain the qualification user")
    tls.sendall(bytes([
        ord("R"), 0, 0, 0, 8, 0, 0, 0, 0,
        ord("K"), 0, 0, 0, 12, 0, 0, 0, 1, 0, 0, 0, 2,
        ord("Z"), 0, 0, 0, 5, ord("I"),
    ]))
print("OTERYN_WP3_TLS13_SERVER_OK TLSv1.3", flush=True)
listener.close()
"#;

#[test]
fn owner_aware_aws_lc_tls_positive_and_denial_qualification()
-> Result<(), Box<dyn Error>> {
    let root = repository_root()?;
    let helper = prepare_helper(&root)?;
    let binary = build_helper(&helper)?;

    let (denial_port, denial_server) = denial_server()?;
    let denial = run_helper(&binary, "deny", denial_port)?;
    ensure_success(&denial, "AWS-LC denial helper")?;
    ensure_marker(&denial, DENIAL_MARKER)?;
    denial_server
        .join()
        .map_err(|_panic| io::Error::other("denial server thread panicked"))??;

    let cert_dir = helper.join("certificate");
    fs::create_dir_all(&cert_dir)?;
    let cert = cert_dir.join("localhost.crt");
    let key = cert_dir.join("localhost.key");
    generate_certificate(&cert, &key)?;

    let (mut tls_server, mut server_stdout, positive_port) =
        spawn_tls_postgres_server(&cert, &key)?;
    let positive = run_helper(&binary, "positive", positive_port)?;
    if let Err(error) = ensure_success(&positive, "AWS-LC TLS-positive helper") {
        let _ = tls_server.kill();
        let _ = tls_server.wait();
        return Err(error);
    }
    ensure_marker(&positive, POSITIVE_MARKER)?;

    let server_status = tls_server.wait()?;
    let mut server_tail = String::new();
    server_stdout.read_to_string(&mut server_tail)?;
    if !server_status.success() || !server_tail.contains(SERVER_MARKER) {
        return Err(io::Error::other(format!(
            "TLS1.3 PostgreSQL fixture failed: status={server_status}, stdout={server_tail:?}"
        ))
        .into());
    }

    Ok(())
}

fn repository_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    Ok(manifest.join("../..").canonicalize()?)
}

fn prepare_helper(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let helper = root
        .join("target")
        .join(format!("wp3-aws-lc-qualification-{}", std::process::id()));
    fs::create_dir_all(helper.join("src"))?;

    let core = root.join("vendor/sqlx-core-0.9.0");
    let postgres = root.join("vendor/sqlx-postgres-0.9.0");
    let tokio = root.join("vendor/tokio-1.53.1");
    let rustls = root.join("vendor/rustls-0.23.43");
    let manifest = format!(
        r#"[package]
name = "oteryn-wp3-aws-lc-qualification"
version = "0.0.0"
edition = "2024"
rust-version = "1.94"
publish = false

[dependencies]
sqlx = {{ version = "=0.9.0", default-features = false, features = ["runtime-tokio", "tls-rustls-aws-lc-rs", "postgres"] }}
tokio = {{ version = "=1.53.1", default-features = false, features = ["io-util", "net", "rt", "time"] }}

[patch.crates-io]
sqlx-core = {{ path = {core:?} }}
sqlx-postgres = {{ path = {postgres:?} }}
tokio = {{ path = {tokio:?} }}
rustls = {{ path = {rustls:?} }}
"#,
        core = core.display().to_string(),
        postgres = postgres.display().to_string(),
        tokio = tokio.display().to_string(),
        rustls = rustls.display().to_string(),
    );
    fs::write(helper.join("Cargo.toml"), manifest)?;
    fs::write(helper.join("src/main.rs"), HELPER_SOURCE)?;

    let mut lock = fs::read_to_string(root.join("Cargo.lock"))?;
    if !lock.ends_with('\n') {
        lock.push('\n');
    }
    lock.push_str(HELPER_LOCK_ENTRY);
    fs::write(helper.join("Cargo.lock"), lock)?;
    Ok(helper)
}

fn build_helper(helper: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new("cargo")
        .arg("+1.94.0")
        .arg("build")
        .arg("--locked")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(helper.join("Cargo.toml"))
        .env("CARGO_TERM_COLOR", "never")
        .output()?;
    ensure_success(&output, "isolated SQLx AWS-LC profile build")?;
    Ok(helper
        .join("target/debug")
        .join("oteryn-wp3-aws-lc-qualification"))
}

fn run_helper(binary: &Path, mode: &str, port: u16) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(binary)
        .env("OTERYN_WP3_MODE", mode)
        .env("OTERYN_WP3_PORT", port.to_string())
        .output()?)
}

fn ensure_success(output: &Output, label: &str) -> Result<(), Box<dyn Error>> {
    if output.status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "{label} failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
    .into())
}

fn ensure_marker(output: &Output, marker: &str) -> Result<(), Box<dyn Error>> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains(marker) {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "qualification helper did not emit {marker}: {stdout}"
    ))
    .into())
}

fn denial_server() -> Result<(u16, thread::JoinHandle<io::Result<()>>), Box<dyn Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    let handle = thread::spawn(move || {
        let (mut socket, _peer) = listener.accept()?;
        let mut request = [0_u8; 8];
        socket.read_exact(&mut request)?;
        if request != SSL_REQUEST {
            return Err(io::Error::other("unexpected PostgreSQL SSLRequest"));
        }
        socket.write_all(b"S")?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut byte = [0_u8; 1];
        match socket.read(&mut byte) {
            Ok(0) => Ok(()),
            Ok(_count) => Err(io::Error::other(
                "underfunded owner emitted TLS bytes before terminal denial",
            )),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => Err(io::Error::other(
                "underfunded owner did not terminate after resource denial",
            )),
            Err(error) => Err(error),
        }
    });
    Ok((port, handle))
}

fn generate_certificate(cert: &Path, key: &Path) -> Result<(), Box<dyn Error>> {
    let output = Command::new("openssl")
        .args(["req", "-x509", "-newkey", "rsa:2048", "-sha256", "-nodes"])
        .arg("-keyout")
        .arg(key)
        .arg("-out")
        .arg(cert)
        .args(["-days", "1", "-subj", "/CN=localhost"])
        .output()?;
    ensure_success(&output, "test certificate generation")
}

fn spawn_tls_postgres_server(
    cert: &Path,
    key: &Path,
) -> Result<(std::process::Child, BufReader<std::process::ChildStdout>, u16), Box<dyn Error>> {
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(TLS_SERVER_SOURCE)
        .arg(cert)
        .arg(key)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("TLS fixture stdout pipe is unavailable"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other("TLS fixture exited before publishing its port").into());
    }
    let port = line.trim().parse()?;
    Ok((child, reader, port))
}
