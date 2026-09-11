#![cfg(target_os = "linux")]

use std::env::VarError;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

const DENIAL_MARKER: &str = "OTERYN_WP3_TLS_DENIAL_UNAVAILABLE";
const POSITIVE_MARKER: &str = "OTERYN_WP3_PG17_TLS13_VERIFY_FULL_POSITIVE";
const POSTGRES_IMAGE: &str = "postgres:17.6-bookworm@sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3";
const SERVER_CERT_PATH: &str = "/tmp/oteryn-wp3-server.crt";
const SERVER_KEY_PATH: &str = "/tmp/oteryn-wp3-server.key";

const HELPER_LOCK_ENTRY: &str = r#"
[[package]]
name = "oteryn-wp3-aws-lc-qualification"
version = "0.0.0"
dependencies = [
 "sqlx",
 "tokio",
]
"#;

const HELPER_SOURCE: &str = include_str!("oteryn_resource_budget_helper.rs");

struct TestPki {
    ca_cert: PathBuf,
    server_cert: PathBuf,
    server_key: PathBuf,
}

#[test]
fn owner_aware_aws_lc_tls_positive_and_denial_qualification() -> Result<(), Box<dyn Error>> {
    let admin_url = match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(VarError::NotPresent) => {
            eprintln!(
                "WP3 AWS-LC TLS qualification NOT_APPLICABLE: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };

    let root = repository_root()?;
    let helper = prepare_helper(&root)?;
    let pki = generate_test_pki(&helper.join("pki"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(configure_postgres_tls(&admin_url, &pki))?;

    let binary = build_helper(&helper)?;
    let positive = run_helper(&binary, "positive", &admin_url, &pki.ca_cert)?;
    ensure_success(&positive, "real PostgreSQL 17.6 AWS-LC TLS-positive helper")?;
    ensure_marker(&positive, POSITIVE_MARKER)?;

    let denial = run_helper(&binary, "deny", &admin_url, &pki.ca_cert)?;
    ensure_success(&denial, "real PostgreSQL 17.6 AWS-LC denial helper")?;
    ensure_marker(&denial, DENIAL_MARKER)?;
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

[workspace]
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

fn generate_test_pki(directory: &Path) -> Result<TestPki, Box<dyn Error>> {
    fs::create_dir_all(directory)?;
    let ca_key = directory.join("ca.key");
    let ca_cert = directory.join("ca.crt");
    let server_key = directory.join("server.key");
    let server_csr = directory.join("server.csr");
    let server_cert = directory.join("server.crt");
    let extensions = directory.join("server.ext");
    fs::write(
        &extensions,
        "[server_ext]\nsubjectAltName=DNS:localhost\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n",
    )?;

    run_checked(
        Command::new("openssl")
            .arg("req")
            .arg("-x509")
            .arg("-newkey")
            .arg("rsa:2048")
            .arg("-sha256")
            .arg("-nodes")
            .arg("-days")
            .arg("1")
            .arg("-keyout")
            .arg(&ca_key)
            .arg("-out")
            .arg(&ca_cert)
            .arg("-subj")
            .arg("/CN=Oteryn-WP3-Test-CA")
            .arg("-addext")
            .arg("basicConstraints=critical,CA:TRUE")
            .arg("-addext")
            .arg("keyUsage=critical,keyCertSign,cRLSign"),
        "generate ephemeral WP3 test CA",
    )?;
    run_checked(
        Command::new("openssl")
            .arg("req")
            .arg("-new")
            .arg("-newkey")
            .arg("rsa:2048")
            .arg("-sha256")
            .arg("-nodes")
            .arg("-keyout")
            .arg(&server_key)
            .arg("-out")
            .arg(&server_csr)
            .arg("-subj")
            .arg("/CN=localhost"),
        "generate ephemeral WP3 server CSR",
    )?;
    run_checked(
        Command::new("openssl")
            .arg("x509")
            .arg("-req")
            .arg("-in")
            .arg(&server_csr)
            .arg("-CA")
            .arg(&ca_cert)
            .arg("-CAkey")
            .arg(&ca_key)
            .arg("-CAcreateserial")
            .arg("-out")
            .arg(&server_cert)
            .arg("-days")
            .arg("1")
            .arg("-sha256")
            .arg("-extfile")
            .arg(&extensions)
            .arg("-extensions")
            .arg("server_ext"),
        "sign ephemeral WP3 localhost server certificate",
    )?;

    Ok(TestPki {
        ca_cert,
        server_cert,
        server_key,
    })
}

async fn configure_postgres_tls(admin_url: &str, pki: &TestPki) -> Result<(), Box<dyn Error>> {
    let pool = sqlx::PgPool::connect(admin_url).await?;
    let version_num: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&pool)
        .await?;
    if version_num != "170006" {
        return Err(
            format!("expected PostgreSQL 17.6, got server_version_num={version_num}").into(),
        );
    }

    let container = postgres_service_container()?;
    docker_copy_owned(&container, &pki.server_cert, SERVER_CERT_PATH)?;
    docker_copy_owned(&container, &pki.server_key, SERVER_KEY_PATH)?;

    for statement in [
        "ALTER SYSTEM SET ssl = 'on'",
        "ALTER SYSTEM SET ssl_cert_file = '/tmp/oteryn-wp3-server.crt'",
        "ALTER SYSTEM SET ssl_key_file = '/tmp/oteryn-wp3-server.key'",
        "ALTER SYSTEM SET ssl_min_protocol_version = 'TLSv1.3'",
        "ALTER SYSTEM SET ssl_max_protocol_version = 'TLSv1.3'",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let reloaded: bool = sqlx::query_scalar("SELECT pg_reload_conf()")
        .fetch_one(&pool)
        .await?;
    if !reloaded {
        return Err("PostgreSQL rejected pg_reload_conf()".into());
    }

    for _ in 0..50 {
        let settings: (String, String, String) = sqlx::query_as(
            "SELECT current_setting('ssl'), current_setting('ssl_min_protocol_version'), \
                    current_setting('ssl_max_protocol_version')",
        )
        .fetch_one(&pool)
        .await?;
        if settings == ("on".to_owned(), "TLSv1.3".to_owned(), "TLSv1.3".to_owned()) {
            pool.close().await;
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    pool.close().await;
    Err("PostgreSQL 17.6 did not activate the TLS1.3 test configuration".into())
}

fn postgres_service_container() -> Result<String, Box<dyn Error>> {
    let output = Command::new("docker")
        .arg("ps")
        .arg("--filter")
        .arg("publish=5432")
        .arg("--format")
        .arg("{{.ID}}")
        .output()?;
    ensure_success(&output, "locate PostgreSQL 17.6 service container")?;

    let mut matches = Vec::new();
    for id in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
    {
        let inspect = Command::new("docker")
            .arg("inspect")
            .arg("--format")
            .arg("{{.Config.Image}}")
            .arg(id)
            .output()?;
        ensure_success(&inspect, "inspect PostgreSQL service container")?;
        if String::from_utf8_lossy(&inspect.stdout).trim() == POSTGRES_IMAGE {
            matches.push(id.to_owned());
        }
    }
    if matches.len() != 1 {
        return Err(format!(
            "expected exactly one canonical PostgreSQL 17.6 service container, found {}",
            matches.len()
        )
        .into());
    }
    Ok(matches.remove(0))
}

fn docker_copy_owned(
    container: &str,
    source: &Path,
    destination: &str,
) -> Result<(), Box<dyn Error>> {
    run_checked(
        Command::new("docker")
            .arg("cp")
            .arg(source)
            .arg(format!("{container}:{destination}")),
        "copy PostgreSQL TLS fixture into service container",
    )?;
    run_checked(
        Command::new("docker")
            .arg("exec")
            .arg("--user")
            .arg("root")
            .arg(container)
            .arg("chown")
            .arg("postgres:postgres")
            .arg(destination),
        "set PostgreSQL TLS fixture ownership",
    )?;
    run_checked(
        Command::new("docker")
            .arg("exec")
            .arg("--user")
            .arg("root")
            .arg(container)
            .arg("chmod")
            .arg("600")
            .arg(destination),
        "set PostgreSQL TLS fixture permissions",
    )?;
    Ok(())
}

fn build_helper(helper: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let lock_path = helper.join("Cargo.lock");
    let seeded_lock = fs::read_to_string(&lock_path)?;
    let seeded_identities = lock_package_identities(&seeded_lock);

    let reconcile = Command::new("cargo")
        .arg("+1.94.0")
        .arg("metadata")
        .arg("--offline")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(helper.join("Cargo.toml"))
        .env("CARGO_TERM_COLOR", "never")
        .output()?;
    ensure_success(&reconcile, "isolated SQLx AWS-LC offline lock reconciliation")?;

    let reconciled_lock = fs::read_to_string(&lock_path)?;
    if lock_package_identities(&reconciled_lock) != seeded_identities {
        return Err("offline lock reconciliation changed the seeded package identity set".into());
    }

    let output = Command::new("cargo")
        .arg("+1.94.0")
        .arg("build")
        .arg("--locked")
        .arg("--offline")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(helper.join("Cargo.toml"))
        .env("CARGO_TERM_COLOR", "never")
        .env("CARGO_TARGET_DIR", helper.join("target"))
        .output()?;
    ensure_success(&output, "isolated SQLx AWS-LC profile build")?;
    if fs::read_to_string(&lock_path)? != reconciled_lock {
        return Err("locked offline helper build mutated Cargo.lock".into());
    }

    Ok(helper
        .join("target/debug")
        .join("oteryn-wp3-aws-lc-qualification"))
}

fn lock_package_identities(lock: &str) -> Vec<String> {
    let mut identities = Vec::new();
    let mut package = Vec::new();
    let mut in_package = false;

    for line in lock.lines() {
        if line == "[[package]]" {
            if in_package {
                identities.push(package.join("\n"));
                package.clear();
            }
            in_package = true;
            continue;
        }
        if in_package
            && (line.starts_with("name = ")
                || line.starts_with("version = ")
                || line.starts_with("source = ")
                || line.starts_with("checksum = "))
        {
            package.push(line.to_owned());
        }
    }
    if in_package {
        identities.push(package.join("\n"));
    }
    identities.sort();
    identities
}

fn run_helper(
    binary: &Path,
    mode: &str,
    admin_url: &str,
    ca_cert: &Path,
) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(binary)
        .env("OTERYN_WP3_MODE", mode)
        .env("OTERYN_TEST_POSTGRES_ADMIN_URL", admin_url)
        .env("OTERYN_WP3_CA_CERT", ca_cert)
        .output()?)
}

fn run_checked(command: &mut Command, label: &str) -> Result<(), Box<dyn Error>> {
    let output = command.output()?;
    ensure_success(&output, label)
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
