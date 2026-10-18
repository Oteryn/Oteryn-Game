#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// D1343 STAFF_ACTION PostgreSQL cases shared by the focused wrapper and the
// canonical durability aggregate. The crate root supplies one durability type
// universe and the public production staff module.

use crate::durability::staff_action::{self as owner, Begin, StaffError, StoredOutcome};
use crate::staff::{
    audit::{Contract, Reason, ResultCode},
    signature::{
        self, Action, Decimal, Disposition, Error, History, Outcome, RECEIPT_NAMESPACE,
        REQUEST_NAMESPACE, Receipt, VerifierTrust,
    },
};
use crate::{durability, node, staff};
use sqlx::{Connection, Executor};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn debug(e: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{e:?}").into()
}
fn runtime() -> TestResult<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

// This fixture owns its OpenSSH key and roster independently of production
// internals. Verification loads that roster through the public, checked-file API.
struct Signer {
    path: PathBuf,
    roster: Vec<u8>,
}
impl Signer {
    fn new() -> Self {
        Self::with_role(None)
    }
    /// GM-1 (D1437): `role` names the roster role line of the one signer.
    fn with_role(role: Option<&str>) -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let nonce = node::operator_files::random_bytes::<32>().expect("fixture nonce");
        let path =
            std::env::temp_dir().join(format!("oteryn-staff-pg-{}", signature::sha256(&nonce)));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .expect("private fixture directory");
        let mut signer = Self {
            path,
            roster: Vec::new(),
        };
        assert!(
            Command::new("/usr/bin/ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(signer.path.join("key"))
                .status()
                .expect("OpenSSH")
                .success()
        );
        let public =
            std::fs::read_to_string(signer.path.join("key.pub")).expect("public fixture key");
        let key = public
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        let role = role.map_or_else(String::new, |role| {
            format!("{}operator {role}\n", signature::ROLE_PREFIX)
        });
        signer.roster = format!(
            "{role}operator namespaces=\"{REQUEST_NAMESPACE},{RECEIPT_NAMESPACE}\" {key}\n"
        )
        .into_bytes();
        signer.write("allowed_signers", &signer.roster);
        signer
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(self.path.join(name))
            .expect("private fixture file");
        file.write_all(bytes).expect("fixture bytes");
        file.sync_all().expect("fixture file sync");
    }
    fn trust(&self, environment: &str, activation: &str) -> VerifierTrust {
        VerifierTrust::load(
            environment.into(),
            &self.path.join("allowed_signers"),
            signature::sha256(&self.roster),
            activation.into(),
        )
        .expect("independent fixture trust through public loader")
    }
    fn sign(&self, bytes: Vec<u8>, namespace: &str) -> signature::SignedBytes {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let name = format!(
            "message-{}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        self.write(&name, &bytes);
        assert!(
            Command::new("/usr/bin/ssh-keygen")
                .args(["-Y", "sign", "-f"])
                .arg(self.path.join("key"))
                .args(["-n", namespace])
                .arg(self.path.join(&name))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("upstream sign")
                .success()
        );
        signature::SignedBytes {
            bytes,
            signature: std::fs::read_to_string(self.path.join(format!("{name}.sig")))
                .expect("signature"),
            principal: "operator".into(),
        }
    }
    fn request(&self, operation: u8, sequence: u64, previous: &str) -> signature::Request {
        signature::Request {
            version: 1,
            operation_id: format!("00000000-0000-7000-8000-{operation:012x}"),
            environment: "alpha-test".into(),
            roster_sha256: signature::sha256(&self.roster),
            issued_at: Decimal(1_000_000),
            control_seq: Decimal(sequence),
            previous_link_sha256: previous.into(),
            scope: Some(signature::Scope {
                world_id: "00000000-0000-7000-8000-000000000010".into(),
                channel_id: "00000000-0000-7000-8000-000000000011".into(),
            }),
            reason: Reason::Support,
            action: Action::Inspect {
                character_id: "00000000-0000-7000-8000-000000000001".into(),
            },
        }
    }
    fn activation(&self) -> (VerifierTrust, signature::SignedBytes) {
        let mut request = self.request(1, 1, &"0".repeat(64));
        request.scope = None;
        request.reason = Reason::Roster;
        request.action = Action::RosterActivate {
            roster_sha256: signature::sha256(&self.roster),
        };
        let signed = self.sign(
            request.canonical_bytes().expect("activation JCS"),
            REQUEST_NAMESPACE,
        );
        (
            self.trust("alpha-test", &signature::sha256(&signed.bytes)),
            signed,
        )
    }
}
impl Drop for Signer {
    fn drop(&mut self) {
        if let Ok(files) = std::fs::read_dir(&self.path) {
            for file in files.flatten() {
                let _ = std::fs::remove_file(file.path());
            }
        }
        let _ = std::fs::remove_dir(&self.path);
    }
}

fn signed_activation(
    signer: &Signer,
    environment: &str,
    sequence: u64,
    previous: &str,
) -> TestResult<signature::VerifiedRequest> {
    let mut request = signer.request(70, sequence, previous);
    request.environment = environment.into();
    request.scope = None;
    request.reason = Reason::Roster;
    request.action = Action::RosterActivate {
        roster_sha256: signature::sha256(&signer.roster),
    };
    let signed = signer.sign(request.canonical_bytes().map_err(debug)?, REQUEST_NAMESPACE);
    signer
        .trust(environment, &signature::sha256(&signed.bytes))
        .verify_request(signed)
        .map_err(debug)
}

async fn insert_activation_link(
    connection: &mut sqlx::PgConnection,
    request: &signature::VerifiedRequest,
) -> Result<(), sqlx::Error> {
    let intent = request.request();
    let signed = request.signed();
    sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES($1,$2::text::numeric,$3,$4,$5,'ACTIVATION',$6,$7,$8,$9,txid_current())")
        .bind(&intent.environment)
        .bind(intent.control_seq.0.to_string())
        .bind(request.digest())
        .bind(&intent.previous_link_sha256)
        .bind(&intent.roster_sha256)
        .bind(&signed.bytes)
        .bind(&signed.principal)
        .bind(request.fingerprint())
        .bind(&signed.signature)
        .execute(connection)
        .await?;
    Ok(())
}

async fn advance_activation_head(
    connection: &mut sqlx::PgConnection,
    request: &signature::VerifiedRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO game_staff_control_heads(environment,control_seq,link_sha256,activation_sha256) VALUES($1,$2::text::numeric,$3,$3) ON CONFLICT(environment) DO UPDATE SET control_seq=EXCLUDED.control_seq,link_sha256=EXCLUDED.link_sha256,activation_sha256=EXCLUDED.activation_sha256")
        .bind(&request.request().environment)
        .bind(request.request().control_seq.0.to_string())
        .bind(request.digest())
        .execute(&mut *connection)
        .await?;
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(connection)
        .await?;
    Ok(())
}

async fn commit_activation(
    pool: &sqlx::PgPool,
    request: &signature::VerifiedRequest,
) -> TestResult {
    let mut tx = pool.begin().await?;
    insert_activation_link(&mut tx, request).await?;
    advance_activation_head(&mut tx, request).await?;
    tx.commit().await?;
    Ok(())
}

struct Fixture {
    admin: String,
    name: String,
    url: String,
    pool: sqlx::PgPool,
    signer: Signer,
    trust: Arc<VerifierTrust>,
    history: Arc<History>,
}
impl Fixture {
    async fn create(tag: &str) -> TestResult<Option<Self>> {
        Self::create_as(tag, None).await
    }
    async fn create_as(tag: &str, role: Option<&str>) -> TestResult<Option<Self>> {
        let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: staff PG cases were not executed; missing OTERYN_TEST_POSTGRES_ADMIN_URL"
            );
            return Ok(None);
        };
        if !admin.starts_with("postgresql://oteryn_test_admin:")
            || !admin.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe test admin URL".into());
        }
        let contract = Contract::registered().map_err(debug)?;
        assert_eq!(contract.event_type(), 5, "D1343 STAFF_ACTION allocation");
        assert_eq!(contract.revision(), 1);
        let name = format!(
            "gm_{tag}_{}",
            signature::sha256(&node::operator_files::random_bytes::<32>()?)[..20].to_owned()
        );
        let mut connection = sqlx::PgConnection::connect(&admin).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(version, "170006", "canonical PostgreSQL is 17.6");
        connection
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        connection.close().await?;
        let url = format!(
            "{}/{name}",
            admin.strip_suffix("/postgres").ok_or("admin URL")?
        );
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        // D1343 installs the real migration through the shared SQLx ledger.
        // Never apply a second, fixture-only copy of the staff schema.
        let installed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=116 AND success) AND to_regclass('game_staff_operations') IS NOT NULL")
            .fetch_one(&pool)
            .await?;
        assert!(installed, "staff migration 0116 must be installed");
        sqlx::raw_sql("CREATE TABLE staff_fixture_domain(operation_id text PRIMARY KEY); GRANT SELECT,INSERT ON staff_fixture_domain TO oteryn_game_runtime").execute(&pool).await?;
        let signer = Signer::with_role(role);
        let (trust, activation) = signer.activation();
        let checked = trust.verify_request(activation.clone()).map_err(debug)?;
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',1,$1,$2,$3,'ACTIVATION',$4,$5,$6,$7,txid_current())")
            .bind(checked.digest()).bind("0".repeat(64)).bind(trust.roster_digest()).bind(&activation.bytes).bind(&activation.principal).bind(checked.fingerprint()).bind(&activation.signature).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_staff_control_heads(environment,control_seq,link_sha256,activation_sha256) VALUES('alpha-test',1,$1,$1)").bind(trust.activation_digest()).execute(&mut *tx).await?;
        tx.commit().await?;
        // D1412: the reviewed activation history pins the first roster at 1.
        let history = History::from_reviewed(
            vec![signature::Activation {
                sequence: 1,
                trust: signer.trust("alpha-test", trust.activation_digest()),
            }],
            &trust,
        )
        .map_err(debug)?;
        Ok(Some(Self {
            admin,
            name,
            url,
            pool,
            signer,
            trust: Arc::new(trust),
            history: Arc::new(history),
        }))
    }
    fn signed(
        &self,
        operation: u8,
        sequence: u64,
        previous: &str,
        mutation: bool,
    ) -> signature::SignedBytes {
        let mut request = self.signer.request(operation, sequence, previous);
        if mutation {
            request.action = Action::Mute {
                character_id: "00000000-0000-7000-8000-000000000001".into(),
                duration_ms: Decimal(60_000),
            };
        }
        self.signer.sign(
            request.canonical_bytes().expect("request"),
            REQUEST_NAMESPACE,
        )
    }
    async fn execute(
        &self,
        signed: signature::SignedBytes,
        now: Result<u64, Error>,
        domain_marker: bool,
    ) -> Result<StoredOutcome, StaffError> {
        self.execute_with(&self.trust, &self.history, signed, now, domain_marker)
            .await
    }
    async fn execute_with(
        &self,
        trust: &VerifierTrust,
        history: &History,
        signed: signature::SignedBytes,
        now: Result<u64, Error>,
        domain_marker: bool,
    ) -> Result<StoredOutcome, StaffError> {
        let request = trust.verify_request(signed)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        match owner::fixture_begin_in_transaction(&mut tx, trust, history, request, now).await? {
            Begin::Replay(outcome) => {
                tx.rollback().await?;
                Ok(outcome)
            }
            Begin::Fresh(pending) => {
                if domain_marker {
                    sqlx::query("INSERT INTO staff_fixture_domain(operation_id) VALUES('once')")
                        .execute(&mut *tx)
                        .await?;
                }
                let outcome = Outcome {
                    disposition: Disposition::Committed,
                    result: ResultCode::Ok,
                    action_id: node::uuid_text(
                        &node::operator_files::uuid_v7().map_err(|_| Error::Invalid)?,
                    ),
                    transaction_id: node::uuid_text(
                        &node::operator_files::uuid_v7().map_err(|_| Error::Invalid)?,
                    ),
                    occurred_at: Decimal(now?),
                };
                let answer = owner::finish_in_transaction(
                    &mut tx,
                    pending,
                    Contract::registered()?,
                    outcome,
                    "staff-test",
                )
                .await?;
                tx.commit().await?;
                Ok(answer)
            }
        }
    }
    async fn counts(&self) -> TestResult<(i64, i64, i64, i64, String)> {
        Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_staff_control_links),(SELECT count(*) FROM game_staff_operations),(SELECT count(*) FROM game_staff_action_outbox),(SELECT count(*) FROM staff_fixture_domain),(SELECT control_seq::text FROM game_staff_control_heads WHERE environment='alpha-test')").fetch_one(&self.pool).await?)
    }
    async fn cleanup(self) -> TestResult {
        self.pool.close().await;
        let mut admin = sqlx::PgConnection::connect(&self.admin).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        admin.close().await?;
        Ok(())
    }
}

#[test]
fn staff_pg_activation_requires_current_predecessor_and_narrow_bootstrap() -> TestResult {
    async fn head_rows(pool: &sqlx::PgPool) -> TestResult<String> {
        Ok(sqlx::query_scalar("SELECT COALESCE(json_agg(h ORDER BY environment)::text,'[]') FROM game_staff_control_heads h")
            .fetch_one(pool).await?)
    }

    async fn reject(
        fixture: &Fixture,
        environment: &str,
        sequence: u64,
        previous: &str,
        label: &str,
        failures: &mut Vec<String>,
    ) -> TestResult {
        let signer = Signer::new();
        let request = signed_activation(&signer, environment, sequence, previous)?;
        let before = (fixture.counts().await?, head_rows(&fixture.pool).await?);
        let mut tx = fixture.pool.begin().await?;
        let attempt: Result<(), sqlx::Error> = async {
            insert_activation_link(&mut tx, &request).await?;
            advance_activation_head(&mut tx, &request).await
        }
        .await;
        tx.rollback().await?;
        if (fixture.counts().await?, head_rows(&fixture.pool).await?) != before {
            return Err(format!("activation rollback changed retained state: {label}").into());
        }
        match attempt {
            Ok(()) => failures.push(format!("{label}: accepted")),
            Err(error) => {
                let database = error.as_database_error().ok_or_else(|| debug(&error))?;
                if database.code().as_deref() != Some("23514")
                    || database.message() != "staff predecessor conflict"
                {
                    failures.push(format!("{label}: wrong predecessor refusal"));
                }
            }
        }
        Ok(())
    }

    runtime()?.block_on(async {
        let Some(f) = Fixture::create("activation_predecessor").await? else {
            return Ok(());
        };
        let result: TestResult<Vec<String>> = async {
            let mut failures = Vec::new();
            for (label, environment, sequence, previous) in [
                ("bootstrap_gap", "alpha-bootstrap", 2, "0".repeat(64)),
                ("bootstrap_previous", "alpha-bootstrap", 1, "e".repeat(64)),
                (
                    "rotation_gap",
                    "alpha-test",
                    3,
                    f.trust.activation_digest().into(),
                ),
                (
                    "rotation_stale_sequence",
                    "alpha-test",
                    1,
                    f.trust.activation_digest().into(),
                ),
                ("rotation_wrong_previous", "alpha-test", 2, "e".repeat(64)),
            ] {
                reject(&f, environment, sequence, &previous, label, &mut failures).await?;
            }

            // Bootstrap is allowed only in an empty namespace, at sequence one
            // with the zero predecessor. SQL mirrors come from these exact bytes.
            let bootstrap_signer = Signer::new();
            let bootstrap =
                signed_activation(&bootstrap_signer, "alpha-bootstrap", 1, &"0".repeat(64))?;
            commit_activation(&f.pool, &bootstrap).await?;

            let first_signer = Signer::new();
            let first =
                signed_activation(&first_signer, "alpha-test", 2, f.trust.activation_digest())?;
            commit_activation(&f.pool, &first).await?;
            reject(
                &f,
                "alpha-test",
                3,
                f.trust.activation_digest(),
                "rotation_stale_previous",
                &mut failures,
            )
            .await?;
            let later_signer = Signer::new();
            let later = signed_activation(&later_signer, "alpha-test", 3, first.digest())?;
            commit_activation(&f.pool, &later).await?;

            // A valid later rotation still yields complete independently trusted
            // history; the verifier must not need a skipped or invented link.
            let current = later_signer.trust("alpha-test", later.digest());
            let history = signature::History::from_reviewed(
                vec![
                    signature::Activation {
                        sequence: 1,
                        trust: f.signer.trust("alpha-test", f.trust.activation_digest()),
                    },
                    signature::Activation {
                        sequence: 2,
                        trust: first_signer.trust("alpha-test", first.digest()),
                    },
                    signature::Activation {
                        sequence: 3,
                        trust: later_signer.trust("alpha-test", later.digest()),
                    },
                ],
                &current,
            )
            .map_err(debug)?;
            let root = durability::DurabilityRoot::connect_test_runtime(&f.url)?;
            if !root.maintain_ready_once().await? {
                return Err("staff root did not become ready".into());
            }
            let links = root
                .staff_evidence_page("alpha-test".into(), 0, 3)
                .await
                .map_err(debug)?
                .into_iter()
                .map(|row| row.link)
                .collect::<Vec<_>>();
            let verified = history.verify(&links, 3, later.digest()).map_err(debug)?;
            if verified.len() != 3 {
                return Err("later activation history omitted a link".into());
            }
            drop(root);
            Ok(failures)
        }
        .await;
        f.cleanup().await?;
        let failures = result?;
        assert!(
            failures.is_empty(),
            "activation predecessor failures: {failures:?}"
        );
        Ok(())
    })
}

#[test]
fn staff_pg_activation_insert_waits_for_current_head_then_rejects_stale_predecessor() -> TestResult
{
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("activation_lock").await? else {
            return Ok(());
        };
        let result: TestResult<Vec<&str>> = async {
            let mut failures = Vec::new();
            let signer = Signer::new();
            let candidate =
                signed_activation(&signer, "alpha-test", 2, f.trust.activation_digest())?;
            let candidate_digest = candidate.digest().to_owned();
            let mut blocker = f.pool.begin().await?;
            owner::lock_head(&mut blocker, "alpha-test").await.map_err(debug)?;
            let pool = f.pool.clone();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let waiter = tokio::spawn(async move {
                let mut tx = pool.begin().await?;
                let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                    .fetch_one(&mut *tx).await?;
                sender.send(pid).map_err(|_| sqlx::Error::RowNotFound)?;
                // Stop at INSERT: an UPDATE waiting on the head row would not
                // prove that the activation predecessor was checked beforehand.
                let attempt = insert_activation_link(&mut tx, &candidate).await;
                tx.rollback().await?;
                Ok::<_, sqlx::Error>(attempt)
            });
            let pid = receiver.await?;
            let blocked = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if waiter.is_finished() {
                        break Ok::<_, sqlx::Error>(false);
                    }
                    let blocked: i32 = sqlx::query_scalar("SELECT cardinality(pg_blocking_pids($1))")
                        .bind(pid).fetch_one(&f.pool).await?;
                    if blocked > 0 {
                        break Ok(true);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await??;
            if !blocked {
                failures.push("activation INSERT bypassed the locked head");
            }

            let next_signer = Signer::new();
            let next =
                signed_activation(&next_signer, "alpha-test", 2, f.trust.activation_digest())?;
            insert_activation_link(&mut blocker, &next).await?;
            advance_activation_head(&mut blocker, &next).await?;
            blocker.commit().await?;
            match waiter.await?? {
                Ok(()) => failures.push("activation INSERT accepted the stale predecessor"),
                Err(error) => {
                    let database = error.as_database_error().ok_or_else(|| debug(&error))?;
                    if database.code().as_deref() != Some("23514")
                        || database.message() != "staff predecessor conflict"
                    {
                        failures.push("activation INSERT returned the wrong predecessor refusal");
                    }
                }
            }
            let leaked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_control_links WHERE environment='alpha-test' AND link_sha256=$1)")
                .bind(candidate_digest).fetch_one(&f.pool).await?;
            let head: (String, String, String) = sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
                .fetch_one(&f.pool).await?;
            if leaked || head != ("2".into(), next.digest().into(), next.digest().into()) {
                return Err("stale activation changed the committed head or leaked a link".into());
            }
            Ok(failures)
        }
        .await;
        f.cleanup().await?;
        let failures = result?;
        assert!(failures.is_empty(), "activation lock failures: {failures:?}");
        Ok(())
    })
}

#[test]
fn staff_pg_identical_expired_retry_is_stored_outcome_and_changed_same_id_refuses() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("retry").await? else {
            return Ok(());
        };
        let signed = f.signed(2, 2, f.trust.activation_digest(), false);
        let committed = f
            .execute(signed.clone(), Ok(1_000_000), false)
            .await
            .map_err(debug)?;
        let before = f.counts().await?;
        assert_eq!(
            f.execute(signed.clone(), Ok(u64::MAX - 1), true)
                .await
                .map_err(debug)?,
            committed
        );
        assert_eq!(
            f.execute(signed, Err(Error::Clock), true)
                .await
                .map_err(debug)?,
            committed
        );
        let mut changed = f.signer.request(2, 2, f.trust.activation_digest());
        changed.reason = staff::audit::Reason::Harassment;
        let changed = f
            .signer
            .sign(changed.canonical_bytes().map_err(debug)?, REQUEST_NAMESPACE);
        assert!(matches!(
            f.execute(changed, Ok(1_000_000), false).await,
            Err(StaffError::OperationConflict)
        ));
        assert_eq!(f.counts().await?, before);
        f.cleanup().await
    })
}

#[test]
fn staff_pg_stale_future_clock_sequence_previous_and_roster_refuse_without_writes() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("negative").await? else {
            return Ok(());
        };
        let before = f.counts().await?;
        for now in [Ok(1_600_001), Ok(699_999), Err(Error::Clock)] {
            assert!(
                f.execute(
                    f.signed(2, 2, f.trust.activation_digest(), false),
                    now,
                    false
                )
                .await
                .is_err()
            );
            assert_eq!(f.counts().await?, before);
        }
        for (sequence, previous) in [
            (1, f.trust.activation_digest().to_owned()),
            (2, "f".repeat(64)),
        ] {
            assert!(matches!(
                f.execute(
                    f.signed(2, sequence, &previous, false),
                    Ok(1_000_000),
                    false
                )
                .await,
                Err(StaffError::Conflict)
            ));
            assert_eq!(f.counts().await?, before);
        }
        let mut other = f.signer.request(2, 2, f.trust.activation_digest());
        other.roster_sha256 = "e".repeat(64);
        let signed = f
            .signer
            .sign(other.canonical_bytes().map_err(debug)?, REQUEST_NAMESPACE);
        assert!(matches!(
            f.execute(signed, Ok(1_000_000), false).await,
            Err(StaffError::Rejected(Error::Binding))
        ));
        assert_eq!(f.counts().await?, before);
        f.cleanup().await
    })
}

#[test]
fn staff_pg_failed_audit_insert_rolls_back_domain_link_outcome_and_head() -> TestResult {
    runtime()?.block_on(async {
        let Some(f)=Fixture::create("rollback").await? else {return Ok(());};
        // Sequence changes survive rollback: prove the actual audit INSERT was
        // reached, rather than accepting an unrelated earlier refusal as proof.
        sqlx::raw_sql("CREATE SEQUENCE staff_fixture_audit_attempts; GRANT USAGE ON SEQUENCE staff_fixture_audit_attempts TO oteryn_game_runtime; CREATE FUNCTION staff_fixture_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('staff_fixture_audit_attempts'); RAISE EXCEPTION 'fixture audit failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER staff_fixture_fail BEFORE INSERT ON game_staff_action_outbox FOR EACH ROW EXECUTE FUNCTION staff_fixture_fail()").execute(&f.pool).await?;
        let before=f.counts().await?;
        let failed=f.execute(f.signed(2,2,f.trust.activation_digest(),true),Ok(1_000_000),true).await;
        assert!(matches!(failed,Err(StaffError::Database(durability::DurabilityError::Database(durability::SqlxFailureClass::Database(None))))));
        let attempts:i64=sqlx::query_scalar("SELECT last_value FROM staff_fixture_audit_attempts").fetch_one(&f.pool).await?;
        assert_eq!(attempts,1);
        assert_eq!(f.counts().await?,before);
        // The read participant also returns no successful outcome on audit failure.
        let read=f.signed(3,2,f.trust.activation_digest(),false);
        let failed=f.execute(read.clone(),Ok(1_000_000),false).await;
        assert!(matches!(failed,Err(StaffError::Database(durability::DurabilityError::Database(durability::SqlxFailureClass::Database(None))))));
        let attempts:i64=sqlx::query_scalar("SELECT last_value FROM staff_fixture_audit_attempts").fetch_one(&f.pool).await?;
        assert_eq!(attempts,2);
        assert_eq!(f.counts().await?,before);
        sqlx::query("DROP TRIGGER staff_fixture_fail ON game_staff_action_outbox").execute(&f.pool).await?;
        assert!(f.execute(read,Ok(1_000_000),false).await.is_ok());
        f.cleanup().await
    })
}

#[test]
fn staff_pg_old_loaded_config_rejects_new_work_after_recorded_activation_and_retains_terminal_outcome()
-> TestResult {
    runtime()?.block_on(async {
        let Some(f)=Fixture::create("activation").await? else {return Ok(());};
        let old=f.signed(2,2,f.trust.activation_digest(),false);
        let old_digest=signature::sha256(&old.bytes);
        let outcome=f.execute(old.clone(),Ok(1_000_000),false).await.map_err(debug)?;
        // Independently provision another personal key/roster, sign and record
        // its activation outside the runtime role (ALPHA has no activation API).
        let replacement=Signer::new();let mut activation=replacement.request(3,3,&old_digest);
        activation.scope=None;
        activation.reason=staff::audit::Reason::Roster;
        activation.action=Action::RosterActivate {roster_sha256:signature::sha256(&replacement.roster)};
        let signed=replacement.sign(activation.canonical_bytes().map_err(debug)?,REQUEST_NAMESPACE);
        let digest=signature::sha256(&signed.bytes);
        let current=replacement.trust("alpha-test",&digest);
        let verified=current.verify_request(signed.clone()).map_err(debug)?;
        let mut tx=f.pool.begin().await?;
        sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',3,$1,$2,$3,'ACTIVATION',$4,$5,$6,$7,txid_current())")
            .bind(&digest).bind(&old_digest).bind(current.roster_digest()).bind(&signed.bytes).bind(&signed.principal).bind(verified.fingerprint()).bind(&signed.signature).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_staff_control_heads SET control_seq=3,link_sha256=$1,activation_sha256=$1 WHERE environment='alpha-test'").bind(&digest).execute(&mut *tx).await?;
        tx.commit().await?;
        let before=f.counts().await?;
        // All signed old-request fields except independently current activation
        // are valid. It is new work, not replay of the earlier operation.
        let stale=f.signed(4,4,&digest,false);
        assert!(matches!(f.execute(stale,Ok(1_000_000),false).await,Err(StaffError::Rejected(Error::Binding))));
        assert!(matches!(current.verify_request(old.clone()),Err(Error::Signature)));
        let root=durability::DurabilityRoot::connect_test_runtime(&f.url)?;
        assert!(root.maintain_ready_once().await?);
        let rows=root.staff_evidence_page("alpha-test".into(),0,3).await.map_err(debug)?;
        let history=signature::History::from_reviewed(vec![
            signature::Activation {sequence:1,trust:f.signer.trust("alpha-test",f.trust.activation_digest())},
            signature::Activation {sequence:3,trust:replacement.trust("alpha-test",&digest)},
        ],&current).map_err(debug)?;
        let prove=|wanted:String|->Result<signature::HistoricalRequest,Error> {
            let mut cursor=history.cursor_for_request(wanted)?;
            for row in &rows {cursor.push(&row.link)?;}
            cursor.finish_reconciliation(3,&digest)
        };
        // Genuine restarted/current-roster path is read-only historical
        // reconciliation, not a retry through the old-key execution verifier.
        assert_eq!(root.reconcile_staff_historical(prove(old_digest.clone()).map_err(debug)?).await.map_err(debug)?,Some(outcome.clone()));
        assert!(prove(signature::sha256(b"changed same operation bytes")).is_err());
        assert!(root.reconcile_staff_historical(prove(digest.clone()).map_err(debug)?).await.map_err(debug)?.is_none());
        let mut missing=history.cursor_for_request(old_digest.clone()).map_err(debug)?;
        missing.push(&rows[0].link).map_err(debug)?;
        assert!(missing.push(&rows[2].link).is_err());
        assert_eq!(f.execute(old,Err(Error::Clock),true).await.map_err(debug)?,outcome);
        // This fixture deliberately retains the old loaded verifier. A node
        // restarted with current trust rejects old-roster submissions before
        // runtime lookup; this test does not claim cross-rotation runtime retry.
        assert_eq!(f.counts().await?,before);
        // A hostile owner can bypass triggers, but cannot turn changed retained
        // operation bytes into a terminal match for the chain-sealed request.
        let forged=b"different request bytes".to_vec();
        let mut tx=f.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_staff_operations SET request_bytes=$1,request_sha256=encode(sha256($1),'hex'),created_xact_id=txid_current() WHERE environment='alpha-test'").bind(forged).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(matches!(root.reconcile_staff_historical(prove(old_digest).map_err(debug)?).await,Err(StaffError::OperationConflict)));
        drop(root);
        f.cleanup().await
    })
}

#[test]
fn staff_pg_same_predecessor_two_connections_have_one_typed_loser() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("concurrency").await? else {
            return Ok(());
        };
        let f = Arc::new(f);
        let a = f.signed(2, 2, f.trust.activation_digest(), false);
        let b = f.signed(3, 2, f.trust.activation_digest(), false);
        let barrier = Arc::new(tokio::sync::Barrier::new(3));
        let launch = |signed| {
            let f = Arc::clone(&f);
            let barrier = Arc::clone(&barrier);
            tokio::spawn(async move {
                barrier.wait().await;
                f.execute(signed, Ok(1_000_000), false).await
            })
        };
        let first = launch(a);
        let second = launch(b);
        barrier.wait().await;
        let results = [first.await?, second.await?];
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| matches!(r, Err(StaffError::Conflict)))
                .count(),
            1
        );
        let counts = f.counts().await?;
        assert_eq!(
            (counts.0, counts.1, counts.2, counts.4),
            (2, 1, 1, "2".into())
        );
        Arc::try_unwrap(f)
            .map_err(|_| "fixture still shared")?
            .cleanup()
            .await
    })
}

#[test]
fn staff_pg_receipt_requires_exact_digests_disposition_and_chain_then_retries() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("receipt").await? else {
            return Ok(());
        };
        let signed = f.signed(2, 2, f.trust.activation_digest(), true);
        let request_digest = signature::sha256(&signed.bytes);
        let stored = f
            .execute(signed, Ok(1_000_000), true)
            .await
            .map_err(debug)?;
        let receipt = Receipt {
            version: 1,
            environment: f.trust.environment().into(),
            roster_sha256: f.trust.roster_digest().into(),
            operation_id: f
                .signer
                .request(2, 2, f.trust.activation_digest())
                .operation_id,
            request_sha256: request_digest.clone(),
            outcome_sha256: signature::sha256(&stored.bytes),
            disposition: Disposition::Committed,
            issued_at: Decimal(1_000_000),
            control_seq: Decimal(3),
            previous_link_sha256: request_digest.clone(),
        };
        let submit = |value: Receipt| {
            f.trust.verify_receipt(
                f.signer.sign(
                    oteryn_release_manifest::jcs::encode(&value, signature::MAX_REQUEST_BYTES)
                        .expect("receipt JCS"),
                    RECEIPT_NAMESPACE,
                ),
            )
        };
        let before = f.counts().await?;
        let mut request_mismatch = receipt.clone();
        request_mismatch.request_sha256 = "e".repeat(64);
        let mut outcome_mismatch = receipt.clone();
        outcome_mismatch.outcome_sha256 = "e".repeat(64);
        let mut previous_mismatch = receipt.clone();
        previous_mismatch.previous_link_sha256 = "e".repeat(64);
        for altered in [request_mismatch, outcome_mismatch, previous_mismatch] {
            let verified = submit(altered).map_err(debug)?;
            let mut tx = f.pool.begin().await?;
            assert!(
                owner::receipt_in_transaction(&mut tx, &f.trust, &verified)
                    .await
                    .is_err()
            );
            tx.rollback().await?;
        }
        let mut refused = receipt.clone();
        refused.disposition = Disposition::Refused;
        let verified = submit(refused).map_err(debug)?;
        let mut tx = f.pool.begin().await?;
        assert!(
            owner::receipt_in_transaction(&mut tx, &f.trust, &verified)
                .await
                .is_err()
        );
        tx.rollback().await?;
        assert_eq!(f.counts().await?, before);
        let next = f.signed(3, 3, &request_digest, false);
        assert!(matches!(
            f.execute(next, Ok(1_000_000), false).await,
            Err(StaffError::Rejected(Error::Receipt))
        ));
        let verified = submit(receipt).map_err(debug)?;
        let mut tx = f.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        owner::receipt_in_transaction(&mut tx, &f.trust, &verified)
            .await
            .map_err(debug)?;
        tx.commit().await?;
        let mut tx = f.pool.begin().await?;
        owner::receipt_in_transaction(&mut tx, &f.trust, &verified)
            .await
            .map_err(debug)?;
        tx.commit().await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_staff_action_receipts")
            .fetch_one(&f.pool)
            .await?;
        assert_eq!(count, 1);
        f.cleanup().await
    })
}

#[test]
fn staff_pg_runtime_cannot_edit_delete_or_truncate_retained_evidence() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("grants").await? else {
            return Ok(());
        };
        f.execute(
            f.signed(2, 2, f.trust.activation_digest(), false),
            Ok(1_000_000),
            false,
        )
        .await
        .map_err(debug)?;
        for sql in [
            "UPDATE game_staff_operations SET principal='other'",
            "DELETE FROM game_staff_operations",
            "TRUNCATE game_staff_operations",
            "UPDATE game_staff_control_links SET principal='other'",
            "DELETE FROM game_staff_action_receipts",
        ] {
            let mut tx = f.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            let error = sqlx::query(sqlx::AssertSqlSafe(sql))
                .execute(&mut *tx)
                .await
                .expect_err("privilege must refuse");
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("42501")
            );
            tx.rollback().await?;
        }
        f.cleanup().await
    })
}

#[test]
fn staff_pg_member_logins_cannot_insert_activations() -> TestResult {
    async fn activation_attempt(
        connection: &mut sqlx::PgConnection,
        request: &signature::VerifiedRequest,
    ) -> Result<(), sqlx::Error> {
        let signed = request.signed();
        sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',2,$1,$2,$3,'ACTIVATION',$4,$5,$6,$7,txid_current())")
            .bind(request.digest())
            .bind(&request.request().previous_link_sha256)
            .bind(&request.request().roster_sha256)
            .bind(&signed.bytes)
            .bind(&signed.principal)
            .bind(request.fingerprint())
            .bind(&signed.signature)
            .execute(&mut *connection)
            .await?;
        let updated = sqlx::query("UPDATE game_staff_control_heads SET control_seq=2,link_sha256=$1,activation_sha256=$1 WHERE environment='alpha-test'")
            .bind(request.digest())
            .execute(&mut *connection)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
        sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
            .execute(connection)
            .await?;
        Ok(())
    }

    runtime()?.block_on(async {
        let Some(f) = Fixture::create("member_login").await? else {
            return Ok(());
        };
        let mut admin = sqlx::PgConnection::connect(&f.admin).await?;
        let mut created_roles = Vec::new();
        let result: TestResult<Vec<String>> = async {
            let before = f.counts().await?;
            let head_before: (String, String, String) = sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
                .fetch_one(&f.pool).await?;
            let mut activation = f.signer.request(2, 2, f.trust.activation_digest());
            activation.scope = None;
            activation.reason = staff::audit::Reason::Roster;
            activation.action = Action::RosterActivate {
                roster_sha256: f.trust.roster_digest().into(),
            };
            let signed = f.signer.sign(
                activation.canonical_bytes().map_err(debug)?,
                REQUEST_NAMESPACE,
            );
            let checked = f.trust.verify_request(signed).map_err(debug)?;

            // Independently provisioned owner activation remains legal. All
            // signed fields and deferred constraints match the negative cases.
            let mut owner_tx = f.pool.begin().await?;
            activation_attempt(&mut owner_tx, &checked).await?;
            owner_tx.rollback().await?;

            let nonce = f.name.rsplit('_').next().ok_or("fixture name")?;
            let mut accepted = Vec::new();
            for (label, group, indirect) in [
                ("runtime_direct", "oteryn_game_runtime", false),
                ("control_direct", "oteryn_game_control", false),
                ("runtime_indirect", "oteryn_game_runtime", true),
                ("control_indirect", "oteryn_game_control", true),
            ] {
                let login = format!("oteryn_gm_2181_{label}_{nonce}");
                let password = signature::sha256(&node::operator_files::random_bytes::<32>()?);
                // All interpolated identifiers and the password are generated
                // ASCII fixture values, never supplied by the admin URL.
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE ROLE {login} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
                )))
                .execute(&mut admin)
                .await?;
                created_roles.push(login.clone());
                let inherited = if indirect {
                    let bridge = format!("{login}_bridge");
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "CREATE ROLE {bridge} NOLOGIN INHERIT"
                    )))
                    .execute(&mut admin)
                    .await?;
                    created_roles.push(bridge.clone());
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "GRANT {group} TO {bridge} WITH INHERIT TRUE"
                    )))
                    .execute(&mut admin)
                    .await?;
                    bridge
                } else {
                    group.into()
                };
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "GRANT {inherited} TO {login} WITH INHERIT TRUE"
                )))
                .execute(&mut admin)
                .await?;
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "GRANT CONNECT ON DATABASE {} TO {login}",
                    f.name
                )))
                .execute(&mut admin)
                .await?;

                let options = f
                    .url
                    .parse::<sqlx::postgres::PgConnectOptions>()?
                    .username(&login)
                    .password(&password);
                let mut connection = sqlx::PgConnection::connect_with(&options).await?;
                // A real LOGIN connection, with no SET ROLE: inherited INSERT
                // and head UPDATE must be available before the guard is tried.
                let identity: (String, String, bool, bool, bool, bool) = sqlx::query_as("SELECT current_user::text,session_user::text,pg_has_role(current_user,$1::name,'USAGE'),has_table_privilege(current_user,'game_staff_control_links','INSERT'),has_column_privilege(current_user,'game_staff_control_heads','control_seq','UPDATE'),has_column_privilege(current_user,'game_staff_control_heads','link_sha256','UPDATE')")
                    .bind(group).fetch_one(&mut connection).await?;
                if identity != (login.clone(), login, true, true, true, true) {
                    return Err(format!("invalid LOGIN membership fixture: {label}").into());
                }
                let mut tx = connection.begin().await?;
                let attempt = activation_attempt(&mut tx, &checked).await;
                tx.rollback().await?;
                connection.close().await?;
                match attempt {
                    Ok(()) => accepted.push(label.to_owned()),
                    Err(error) => {
                        let database = error.as_database_error().ok_or_else(|| debug(&error))?;
                        if database.code().as_deref() != Some("23514")
                            || database.message() != "runtime activation forbidden"
                        {
                            return Err(debug(error));
                        }
                    }
                }
                let head_after: (String, String, String) = sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
                    .fetch_one(&f.pool).await?;
                if f.counts().await? != before || head_after != head_before {
                    return Err(format!("activation attempt changed retained state: {label}").into());
                }
            }
            Ok(accepted)
        }
        .await;

        // Roles are cluster-global. Drop only this fixture's database and
        // successfully created random roles, including before the RED assert.
        let database_cleanup = f.cleanup().await;
        for role in created_roles.iter().rev() {
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {role}")))
                .execute(&mut admin)
                .await?;
        }
        admin.close().await?;
        database_cleanup?;
        let accepted = result?;
        assert!(
            accepted.is_empty(),
            "runtime/control member logins inserted activations: {accepted:?}"
        );
        Ok(())
    })
}

#[test]
fn staff_pg_forensic_scan_detects_orphan_outbox_after_owner_bypasses_foreign_keys() -> TestResult {
    runtime()?.block_on(async {
        let Some(f)=Fixture::create("orphan").await? else {return Ok(());};
        let mut tx=f.pool.begin().await?;
        assert!(!owner::evidence_orphans_in_transaction(&mut tx,"alpha-test",1).await.map_err(debug)?);
        tx.rollback().await?;
        let mut tx=f.pool.begin().await?;
        // Explicit hostile database-owner operator; runtime cannot bypass its FKs.
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_staff_action_outbox(environment,operation_id,event_id,transaction_id,event_type_id,schema_revision,retention_profile_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,created_xact_id) VALUES('alpha-test','00000000-0000-7000-8000-000000000061','00000000-0000-7000-8000-000000000062','00000000-0000-7000-8000-000000000063',$2,1,'STAFF_ACTION_AUDIT_RETENTION_V1',1000000,31537000000,$1,encode(sha256($1),'hex'),1,txid_current())")
            .bind(vec![1_u8]).bind(i64::from(Contract::registered().map_err(debug)?.event_type())).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx=f.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        assert!(owner::evidence_orphans_in_transaction(&mut tx,"alpha-test",1).await.map_err(debug)?);
        tx.rollback().await?;f.cleanup().await
    })
}

#[test]
fn staff_pg_first_execution_samples_clock_after_actual_head_lock_wait() -> TestResult {
    runtime()?.block_on(async {
        use std::sync::atomic::{AtomicU64, Ordering};
        let Some(f) = Fixture::create("clockwait").await? else {
            return Ok(());
        };
        let before = f.counts().await?;
        let request = f
            .trust
            .verify_request(f.signed(2, 2, f.trust.activation_digest(), false))
            .map_err(debug)?;
        let mut blocker = f.pool.begin().await?;
        owner::lock_head(&mut blocker, "alpha-test")
            .await
            .map_err(debug)?;
        let pool = f.pool.clone();
        let trust = Arc::clone(&f.trust);
        let history = Arc::clone(&f.history);
        let now = Arc::new(AtomicU64::new(1_000_000));
        let calls = Arc::new(AtomicU64::new(0));
        let sample = Arc::clone(&now);
        let observed = Arc::clone(&calls);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let waiter = tokio::spawn(async move {
            let mut tx = pool.begin().await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *tx)
                .await?;
            sender
                .send(pid)
                .map_err(|_| StaffError::Rejected(Error::Invalid))?;
            let result =
                owner::fixture_begin_with_clock(&mut tx, &trust, &history, request, move || {
                    observed.fetch_add(1, Ordering::SeqCst);
                    Ok(sample.load(Ordering::SeqCst))
                })
                .await;
            tx.rollback().await?;
            result
        });
        let pid = receiver.await?;
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let blocked: i32 = sqlx::query_scalar("SELECT cardinality(pg_blocking_pids($1))")
                    .bind(pid)
                    .fetch_one(&f.pool)
                    .await?;
                if blocked > 0 {
                    break Ok::<_, sqlx::Error>(());
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await??;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        now.store(1_600_001, Ordering::SeqCst);
        blocker.rollback().await?;
        assert!(matches!(
            waiter.await?,
            Err(StaffError::Rejected(Error::Stale))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(f.counts().await?, before);
        f.cleanup().await
    })
}

#[test]
fn staff_pg_valid_envelope_does_not_hide_forged_sql_routing_or_outcome_hash() -> TestResult {
    runtime()?.block_on(async {
        let Some(f)=Fixture::create("mirrors").await? else {return Ok(());};
        let contract=Contract::registered().map_err(debug)?;
        let different_event_type=contract.event_type().checked_add(1).ok_or("event type overflow")?;
        let signed=f.signed(2,2,f.trust.activation_digest(),false);
        let verified=f.trust.verify_request(signed.clone()).map_err(debug)?;
        let outcome=f.execute(signed,Ok(1_000_000),false).await.map_err(debug)?;
        let root=durability::DurabilityRoot::connect_test_runtime(&f.url)?;
        assert!(root.maintain_ready_once().await?);
        let rows=root.staff_evidence_page("alpha-test".into(),0,2).await.map_err(debug)?;
        let event=rows[1].operation.as_ref().and_then(|o|o.envelope.as_ref()).ok_or("missing audit event")?;
        assert!(staff::audit::verify_stored_event(contract,&verified,&outcome.outcome,event).is_ok());
        // Legal SQL shape, hostile routing mirror; the signed event is unchanged.
        sqlx::query("UPDATE game_staff_action_outbox SET event_type_id=$1 WHERE environment='alpha-test'").bind(i64::from(different_event_type)).execute(&f.pool).await?;
        let rows=root.staff_evidence_page("alpha-test".into(),0,2).await.map_err(debug)?;
        let event=rows[1].operation.as_ref().and_then(|o|o.envelope.as_ref()).ok_or("missing audit event")?;
        assert!(staff::audit::verify_stored_event(contract,&verified,&outcome.outcome,event).is_err());
        sqlx::query("UPDATE game_staff_action_outbox SET event_type_id=$1 WHERE environment='alpha-test'").bind(i64::from(contract.event_type())).execute(&f.pool).await?;
        // Hostile owner bypasses only the hash CHECK/immutability; all other
        // row values and cryptographic evidence remain valid.
        sqlx::raw_sql("DO $$ DECLARE c record; BEGIN FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_staff_operations'::regclass AND contype='c' AND pg_get_constraintdef(oid) LIKE '%outcome_sha256%' LOOP EXECUTE format('ALTER TABLE game_staff_operations DROP CONSTRAINT %I',c.conname); END LOOP; END $$").execute(&f.pool).await?;
        let mut tx=f.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_staff_operations SET outcome_sha256=repeat('f',64),created_xact_id=txid_current() WHERE environment='alpha-test'").execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(root.staff_evidence_has_orphans("alpha-test".into(),2).await.map_err(debug)?);
        drop(root);f.cleanup().await
    })
}

#[test]
fn staff_pg_expired_outbox_cannot_hide_forged_operation_identity() -> TestResult {
    runtime()?.block_on(async {
        let Some(f)=Fixture::create("expiredmirror").await? else {return Ok(());};
        let signed=f.signed(2,2,f.trust.activation_digest(),false);
        let verified=f.trust.verify_request(signed.clone()).map_err(debug)?;
        let outcome=f.execute(signed,Ok(1_000_000),false).await.map_err(debug)?;
        let root=durability::DurabilityRoot::connect_test_runtime(&f.url)?;
        assert!(root.maintain_ready_once().await?);
        // Explicit owner-side pruning fixture, after the signed conservative
        // retention boundary; production pruning remains a held lifecycle slot.
        let expiry=staff::audit::evidence_expiry(&verified,&outcome.outcome,None).map_err(debug)?;
        let verification_time=expiry.checked_add(1).ok_or("expiry overflow")?;
        sqlx::query("DELETE FROM game_staff_action_outbox WHERE environment='alpha-test'").execute(&f.pool).await?;
        let rows=root.staff_evidence_page("alpha-test".into(),0,2).await.map_err(debug)?;
        let operation=rows[1].operation.as_ref().ok_or("missing retained outcome")?;
        assert!(operation.envelope.is_none());
        assert!(verification_time>=staff::audit::evidence_expiry(&verified,&outcome.outcome,None).map_err(debug)?);
        assert!(staff::audit::verify_stored_operation(&verified,&operation.record).is_ok());
        // Retain the original local transaction stamp, changing ONLY the UUID
        // mirror. The hostile owner can remove insert-time table checks.
        sqlx::raw_sql("DO $$ DECLARE c record; BEGIN FOR c IN SELECT conname FROM pg_constraint WHERE conrelid='game_staff_operations'::regclass AND contype='c' AND pg_get_constraintdef(oid) LIKE '%created_xact_id%' LOOP EXECUTE format('ALTER TABLE game_staff_operations DROP CONSTRAINT %I',c.conname); END LOOP; END $$").execute(&f.pool).await?;
        let mut tx=f.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_staff_operations SET operation_id='00000000-0000-7000-8000-000000000065' WHERE environment='alpha-test'").execute(&mut *tx).await?;
        tx.commit().await?;
        let rows=root.staff_evidence_page("alpha-test".into(),0,2).await.map_err(debug)?;
        let operation=rows[1].operation.as_ref().ok_or("missing forged retained outcome")?;
        assert!(operation.envelope.is_none());
        assert!(staff::audit::verify_stored_operation(&verified,&operation.record).is_err());
        drop(root);f.cleanup().await
    })
}

#[test]
fn staff_pg_member_logins_cannot_commit_receipts_without_matching_links() -> TestResult {
    fn signed_receipt(
        f: &Fixture,
        operation: u8,
        sequence: u64,
        request_digest: &str,
        stored: &StoredOutcome,
    ) -> TestResult<signature::VerifiedReceipt> {
        let receipt = Receipt {
            version: 1,
            environment: f.trust.environment().into(),
            roster_sha256: f.trust.roster_digest().into(),
            operation_id: f
                .signer
                .request(operation, sequence, request_digest)
                .operation_id,
            request_sha256: request_digest.into(),
            outcome_sha256: signature::sha256(&stored.bytes),
            disposition: Disposition::Committed,
            issued_at: Decimal(1_000_000),
            control_seq: Decimal(sequence),
            previous_link_sha256: request_digest.into(),
        };
        let bytes = oteryn_release_manifest::jcs::encode(&receipt, signature::MAX_REQUEST_BYTES)
            .map_err(debug)?;
        f.trust
            .verify_receipt(f.signer.sign(bytes, RECEIPT_NAMESPACE))
            .map_err(debug)
    }

    async fn commit_receipt(
        connection: &mut sqlx::PgConnection,
        f: &Fixture,
        receipt: &signature::VerifiedReceipt,
    ) -> TestResult {
        let mut tx = connection.begin().await?;
        owner::receipt_in_transaction(&mut tx, &f.trust, receipt)
            .await
            .map_err(debug)?;
        tx.commit().await?;
        Ok(())
    }

    async fn run_role(label: &str, group: &str) -> TestResult<Vec<String>> {
        // Each real COMMIT attack has its own database. On the old migration an
        // accepted row must not mask the second role with a primary-key error.
        let Some(f) = Fixture::create(label).await? else {
            return Ok(Vec::new());
        };
        let mut admin = sqlx::PgConnection::connect(&f.admin).await?;
        let mut created_roles = Vec::new();
        let result: TestResult<Vec<String>> = async {
            let nonce = f.name.rsplit('_').next().ok_or("fixture name")?;
            let login = format!("oteryn_gm_2181_{label}_{nonce}");
            let password = signature::sha256(&node::operator_files::random_bytes::<32>()?);
            // Only generated ASCII fixture identifiers/passwords are interpolated.
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE ROLE {login} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
            )))
            .execute(&mut admin)
            .await?;
            created_roles.push(login.clone());
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT {group} TO {login} WITH INHERIT TRUE"
            )))
            .execute(&mut admin)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT CONNECT ON DATABASE {} TO {login}",
                f.name
            )))
            .execute(&mut admin)
            .await?;
            let options = f
                .url
                .parse::<sqlx::postgres::PgConnectOptions>()?
                .username(&login)
                .password(&password);
            let mut connection = sqlx::PgConnection::connect_with(&options).await?;
            let identity: (String, String, bool, bool, bool, bool, bool) = sqlx::query_as("SELECT current_user::text,session_user::text,pg_has_role(current_user,$1::name,'USAGE'),has_table_privilege(current_user,'game_staff_action_receipts','INSERT'),has_table_privilege(current_user,'game_staff_control_links','INSERT'),has_column_privilege(current_user,'game_staff_control_heads','control_seq','UPDATE'),has_column_privilege(current_user,'game_staff_control_heads','link_sha256','UPDATE')")
                .bind(group).fetch_one(&mut connection).await?;
            if identity != (login.clone(), login, true, true, true, true, true) {
                return Err(format!("invalid receipt LOGIN fixture: {label}").into());
            }

            // A lawful receipt is committed in a later transaction than its
            // operation, by this actual LOGIN, on both old and repaired SQL.
            let first = f.signed(2, 2, f.trust.activation_digest(), true);
            let first_digest = signature::sha256(&first.bytes);
            let first_outcome = f.execute(first, Ok(1_000_000), true).await.map_err(debug)?;
            let first_receipt = signed_receipt(&f, 2, 3, &first_digest, &first_outcome)?;
            commit_receipt(&mut connection, &f, &first_receipt).await?;
            let provenance: (bool, bool, bool) = sqlx::query_as("SELECT r.created_xact_id=l.created_xact_id,r.created_xact_id<>o.created_xact_id,r.receipt_bytes=l.signed_bytes AND r.principal=l.principal AND r.key_fingerprint=l.key_fingerprint AND r.signature=l.signature FROM game_staff_action_receipts r JOIN game_staff_control_links l ON l.environment=r.environment AND l.control_seq=r.control_seq AND l.link_kind='RECEIPT' JOIN game_staff_operations o ON o.environment=r.environment AND o.operation_id=r.operation_id")
                .fetch_one(&f.pool).await?;
            if provenance != (true, true, true) {
                return Err(format!("lawful receipt transaction binding changed: {label}").into());
            }
            let after_receipt = f.counts().await?;
            commit_receipt(&mut connection, &f, &first_receipt).await?;
            let receipt_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_staff_action_receipts")
                .fetch_one(&f.pool).await?;
            if f.counts().await? != after_receipt || receipt_count != 1 {
                return Err(format!("identical receipt retry changed evidence: {label}").into());
            }

            // A second committed mutation now requires reconciliation. Its valid
            // signed receipt bytes are deliberately routed to the existing
            // REQUEST sequence; no new RECEIPT link or head update is inserted.
            let first_receipt_digest = signature::sha256(&first_receipt.signed().bytes);
            let pending = f.signed(4, 4, &first_receipt_digest, true);
            let pending_digest = signature::sha256(&pending.bytes);
            let pending_outcome = f.execute(pending, Ok(1_000_000), false).await.map_err(debug)?;
            let receipt = signed_receipt(&f, 4, 5, &pending_digest, &pending_outcome)?;
            let before = f.counts().await?;
            let head_before: (String, String, String) = sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
                .fetch_one(&f.pool).await?;
            let pending_sequence: String = sqlx::query_scalar("SELECT control_seq::text FROM game_staff_operations WHERE environment=$1 AND operation_id=$2::text::uuid AND requires_receipt")
                .bind(f.trust.environment()).bind(&receipt.receipt().operation_id)
                .fetch_one(&f.pool).await?;
            let next = f.signed(5, 5, &pending_digest, false);
            if !matches!(
                f.execute(next.clone(), Ok(1_000_000), false).await,
                Err(StaffError::Rejected(Error::Receipt))
            ) {
                return Err(format!("missing-receipt fence absent before attack: {label}").into());
            }
            let mut tx = connection.begin().await?;
            sqlx::query("INSERT INTO game_staff_action_receipts(environment,operation_id,receipt_bytes,principal,key_fingerprint,signature,control_seq,created_xact_id) VALUES($1,$2::text::uuid,$3,$4,$5,$6,$7::text::numeric,txid_current())")
                .bind(f.trust.environment())
                .bind(&receipt.receipt().operation_id)
                .bind(&receipt.signed().bytes)
                .bind(&receipt.signed().principal)
                .bind(receipt.fingerprint())
                .bind(&receipt.signed().signature)
                .bind(&pending_sequence)
                .execute(&mut *tx).await?;
            // Test the real deferred COMMIT boundary, without naming a trigger
            // that does not exist in the genuine RED migration.
            let attempt = tx.commit().await;
            let mut failures = Vec::new();
            match attempt {
                Ok(()) => {
                    failures.push(format!("{label}: direct receipt COMMIT accepted"));
                    let unpaired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_staff_action_receipts r JOIN game_staff_control_links l ON l.environment=r.environment AND l.control_seq=r.control_seq WHERE l.link_kind='REQUEST'")
                        .fetch_one(&f.pool).await?;
                    if unpaired != 1 {
                        return Err(format!("accepted direct receipt was not retained: {label}").into());
                    }
                    let request = f.trust.verify_request(next).map_err(debug)?;
                    let mut probe = connection.begin().await?;
                    let bypass = owner::fixture_begin_in_transaction(
                        &mut probe, &f.trust, &f.history, request, Ok(1_000_000)
                    ).await;
                    probe.rollback().await?;
                    if !matches!(bypass, Ok(Begin::Fresh(_))) {
                        return Err(format!("direct receipt did not reproduce the missing-receipt bypass: {label}").into());
                    }
                }
                Err(error) => {
                    let database = error.as_database_error().ok_or_else(|| debug(&error))?;
                    if database.code().as_deref() != Some("23514")
                        || database.message() != "staff receipt has no same-transaction link"
                    {
                        return Err(debug(error));
                    }
                    // A failed deferred COMMIT must leave the earlier committed
                    // mutation intact and must not discharge its receipt fence.
                    let head_after: (String, String, String) = sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
                        .fetch_one(&f.pool).await?;
                    let receipts_after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_staff_action_receipts")
                        .fetch_one(&f.pool).await?;
                    if f.counts().await? != before || head_after != head_before || receipts_after != 1 {
                        return Err(format!("failed receipt COMMIT changed retained state: {label}").into());
                    }
                    if !matches!(
                        f.execute(next, Ok(1_000_000), false).await,
                        Err(StaffError::Rejected(Error::Receipt))
                    ) {
                        return Err(format!("failed receipt COMMIT discharged the receipt fence: {label}").into());
                    }
                    commit_receipt(&mut connection, &f, &receipt).await?;
                    let reconciled = f.counts().await?;
                    commit_receipt(&mut connection, &f, &receipt).await?;
                    if f.counts().await? != reconciled {
                        return Err(format!("post-refusal receipt retry changed retained state: {label}").into());
                    }
                    let resumed = f.signed(6, 6, &signature::sha256(&receipt.signed().bytes), false);
                    f.execute(resumed, Ok(1_000_000), false).await.map_err(debug)?;
                }
            }
            connection.close().await?;
            Ok(failures)
        }
        .await;

        // These are the only database/cluster roles owned by this invocation.
        // Even an accepted RED COMMIT is fully cleaned before final assertions.
        let database_cleanup = f.cleanup().await;
        let mut role_cleanup = Ok(());
        for role in created_roles.iter().rev() {
            let removed = sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {role}")))
                .execute(&mut admin)
                .await;
            if let Err(error) = removed {
                role_cleanup = Err(error);
            }
        }
        let admin_cleanup = admin.close().await;
        database_cleanup?;
        role_cleanup?;
        admin_cleanup?;
        result
    }

    runtime()?.block_on(async {
        let mut failures = Vec::new();
        for (label, group) in [
            ("runtime_receipt", "oteryn_game_runtime"),
            ("control_receipt", "oteryn_game_control"),
        ] {
            failures.extend(run_role(label, group).await?);
        }
        assert!(
            failures.is_empty(),
            "receipt link commit failures: {failures:?}"
        );
        Ok(())
    })
}

#[test]
fn staff_pg_forged_receipt_commit_cannot_release_fence_for_member_logins() -> TestResult {
    #[derive(Clone, Copy)]
    enum Forgery {
        Signature,
        Binding,
    }

    async fn head_row(pool: &sqlx::PgPool) -> TestResult<(String, String, String)> {
        Ok(sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
            .fetch_one(pool).await?)
    }

    async fn run_role(label: &str, group: &str, forgery: Forgery) -> TestResult<Vec<String>> {
        let Some(f) = Fixture::create(label).await? else {
            return Ok(Vec::new());
        };
        let mut admin = sqlx::PgConnection::connect(&f.admin).await?;
        let mut created_roles = Vec::new();
        let result: TestResult<Vec<String>> = async {
            let nonce = f.name.rsplit('_').next().ok_or("fixture name")?;
            let login = format!("oteryn_gm_2181_{label}_{nonce}");
            let password = signature::sha256(&node::operator_files::random_bytes::<32>()?);
            // Only generated ASCII fixture identifiers/passwords are interpolated.
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE ROLE {login} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
            )))
            .execute(&mut admin)
            .await?;
            created_roles.push(login.clone());
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT {group} TO {login} WITH INHERIT TRUE"
            )))
            .execute(&mut admin)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT CONNECT ON DATABASE {} TO {login}",
                f.name
            )))
            .execute(&mut admin)
            .await?;
            let options = f
                .url
                .parse::<sqlx::postgres::PgConnectOptions>()?
                .username(&login)
                .password(&password);
            let mut connection = sqlx::PgConnection::connect_with(&options).await?;
            let identity: (String, String, bool, bool) = sqlx::query_as("SELECT current_user::text,session_user::text,pg_has_role(current_user,$1::name,'USAGE'),pg_has_role(current_user,'oteryn_test_admin','USAGE')")
                .bind(group).fetch_one(&mut connection).await?;
            if identity != (login.clone(), login, true, false) {
                return Err(format!("invalid forged-receipt LOGIN fixture: {label}").into());
            }

            // A lawful committed mutation leaves the receipt fence pending.
            let mutation = f.signed(2, 2, f.trust.activation_digest(), true);
            let mutation_digest = signature::sha256(&mutation.bytes);
            let stored = f
                .execute(mutation.clone(), Ok(1_000_000), true)
                .await
                .map_err(debug)?;
            let mut receipt = Receipt {
                version: 1,
                environment: f.trust.environment().into(),
                roster_sha256: f.trust.roster_digest().into(),
                operation_id: f
                    .signer
                    .request(2, 2, f.trust.activation_digest())
                    .operation_id,
                request_sha256: mutation_digest.clone(),
                outcome_sha256: signature::sha256(&stored.bytes),
                disposition: Disposition::Committed,
                issued_at: Decimal(1_000_000),
                control_seq: Decimal(3),
                previous_link_sha256: mutation_digest.clone(),
            };
            let encode = |value: &Receipt| {
                oteryn_release_manifest::jcs::encode(value, signature::MAX_REQUEST_BYTES)
                    .map_err(debug)
            };
            let (forged, fingerprint, refusal) = match forgery {
                Forgery::Signature => {
                    // A genuine rostered signature over other receipt bytes is
                    // attached to the exact bound receipt bytes.
                    let mut other = receipt.clone();
                    other.issued_at = Decimal(1_000_001);
                    let donor = f.signer.sign(encode(&other)?, RECEIPT_NAMESPACE);
                    let fingerprint = f
                        .trust
                        .verify_receipt(donor.clone())
                        .map_err(debug)?
                        .fingerprint()
                        .to_owned();
                    let forged = signature::SignedBytes {
                        bytes: encode(&receipt)?,
                        signature: donor.signature,
                        principal: donor.principal,
                    };
                    if !matches!(
                        f.trust.verify_receipt(forged.clone()),
                        Err(Error::Signature)
                    ) {
                        return Err(format!("forged signature verified: {label}").into());
                    }
                    (forged, fingerprint, Error::Signature)
                }
                Forgery::Binding => {
                    // A valid signature by the rostered key binds another outcome.
                    receipt.outcome_sha256 = "e".repeat(64);
                    let signed = f.signer.sign(encode(&receipt)?, RECEIPT_NAMESPACE);
                    let fingerprint = f
                        .trust
                        .verify_receipt(signed.clone())
                        .map_err(debug)?
                        .fingerprint()
                        .to_owned();
                    (signed, fingerprint, Error::Receipt)
                }
            };
            let forged_digest = signature::sha256(&forged.bytes);

            // The actual LOGIN commits structurally complete forged evidence:
            // RECEIPT link, receipt row and head advance in one transaction.
            let mut tx = connection.begin().await?;
            sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',3,$1,$2,$3,'RECEIPT',$4,$5,$6,$7,txid_current())")
                .bind(&forged_digest).bind(&mutation_digest).bind(f.trust.roster_digest()).bind(&forged.bytes).bind(&forged.principal).bind(&fingerprint).bind(&forged.signature)
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_staff_action_receipts(environment,operation_id,receipt_bytes,principal,key_fingerprint,signature,control_seq,created_xact_id) VALUES('alpha-test',$1::text::uuid,$2,$3,$4,$5,3,txid_current())")
                .bind(&receipt.operation_id).bind(&forged.bytes).bind(&forged.principal).bind(&fingerprint).bind(&forged.signature)
                .execute(&mut *tx).await?;
            let advanced = sqlx::query("UPDATE game_staff_control_heads SET control_seq=3,link_sha256=$1 WHERE environment='alpha-test' AND control_seq=2 AND link_sha256=$2")
                .bind(&forged_digest).bind(&mutation_digest)
                .execute(&mut *tx).await?;
            if advanced.rows_affected() != 1 {
                return Err(format!("forged head advance missed: {label}").into());
            }
            tx.commit()
                .await
                .map_err(|error| format!("forged receipt COMMIT refused before the fence: {label}: {error:?}"))?;

            let before = f.counts().await?;
            let head_before = head_row(&f.pool).await?;
            if head_before.1 != forged_digest {
                return Err(format!("forged head was not committed: {label}").into());
            }
            let mut failures = Vec::new();
            let root = durability::DurabilityRoot::connect_test_runtime(&f.url)?;
            if !root.maintain_ready_once().await? {
                return Err(format!("fixture root not ready: {label}").into());
            }
            let next = f.signed(4, 4, &forged_digest, false);
            for attempt in 0..2 {
                match root
                    .staff_prepare_head(f.trust.environment().into(), Arc::clone(&f.history))
                    .await
                {
                    Ok(_) => failures.push(format!("{label}: preparation admitted ({attempt})")),
                    Err(StaffError::Rejected(error)) if error == refusal => {}
                    Err(error) => return Err(debug(error)),
                }
                let request = f.trust.verify_request(next.clone()).map_err(debug)?;
                let mut probe = connection.begin().await?;
                let begun = owner::fixture_begin_in_transaction(
                    &mut probe,
                    &f.trust,
                    &f.history,
                    request,
                    Ok(1_000_000),
                )
                .await;
                probe.rollback().await?;
                match begun {
                    Ok(Begin::Fresh(_)) => {
                        failures.push(format!("{label}: next valid request admitted ({attempt})"))
                    }
                    Ok(Begin::Replay(_)) => return Err(format!("next request replayed: {label}").into()),
                    Err(StaffError::Rejected(error)) if error == refusal => {}
                    Err(error) => return Err(debug(error)),
                }
                // The identical retry of the fenced mutation keeps its stored
                // outcome; refusals and retries leave the fence in place.
                if f.execute(mutation.clone(), Err(Error::Clock), true)
                    .await
                    .map_err(debug)?
                    != stored
                {
                    return Err(format!("identical mutation retry changed: {label}").into());
                }
                if f.counts().await? != before || head_row(&f.pool).await? != head_before {
                    return Err(format!("fenced refusal changed retained state: {label}").into());
                }
            }
            drop(root);
            connection.close().await?;
            Ok(failures)
        }
        .await;

        // These are the only database/cluster roles owned by this invocation.
        let database_cleanup = f.cleanup().await;
        let mut role_cleanup = Ok(());
        for role in created_roles.iter().rev() {
            let removed = sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {role}")))
                .execute(&mut admin)
                .await;
            if let Err(error) = removed {
                role_cleanup = Err(error);
            }
        }
        let admin_cleanup = admin.close().await;
        database_cleanup?;
        role_cleanup?;
        admin_cleanup?;
        result
    }

    runtime()?.block_on(async {
        let mut failures = Vec::new();
        for (label, group, forgery) in [
            (
                "runtime_forged_sig",
                "oteryn_game_runtime",
                Forgery::Signature,
            ),
            (
                "control_forged_sig",
                "oteryn_game_control",
                Forgery::Signature,
            ),
            (
                "runtime_forged_bind",
                "oteryn_game_runtime",
                Forgery::Binding,
            ),
            (
                "control_forged_bind",
                "oteryn_game_control",
                Forgery::Binding,
            ),
        ] {
            failures.extend(run_role(label, group, forgery).await?);
        }
        assert!(
            failures.is_empty(),
            "forged receipt fence failures: {failures:?}"
        );
        Ok(())
    })
}

#[test]
fn staff_pg_valid_receipt_across_rotation_releases_fence_under_its_own_activation() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("rotation_fence").await? else {
            return Ok(());
        };
        let result: TestResult = async {
            let mutation = f.signed(2, 2, f.trust.activation_digest(), true);
            let mutation_digest = signature::sha256(&mutation.bytes);
            let stored = f
                .execute(mutation.clone(), Ok(1_000_000), true)
                .await
                .map_err(debug)?;
            let receipt = Receipt {
                version: 1,
                environment: f.trust.environment().into(),
                roster_sha256: f.trust.roster_digest().into(),
                operation_id: f
                    .signer
                    .request(2, 2, f.trust.activation_digest())
                    .operation_id,
                request_sha256: mutation_digest.clone(),
                outcome_sha256: signature::sha256(&stored.bytes),
                disposition: Disposition::Committed,
                issued_at: Decimal(1_000_000),
                control_seq: Decimal(3),
                previous_link_sha256: mutation_digest.clone(),
            };
            let verified = f
                .trust
                .verify_receipt(
                    f.signer.sign(
                        oteryn_release_manifest::jcs::encode(
                            &receipt,
                            signature::MAX_REQUEST_BYTES,
                        )
                        .map_err(debug)?,
                        RECEIPT_NAMESPACE,
                    ),
                )
                .map_err(debug)?;
            let mut tx = f.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            owner::receipt_in_transaction(&mut tx, &f.trust, &verified)
                .await
                .map_err(debug)?;
            tx.commit().await?;
            let receipt_digest = signature::sha256(&verified.signed().bytes);

            // Rotate to an independent roster B after roster A's receipt.
            let replacement = Signer::new();
            let activation = signed_activation(&replacement, "alpha-test", 4, &receipt_digest)?;
            commit_activation(&f.pool, &activation).await?;
            let current = replacement.trust("alpha-test", activation.digest());
            if current.verify_receipt(verified.signed().clone()).is_ok() {
                return Err("current roster unexpectedly verifies the roster-A receipt".into());
            }
            let history = Arc::new(
                History::from_reviewed(
                    vec![
                        signature::Activation {
                            sequence: 1,
                            trust: f.signer.trust("alpha-test", f.trust.activation_digest()),
                        },
                        signature::Activation {
                            sequence: 4,
                            trust: replacement.trust("alpha-test", activation.digest()),
                        },
                    ],
                    &current,
                )
                .map_err(debug)?,
            );
            let root = durability::DurabilityRoot::connect_test_runtime(&f.url)?;
            if !root.maintain_ready_once().await? {
                return Err("fixture root not ready".into());
            }
            let head = root
                .staff_prepare_head("alpha-test".into(), Arc::clone(&history))
                .await
                .map_err(debug)?;
            if head.sequence != 4 || head.digest != activation.digest() {
                return Err("rotation preparation returned the wrong head".into());
            }

            // Trust is never taken from the current roster or a history that
            // lacks the receipt's activation: both fail closed.
            let current_only = Arc::new(
                History::from_reviewed(
                    vec![signature::Activation {
                        sequence: 1,
                        trust: replacement.trust("alpha-test", activation.digest()),
                    }],
                    &current,
                )
                .map_err(debug)?,
            );
            let missing = Arc::new(
                History::from_reviewed(
                    vec![signature::Activation {
                        sequence: 4,
                        trust: replacement.trust("alpha-test", activation.digest()),
                    }],
                    &current,
                )
                .map_err(debug)?,
            );
            let before = f.counts().await?;
            if !matches!(
                root.staff_prepare_head("alpha-test".into(), Arc::clone(&current_only))
                    .await,
                Err(StaffError::Rejected(Error::Signature))
            ) {
                return Err("current-roster history released the fence".into());
            }
            if !matches!(
                root.staff_prepare_head("alpha-test".into(), Arc::clone(&missing))
                    .await,
                Err(StaffError::Rejected(Error::MissingMaterial))
            ) {
                return Err("history without the receipt's activation released the fence".into());
            }
            let request = replacement.request(5, 5, activation.digest());
            let next =
                replacement.sign(request.canonical_bytes().map_err(debug)?, REQUEST_NAMESPACE);
            if !matches!(
                f.execute_with(&current, &missing, next.clone(), Ok(1_000_000), false)
                    .await,
                Err(StaffError::Rejected(Error::MissingMaterial))
            ) || f.counts().await? != before
            {
                return Err("request admitted without the receipt's activation".into());
            }

            // The lawful roster-A receipt releases the fence under roster B.
            let admitted = f
                .execute_with(&current, &history, next.clone(), Ok(1_000_000), false)
                .await
                .map_err(debug)?;
            let after = f.counts().await?;
            // Identical retries return the stored outcomes and change nothing.
            if after.4 != "5"
                || f.execute_with(&current, &history, next, Err(Error::Clock), true)
                    .await
                    .map_err(debug)?
                    != admitted
                || f.execute(mutation, Err(Error::Clock), true)
                    .await
                    .map_err(debug)?
                    != stored
                || f.counts().await? != after
            {
                return Err("rotation admission or identical retries changed".into());
            }
            drop(root);
            Ok(())
        }
        .await;
        f.cleanup().await?;
        result
    })
}

#[test]
fn staff_pg_member_logins_cannot_roll_back_head_or_get_forged_links_extended() -> TestResult {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Next {
        Request,
        Receipt,
    }

    async fn head_row(pool: &sqlx::PgPool) -> TestResult<(String, String, String)> {
        Ok(sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
            .fetch_one(pool).await?)
    }

    async fn run_role(label: &str, group: &str, next: Next) -> TestResult<Vec<String>> {
        let Some(f) = Fixture::create(label).await? else {
            return Ok(Vec::new());
        };
        let mut admin = sqlx::PgConnection::connect(&f.admin).await?;
        let mut created_roles = Vec::new();
        let result: TestResult<Vec<String>> = async {
            let nonce = f.name.rsplit('_').next().ok_or("fixture name")?;
            let login = format!("oteryn_gm_2181_{label}_{nonce}");
            let password = signature::sha256(&node::operator_files::random_bytes::<32>()?);
            // Only generated ASCII fixture identifiers/passwords are interpolated.
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE ROLE {login} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
            )))
            .execute(&mut admin)
            .await?;
            created_roles.push(login.clone());
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT {group} TO {login} WITH INHERIT TRUE"
            )))
            .execute(&mut admin)
            .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT CONNECT ON DATABASE {} TO {login}",
                f.name
            )))
            .execute(&mut admin)
            .await?;
            let options = f
                .url
                .parse::<sqlx::postgres::PgConnectOptions>()?
                .username(&login)
                .password(&password);
            let mut connection = sqlx::PgConnection::connect_with(&options).await?;
            let identity: (String, String, bool, bool) = sqlx::query_as("SELECT current_user::text,session_user::text,pg_has_role(current_user,$1::name,'USAGE'),pg_has_role(current_user,'oteryn_test_admin','USAGE')")
                .bind(group).fetch_one(&mut connection).await?;
            if identity != (login.clone(), login, true, false) {
                return Err(format!("invalid forged-link LOGIN fixture: {label}").into());
            }
            let mut failures = Vec::new();

            // Two lawful links, then a direct rollback of the head to the older one.
            let second = f.signed(2, 2, f.trust.activation_digest(), false);
            let second_digest = signature::sha256(&second.bytes);
            f.execute(second, Ok(1_000_000), false).await.map_err(debug)?;
            let third = f.signed(3, 3, &second_digest, false);
            let third_digest = signature::sha256(&third.bytes);
            f.execute(third, Ok(1_000_000), false).await.map_err(debug)?;
            let before = (f.counts().await?, head_row(&f.pool).await?);
            for (case, sequence, digest) in [
                ("rollback", 2, second_digest.clone()),
                ("stay", 3, third_digest.clone()),
            ] {
                let mut tx = connection.begin().await?;
                let moved = sqlx::query("UPDATE game_staff_control_heads SET control_seq=$1::text::numeric,link_sha256=$2 WHERE environment='alpha-test'")
                    .bind(sequence.to_string()).bind(&digest).execute(&mut *tx).await;
                tx.rollback().await?;
                match moved {
                    Ok(_) => failures.push(format!("{label}: head {case} accepted")),
                    Err(error) => {
                        let database = error.as_database_error().ok_or_else(|| debug(&error))?;
                        if database.code().as_deref() != Some("23514")
                            || database.message() != "staff head must advance to its exact successor"
                        {
                            failures.push(format!("{label}: head {case}: {error:?}"));
                        }
                    }
                }
            }
            if (f.counts().await?, head_row(&f.pool).await?) != before {
                return Err(format!("head rollback changed retained state: {label}").into());
            }

            let (mut previous, mut sequence) = (third_digest, 4);
            let mut pending = None;
            if next == Next::Receipt {
                // A lawful mutation whose receipt the operator signs later.
                let mutation = f.signed(4, 4, &previous, true);
                let digest = signature::sha256(&mutation.bytes);
                let stored = f.execute(mutation, Ok(1_000_000), true).await.map_err(debug)?;
                pending = Some((digest.clone(), stored));
                (previous, sequence) = (digest, 5);
            }

            // The LOGIN commits a structurally complete REQUEST link, operation
            // and outbox row whose signature is a donor over other bytes.
            let request = f.signer.request(40, sequence, &previous);
            let mut other = request.clone();
            other.issued_at = Decimal(1_000_001);
            let donor = f.signer.sign(other.canonical_bytes().map_err(debug)?, REQUEST_NAMESPACE);
            let fingerprint = f
                .trust
                .verify_request(donor.clone())
                .map_err(debug)?
                .fingerprint()
                .to_owned();
            let forged = signature::SignedBytes {
                bytes: request.canonical_bytes().map_err(debug)?,
                signature: donor.signature,
                principal: donor.principal,
            };
            if !matches!(f.trust.verify_request(forged.clone()), Err(Error::Signature)) {
                return Err(format!("forged request verified: {label}").into());
            }
            let forged_digest = signature::sha256(&forged.bytes);
            let uuid = || -> TestResult<String> {
                Ok(node::uuid_text(
                    &node::operator_files::uuid_v7().map_err(debug)?,
                ))
            };
            let (action_id, transaction_id) = (uuid()?, uuid()?);
            let mut tx = connection.begin().await?;
            sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',$1::text::numeric,$2,$3,$4,'REQUEST',$5,$6,$7,$8,txid_current())")
                .bind(sequence.to_string()).bind(&forged_digest).bind(&previous).bind(f.trust.roster_digest()).bind(&forged.bytes).bind(&forged.principal).bind(&fingerprint).bind(&forged.signature)
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_staff_operations(environment,operation_id,request_sha256,request_bytes,outcome_bytes,outcome_sha256,principal,key_fingerprint,action_id,transaction_id,control_seq,requires_receipt,occurred_at,created_xact_id) VALUES('alpha-test',$1::text::uuid,$2,$3,$4,encode(sha256($4),'hex'),$5,$6,$7::text::uuid,$8::text::uuid,$9::text::numeric,false,1000000,txid_current())")
                .bind(&request.operation_id).bind(&forged_digest).bind(&forged.bytes).bind(b"{}".to_vec()).bind(&forged.principal).bind(&fingerprint).bind(&action_id).bind(&transaction_id).bind(sequence.to_string())
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_staff_action_outbox(environment,operation_id,event_id,transaction_id,event_type_id,schema_revision,retention_profile_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,created_xact_id) VALUES('alpha-test',$1::text::uuid,$2::text::uuid,$3::text::uuid,5,1,'STAFF_ACTION_AUDIT_RETENTION_V1',1000000,1000000+31536000000,$4,encode(sha256($4),'hex'),1,txid_current())")
                .bind(&request.operation_id).bind(&action_id).bind(&transaction_id).bind(b"forged envelope".to_vec())
                .execute(&mut *tx).await?;
            let advanced = sqlx::query("UPDATE game_staff_control_heads SET control_seq=$1::text::numeric,link_sha256=$2 WHERE environment='alpha-test'")
                .bind(sequence.to_string()).bind(&forged_digest)
                .execute(&mut *tx).await?;
            if advanced.rows_affected() != 1 {
                return Err(format!("forged head advance missed: {label}").into());
            }
            tx.commit()
                .await
                .map_err(|error| format!("forged request COMMIT refused: {label}: {error:?}"))?;

            let before = (f.counts().await?, head_row(&f.pool).await?);
            if before.1.1 != forged_digest {
                return Err(format!("forged head was not committed: {label}").into());
            }
            let root = durability::DurabilityRoot::connect_test_runtime(&f.url)?;
            if !root.maintain_ready_once().await? {
                return Err(format!("fixture root not ready: {label}").into());
            }
            // With a pending mutation the fence refuses first; otherwise the
            // forged head itself must fail verification.
            let refusal = if next == Next::Receipt { Error::Receipt } else { Error::Signature };
            if let Some((mutation_digest, stored)) = &pending {
                let receipt = Receipt {
                    version: 1,
                    environment: f.trust.environment().into(),
                    roster_sha256: f.trust.roster_digest().into(),
                    operation_id: f.signer.request(4, 4, mutation_digest).operation_id,
                    request_sha256: mutation_digest.clone(),
                    outcome_sha256: signature::sha256(&stored.bytes),
                    disposition: Disposition::Committed,
                    issued_at: Decimal(1_000_000),
                    control_seq: Decimal(sequence + 1),
                    previous_link_sha256: forged_digest.clone(),
                };
                let bytes = oteryn_release_manifest::jcs::encode(&receipt, signature::MAX_REQUEST_BYTES)
                    .map_err(debug)?;
                let verified = f
                    .trust
                    .verify_receipt(f.signer.sign(bytes, RECEIPT_NAMESPACE))
                    .map_err(debug)?;
                let mut tx = f.pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
                let stored = owner::receipt_in_transaction(&mut tx, &f.trust, &verified).await;
                tx.rollback().await?;
                match stored {
                    Ok(()) => failures.push(format!("{label}: receipt extended the forged link")),
                    Err(StaffError::Rejected(Error::Signature)) => {}
                    Err(error) => failures.push(format!("{label}: receipt: {error:?}")),
                }
            }
            match root
                .staff_prepare_head(f.trust.environment().into(), Arc::clone(&f.history))
                .await
            {
                Ok(_) => failures.push(format!("{label}: preparation admitted")),
                Err(StaffError::Rejected(error)) if error == refusal => {}
                Err(error) => failures.push(format!("{label}: preparation: {error:?}")),
            }
            let following = f.signed(41, sequence + 1, &forged_digest, false);
            let request = f.trust.verify_request(following).map_err(debug)?;
            let mut probe = connection.begin().await?;
            let begun = owner::fixture_begin_in_transaction(
                &mut probe,
                &f.trust,
                &f.history,
                request,
                Ok(1_000_000),
            )
            .await;
            probe.rollback().await?;
            match begun {
                Ok(Begin::Fresh(_)) => failures.push(format!("{label}: next request extended the forged link")),
                Ok(Begin::Replay(_)) => return Err(format!("next request replayed: {label}").into()),
                Err(StaffError::Rejected(error)) if error == refusal => {}
                Err(error) => failures.push(format!("{label}: next request: {error:?}")),
            }
            if (f.counts().await?, head_row(&f.pool).await?) != before {
                return Err(format!("refusal changed retained state: {label}").into());
            }
            drop(root);
            connection.close().await?;
            Ok(failures)
        }
        .await;

        // These are the only database/cluster roles owned by this invocation.
        let database_cleanup = f.cleanup().await;
        let mut role_cleanup = Ok(());
        for role in created_roles.iter().rev() {
            let removed = sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {role}")))
                .execute(&mut admin)
                .await;
            if let Err(error) = removed {
                role_cleanup = Err(error);
            }
        }
        let admin_cleanup = admin.close().await;
        database_cleanup?;
        role_cleanup?;
        admin_cleanup?;
        result
    }

    runtime()?.block_on(async {
        let mut failures = Vec::new();
        for (label, group, next) in [
            ("runtime_forged_req", "oteryn_game_runtime", Next::Request),
            ("control_forged_req", "oteryn_game_control", Next::Request),
            ("runtime_forged_rcpt", "oteryn_game_runtime", Next::Receipt),
            ("control_forged_rcpt", "oteryn_game_control", Next::Receipt),
        ] {
            failures.extend(run_role(label, group, next).await?);
        }
        assert!(failures.is_empty(), "forged link failures: {failures:?}");
        Ok(())
    })
}

#[test]
fn staff_pg_member_logins_cannot_get_genuine_links_with_forged_rows_extended() -> TestResult {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Forgery {
        // A genuinely signed mutation whose operation row claims no receipt.
        UnfencedMutation,
        // A genuinely signed read whose audit event bytes are not its event.
        ForgedAudit,
    }

    async fn run_role(label: &str, group: &str, forgery: Forgery) -> TestResult<Vec<String>> {
        let Some(f) = Fixture::create(label).await? else {
            return Ok(Vec::new());
        };
        let mut admin = sqlx::PgConnection::connect(&f.admin).await?;
        let mut created_roles = Vec::new();
        let result: TestResult<Vec<String>> = async {
            let nonce = f.name.rsplit('_').next().ok_or("fixture name")?;
            let login = format!("oteryn_gm_2181_{label}_{nonce}");
            let password = signature::sha256(&node::operator_files::random_bytes::<32>()?);
            // Only generated ASCII fixture identifiers/passwords are interpolated.
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE ROLE {login} LOGIN INHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
            )))
            .execute(&mut admin)
            .await?;
            created_roles.push(login.clone());
            sqlx::query(sqlx::AssertSqlSafe(format!("GRANT {group} TO {login} WITH INHERIT TRUE")))
                .execute(&mut admin)
                .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "GRANT CONNECT ON DATABASE {} TO {login}",
                f.name
            )))
            .execute(&mut admin)
            .await?;
            let options = f
                .url
                .parse::<sqlx::postgres::PgConnectOptions>()?
                .username(&login)
                .password(&password);
            let mut connection = sqlx::PgConnection::connect_with(&options).await?;
            let identity: (String, bool) = sqlx::query_as(
                "SELECT current_user::text,pg_has_role(current_user,$1::name,'USAGE')",
            )
            .bind(group)
            .fetch_one(&mut connection)
            .await?;
            if identity != (login.clone(), true) {
                return Err(format!("invalid forged-row LOGIN fixture: {label}").into());
            }
            let mut failures = Vec::new();

            // The worker holds a genuinely signed request and commits it with
            // operation/outbox rows of its own choosing plus the head advance.
            let mutation = forgery == Forgery::UnfencedMutation;
            let signed = f.signed(50, 2, f.trust.activation_digest(), mutation);
            let verified = f.trust.verify_request(signed.clone()).map_err(debug)?;
            let uuid = || -> TestResult<String> {
                Ok(node::uuid_text(&node::operator_files::uuid_v7().map_err(debug)?))
            };
            let outcome = Outcome {
                disposition: Disposition::Committed,
                result: ResultCode::Ok,
                action_id: uuid()?,
                transaction_id: uuid()?,
                occurred_at: Decimal(1_000_000),
            };
            let outcome_bytes = outcome.canonical_bytes().map_err(debug)?;
            let envelope = match forgery {
                Forgery::UnfencedMutation => staff::audit::encode(
                    Contract::registered().map_err(debug)?,
                    &verified,
                    &outcome,
                    1_000_000,
                    "staff-test",
                )
                .map_err(debug)?,
                Forgery::ForgedAudit => b"forged envelope".to_vec(),
            };
            let mut tx = connection.begin().await?;
            sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',2,$1,$2,$3,'REQUEST',$4,$5,$6,$7,txid_current())")
                .bind(verified.digest()).bind(f.trust.activation_digest()).bind(f.trust.roster_digest()).bind(&signed.bytes).bind(&signed.principal).bind(verified.fingerprint()).bind(&signed.signature)
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_staff_operations(environment,operation_id,request_sha256,request_bytes,outcome_bytes,outcome_sha256,principal,key_fingerprint,action_id,transaction_id,control_seq,requires_receipt,occurred_at,created_xact_id) VALUES('alpha-test',$1::text::uuid,$2,$3,$4,encode(sha256($4),'hex'),$5,$6,$7::text::uuid,$8::text::uuid,2,false,1000000,txid_current())")
                .bind(&verified.request().operation_id).bind(verified.digest()).bind(&signed.bytes).bind(&outcome_bytes).bind(&signed.principal).bind(verified.fingerprint()).bind(&outcome.action_id).bind(&outcome.transaction_id)
                .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_staff_action_outbox(environment,operation_id,event_id,transaction_id,event_type_id,schema_revision,retention_profile_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,created_xact_id) VALUES('alpha-test',$1::text::uuid,$2::text::uuid,$3::text::uuid,5,1,'STAFF_ACTION_AUDIT_RETENTION_V1',1000000,1000000+31536000000,$4,encode(sha256($4),'hex'),1,txid_current())")
                .bind(&verified.request().operation_id).bind(&outcome.action_id).bind(&outcome.transaction_id).bind(&envelope)
                .execute(&mut *tx).await?;
            sqlx::query("UPDATE game_staff_control_heads SET control_seq=2,link_sha256=$1 WHERE environment='alpha-test'")
                .bind(verified.digest()).execute(&mut *tx).await?;
            tx.commit()
                .await
                .map_err(|error| format!("forged-row COMMIT refused: {label}: {error:?}"))?;

            let before = f.counts().await?;
            if before.4 != "2" {
                return Err(format!("forged rows were not committed: {label}").into());
            }
            let root = durability::DurabilityRoot::connect_test_runtime(&f.url)?;
            if !root.maintain_ready_once().await? {
                return Err(format!("fixture root not ready: {label}").into());
            }
            // The signed action needs a receipt: the false SQL flag is a
            // binding error. The garbage event bytes fail wire validation.
            let refusal = match forgery {
                Forgery::UnfencedMutation => Error::Binding,
                Forgery::ForgedAudit => Error::Invalid,
            };
            match root
                .staff_prepare_head(f.trust.environment().into(), Arc::clone(&f.history))
                .await
            {
                Ok(_) => failures.push(format!("{label}: preparation admitted")),
                Err(StaffError::Rejected(error)) if error == refusal => {}
                Err(error) => failures.push(format!("{label}: preparation: {error:?}")),
            }
            for (case, request) in [
                ("next request", f.signed(51, 3, verified.digest(), false)),
                ("replay", signed.clone()),
            ] {
                let request = f.trust.verify_request(request).map_err(debug)?;
                let mut probe = connection.begin().await?;
                let begun = owner::fixture_begin_in_transaction(
                    &mut probe,
                    &f.trust,
                    &f.history,
                    request,
                    Ok(1_000_000),
                )
                .await;
                probe.rollback().await?;
                // A replay re-checks only its own stored operation.
                match (case, begun) {
                    (_, Ok(Begin::Fresh(_))) => failures.push(format!("{label}: {case} extended the forged rows")),
                    ("replay", Ok(Begin::Replay(_))) if forgery == Forgery::ForgedAudit => {}
                    ("replay", Err(StaffError::Rejected(Error::Binding))) if mutation => {}
                    ("next request", Err(StaffError::Rejected(error))) if error == refusal => {}
                    (_, Ok(Begin::Replay(_))) => failures.push(format!("{label}: {case} returned the forged outcome")),
                    (_, Err(error)) => failures.push(format!("{label}: {case}: {error:?}")),
                }
            }
            if f.counts().await? != before {
                return Err(format!("refusal changed retained state: {label}").into());
            }
            drop(root);
            connection.close().await?;
            Ok(failures)
        }
        .await;

        // These are the only database/cluster roles owned by this invocation.
        let database_cleanup = f.cleanup().await;
        let mut role_cleanup = Ok(());
        for role in created_roles.iter().rev() {
            let removed = sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {role}")))
                .execute(&mut admin)
                .await;
            if let Err(error) = removed {
                role_cleanup = Err(error);
            }
        }
        let admin_cleanup = admin.close().await;
        database_cleanup?;
        role_cleanup?;
        admin_cleanup?;
        result
    }

    runtime()?.block_on(async {
        let mut failures = Vec::new();
        for (label, group, forgery) in [
            (
                "runtime_unfenced",
                "oteryn_game_runtime",
                Forgery::UnfencedMutation,
            ),
            (
                "control_unfenced",
                "oteryn_game_control",
                Forgery::UnfencedMutation,
            ),
            (
                "runtime_forged_audit",
                "oteryn_game_runtime",
                Forgery::ForgedAudit,
            ),
            (
                "control_forged_audit",
                "oteryn_game_control",
                Forgery::ForgedAudit,
            ),
        ] {
            failures.extend(run_role(label, group, forgery).await?);
        }
        assert!(failures.is_empty(), "forged row failures: {failures:?}");
        Ok(())
    })
}

#[test]
fn staff_pg_activation_waits_for_the_pending_mutation_receipt() -> TestResult {
    runtime()?.block_on(async {
        let Some(f) = Fixture::create("activation_receipt").await? else {
            return Ok(());
        };
        let result: TestResult = async {
            let mutation = f.signed(2, 2, f.trust.activation_digest(), true);
            let mutation_digest = signature::sha256(&mutation.bytes);
            let stored = f
                .execute(mutation, Ok(1_000_000), true)
                .await
                .map_err(debug)?;
            let before = f.counts().await?;
            // The owner may not rotate past a mutation still awaiting receipt.
            let replacement = Signer::new();
            let early = signed_activation(&replacement, "alpha-test", 3, &mutation_digest)?;
            match commit_activation(&f.pool, &early).await {
                Ok(()) => return Err("activation stranded the pending receipt".into()),
                Err(error) => {
                    let error = error
                        .downcast_ref::<sqlx::Error>()
                        .and_then(|e| e.as_database_error())
                        .map(|e| (e.code().map(|c| c.into_owned()), e.message().to_owned()));
                    if error
                        != Some((
                            Some("23514".into()),
                            "staff activation awaits pending receipt".into(),
                        ))
                    {
                        return Err(format!("unexpected activation refusal: {error:?}").into());
                    }
                }
            }
            if f.counts().await? != before {
                return Err("refused activation changed retained state".into());
            }
            // After the lawful receipt the same rotation, one link later, commits.
            let receipt = Receipt {
                version: 1,
                environment: f.trust.environment().into(),
                roster_sha256: f.trust.roster_digest().into(),
                operation_id: f
                    .signer
                    .request(2, 2, f.trust.activation_digest())
                    .operation_id,
                request_sha256: mutation_digest.clone(),
                outcome_sha256: signature::sha256(&stored.bytes),
                disposition: Disposition::Committed,
                issued_at: Decimal(1_000_000),
                control_seq: Decimal(3),
                previous_link_sha256: mutation_digest,
            };
            let verified = f
                .trust
                .verify_receipt(f.signer.sign(
                    oteryn_release_manifest::jcs::encode(&receipt, signature::MAX_REQUEST_BYTES)
                        .map_err(debug)?,
                    RECEIPT_NAMESPACE,
                ))
                .map_err(debug)?;
            let mut tx = f.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            owner::receipt_in_transaction(&mut tx, &f.trust, &verified)
                .await
                .map_err(debug)?;
            tx.commit().await?;
            let receipt_digest = signature::sha256(&verified.signed().bytes);
            let later = signed_activation(&replacement, "alpha-test", 4, &receipt_digest)?;
            commit_activation(&f.pool, &later).await?;
            let head: (String, String, String) = sqlx::query_as("SELECT control_seq::text,link_sha256,activation_sha256 FROM game_staff_control_heads WHERE environment='alpha-test'")
                .fetch_one(&f.pool).await?;
            if head != ("4".into(), later.digest().into(), later.digest().into()) {
                return Err(format!("lawful rotation head: {head:?}").into());
            }
            Ok(())
        }
        .await;
        f.cleanup().await?;
        result
    })
}

// GM-1 (D1437): the audited staff executor, served through the node's control
// dispatch entry (`serve_staff_control`, and its fixture-clock twin for
// requests) on a runtime LOGIN, with the live desk that the node's served
// connections and chat read. Every mutation is followed by its signed receipt,
// as the operator tool does, so the chain keeps accepting new work.
mod executor {
    use super::*;
    use crate::durability::staff_moderation::{
        ItemTextReport, MAX_MUTE_MS, OPEN_SNAPSHOTS_PER_WORLD, REPORT_RETENTION_MS,
        ReportAdmission, StaffNode,
    };
    use crate::staff::live::{self, KickResult, StaffDesk};
    use crate::staff::signature::{ControlMessage, ReportState, Target};

    const NOW: u64 = 1_000_000;
    const TARGET: &str = "00000000-0000-7000-8000-000000000001";
    const REPORTER: &str = "00000000-0000-7000-8000-000000000002";
    const FOREIGN: &str = "00000000-0000-7000-8000-000000000003";
    const ACCOUNT: &str = "00000000-0000-7000-8000-0000000000a1";
    const REPORTER_ACCOUNT: &str = "00000000-0000-7000-8000-0000000000a2";
    const WORLD: &str = "00000000-0000-7000-8000-000000000010";
    const CHANNEL: &str = "00000000-0000-7000-8000-000000000011";
    const HOUR: u64 = 3_600_000;
    /// Report time: a whole hour, far after the staff fixture clock.
    const REPORTED: u64 = HOUR * 500_000;

    // Three Characters (two in this node's World, one in another) and 24 items
    // in this World, each carrying text revision 1. Loaded as the owner with
    // the replica role, as fixtures of other aggregates are.
    const SEED: &str = "\
INSERT INTO game_character_account_guards(account_id) VALUES
 ('00000000-0000-7000-8000-0000000000a1'),('00000000-0000-7000-8000-0000000000a2'),
 ('00000000-0000-7000-8000-0000000000a3');
INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,
 profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES
 ('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-0000000000a1',
  '00000000-0000-7000-8000-000000000010',1,1,'profile-1','ruleset-1','content-1','starter-1','Alda'),
 ('00000000-0000-7000-8000-000000000002','00000000-0000-7000-8000-0000000000a2',
  '00000000-0000-7000-8000-000000000010',1,1,'profile-1','ruleset-1','content-1','starter-1','Brin'),
 ('00000000-0000-7000-8000-000000000003','00000000-0000-7000-8000-0000000000a3',
  '00000000-0000-7000-8000-000000000020',1,1,'profile-1','ruleset-1','content-1','starter-1','Cato');
INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
 definition_revision_ref,quantity,lifecycle,minted_transaction_id)
 SELECT format('00000000-0000-7000-8000-%s',lpad(to_hex(256+n),12,'0'))::uuid,
  '00000000-0000-7000-8000-000000000010','book','oteryn:item.book','revision-1',1,1,
  format('00000000-0000-7000-8000-%s',lpad(to_hex(512+n),12,'0'))::uuid
 FROM generate_series(0,23) AS n;
INSERT INTO game_item_text_revisions(item_instance_id,last_revision)
 SELECT item_instance_id,1 FROM game_item_instances;
INSERT INTO game_item_texts(item_instance_id,text,writer_character_id,written_at,text_revision)
 SELECT item_instance_id,'Reported text '||item_instance_id::text,
  '00000000-0000-7000-8000-000000000001','2026-01-01T00:00:00Z',1 FROM game_item_instances;";

    fn id(text: &str) -> [u8; 16] {
        node::uuid_bytes(text).expect("fixture uuid")
    }
    fn item(n: u16) -> String {
        format!("00000000-0000-7000-8000-{:012x}", 0x100 + n)
    }
    fn here() -> StaffNode {
        StaffNode {
            world: id(WORLD),
            channel: id(CHANNEL),
        }
    }

    struct Answer {
        outcome: Outcome,
        data: Option<serde_json::Value>,
    }

    struct Exec {
        f: Fixture,
        root: durability::DurabilityRoot,
        login: String,
        desk: Arc<StaffDesk>,
        sequence: u64,
        head: String,
        operation: u8,
    }
    impl Exec {
        async fn create(tag: &str, role: Option<&str>) -> TestResult<Option<Self>> {
            let Some(f) = Fixture::create_as(tag, role).await? else {
                return Ok(None);
            };
            let nonce = f.name.rsplit('_').next().ok_or("fixture name")?.to_owned();
            let login = format!("gm_rt_{nonce}");
            let made: TestResult<durability::DurabilityRoot> = async {
                let mut tx = f.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::raw_sql(SEED).execute(&mut *tx).await?;
                tx.commit().await?;
                let password = signature::sha256(&node::operator_files::random_bytes::<32>()?);
                // Generated ASCII identifiers only; nothing comes from the URL.
                for statement in [
                    format!(
                        "CREATE ROLE {login} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"
                    ),
                    format!("GRANT CONNECT ON DATABASE {} TO {login}", f.name),
                ] {
                    sqlx::query(sqlx::AssertSqlSafe(statement))
                        .execute(&f.pool)
                        .await?;
                }
                let (_, address) = f.url.split_once('@').ok_or("fixture URL")?;
                let root = durability::DurabilityRoot::connect_test_runtime(&format!(
                    "postgresql://{login}:{password}@{address}"
                ))?;
                if !root.maintain_ready_once().await? {
                    return Err("runtime staff root did not become ready".into());
                }
                Ok(root)
            }
            .await;
            match made {
                Ok(root) => Ok(Some(Self {
                    head: f.trust.activation_digest().to_owned(),
                    f,
                    root,
                    login,
                    desk: StaffDesk::new(),
                    sequence: 1,
                    operation: 1,
                })),
                Err(error) => {
                    let admin = f.admin.clone();
                    let _ = f.cleanup().await;
                    drop_login(&admin, &login).await?;
                    Err(error)
                }
            }
        }
        async fn cleanup(self) -> TestResult {
            let Self { f, root, login, .. } = self;
            drop(root);
            let admin = f.admin.clone();
            let database = f.cleanup().await;
            drop_login(&admin, &login).await?;
            database
        }
        async fn send(&mut self, action: Action) -> Result<Answer, StaffError> {
            self.send_to(action, here()).await
        }
        async fn send_to(&mut self, action: Action, node: StaffNode) -> Result<Answer, StaffError> {
            self.operation += 1;
            let mut request = self
                .f
                .signer
                .request(self.operation, self.sequence + 1, &self.head);
            let mutation = !action.kind().is_read();
            request.action = action;
            let operation_id = request.operation_id.clone();
            let signed = self
                .f
                .signer
                .sign(request.canonical_bytes()?, REQUEST_NAMESPACE);
            let digest = signature::sha256(&signed.bytes);
            let answer = self
                .root
                .fixture_serve_staff_request(
                    Arc::clone(&self.f.trust),
                    Arc::clone(&self.f.history),
                    Arc::clone(&self.desk),
                    node,
                    signed,
                    NOW,
                )
                .await?;
            self.sequence += 1;
            self.head = digest.clone();
            let (stored, data) = match answer.iter().position(|byte| *byte == b'\n') {
                Some(at) => (&answer[..at], Some(&answer[at + 1..])),
                None => (answer.as_slice(), None),
            };
            let outcome: Outcome = serde_json::from_slice(stored).map_err(|_| Error::Invalid)?;
            let data = data
                .map(serde_json::from_slice::<serde_json::Value>)
                .transpose()
                .map_err(|_| Error::Invalid)?;
            if mutation {
                self.receipt(operation_id, digest, stored, outcome.disposition)
                    .await?;
            }
            Ok(Answer { outcome, data })
        }
        /// The operator's signed receipt of one mutation, through the same
        /// control dispatch entry the node's control socket uses.
        async fn receipt(
            &mut self,
            operation_id: String,
            request: String,
            stored: &[u8],
            disposition: Disposition,
        ) -> Result<(), StaffError> {
            let receipt = Receipt {
                version: 1,
                environment: self.f.trust.environment().into(),
                roster_sha256: self.f.trust.roster_digest().into(),
                operation_id,
                request_sha256: request.clone(),
                outcome_sha256: signature::sha256(stored),
                disposition,
                issued_at: Decimal(NOW),
                control_seq: Decimal(self.sequence + 1),
                previous_link_sha256: request,
            };
            let bytes =
                oteryn_release_manifest::jcs::encode(&receipt, signature::MAX_REQUEST_BYTES)
                    .map_err(|_| Error::Invalid)?;
            let signed = self.f.signer.sign(bytes, RECEIPT_NAMESPACE);
            let digest = signature::sha256(&signed.bytes);
            let answer = self
                .root
                .serve_staff_control(
                    Arc::clone(&self.f.trust),
                    Arc::clone(&self.f.history),
                    Arc::clone(&self.desk),
                    here(),
                    ControlMessage::Receipt(signed),
                )
                .await?;
            if answer != b"receipt-stored" {
                return Err(Error::Receipt.into());
            }
            self.sequence += 1;
            self.head = digest;
            Ok(())
        }
        async fn scalar(&self, sql: &'static str) -> TestResult<i64> {
            Ok(sqlx::query_scalar(sql).fetch_one(&self.f.pool).await?)
        }
    }

    async fn drop_login(admin: &str, login: &str) -> TestResult {
        let mut connection = sqlx::PgConnection::connect(admin).await?;
        connection
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP ROLE IF EXISTS {login}"
            ))))
            .await?;
        connection.close().await?;
        Ok(())
    }

    /// Runs `case` on a fresh executor fixture; the database and the runtime
    /// LOGIN are dropped whether or not the case passes.
    fn exec_case(
        tag: &'static str,
        role: Option<&'static str>,
        case: impl AsyncFnOnce(&mut Exec) -> TestResult,
    ) -> TestResult {
        runtime()?.block_on(async move {
            let Some(mut exec) = Exec::create(tag, role).await? else {
                return Ok(());
            };
            let result = case(&mut exec).await;
            let cleanup = exec.cleanup().await;
            result?;
            cleanup
        })
    }

    fn mute(character: &str, duration_ms: u64) -> Action {
        Action::Mute {
            character_id: character.into(),
            duration_ms: Decimal(duration_ms),
        }
    }
    fn inspect(character: &str) -> Action {
        Action::Inspect {
            character_id: character.into(),
        }
    }
    fn kick(target: Target) -> Action {
        Action::Kick { target }
    }
    fn of(answer: &Answer) -> (Disposition, ResultCode) {
        (answer.outcome.disposition, answer.outcome.result)
    }
    const COMMITTED: (Disposition, ResultCode) = (Disposition::Committed, ResultCode::Ok);

    #[test]
    fn staff_pg_executor_mute_inspect_unmute_and_kick_follow_the_committed_audit() -> TestResult {
        exec_case("exec", Some("GM"), async |e| {
            let target = id(TARGET);
            let muted = e.send(mute(TARGET, 60_000)).await.map_err(debug)?;
            assert_eq!(of(&muted), COMMITTED);
            let data = muted.data.ok_or("mute data")?;
            let sanction = data["sanction_id"]
                .as_str()
                .ok_or("sanction id")?
                .to_owned();
            assert_eq!(data["ends_at"], "1060000");
            // The committed mute reaches the desk chat admission reads.
            assert_eq!(e.desk.mute_seconds(&target, NOW), Some(60));
            let row: (String, String, i64, String) = sqlx::query_as(
                "SELECT status,actor_principal,ends_at,mute_action_id::text \
                 FROM game_character_sanctions WHERE sanction_id=$1::uuid",
            )
            .bind(&sanction)
            .fetch_one(&e.f.pool)
            .await?;
            assert_eq!(
                row,
                (
                    "ACTIVE".into(),
                    "operator".into(),
                    1_060_000,
                    muted.outcome.action_id.clone()
                )
            );
            // A restarted node reads the durable sanction back.
            assert_eq!(
                e.root
                    .staff_active_mute_until(target, NOW)
                    .await
                    .map_err(debug)?,
                Some(1_060_000)
            );

            let seen = e.send(inspect(TARGET)).await.map_err(debug)?;
            assert_eq!(of(&seen), COMMITTED);
            let data = seen.data.ok_or("inspect data")?;
            assert_eq!(data["name"], "Alda");
            assert_eq!(data["online"], false);
            assert_eq!(data["active_mutes"][0]["sanction_id"], sanction.as_str());
            assert_eq!(data["open_reports_filed"], "0");

            let lifted = e
                .send(Action::Unmute {
                    sanction_id: sanction.clone(),
                })
                .await
                .map_err(debug)?;
            assert_eq!(of(&lifted), COMMITTED);
            assert_eq!(e.desk.mute_seconds(&target, NOW), None);
            let again = e
                .send(Action::Unmute {
                    sanction_id: sanction.clone(),
                })
                .await
                .map_err(debug)?;
            assert_eq!(
                of(&again),
                (Disposition::Refused, ResultCode::AlreadyReleased)
            );
            let row: (String, i16, String) = sqlx::query_as(
                "SELECT status,revision,lift_action_id::text FROM game_character_sanctions \
                 WHERE sanction_id=$1::uuid",
            )
            .bind(&sanction)
            .fetch_one(&e.f.pool)
            .await?;
            assert_eq!(row, ("LIFTED".into(), 2, lifted.outcome.action_id));

            // No connection serves the target: refused, audited, no data.
            let by_character = || {
                kick(Target::Character {
                    character_id: TARGET.into(),
                })
            };
            let missing = e.send(by_character()).await.map_err(debug)?;
            assert_eq!(
                of(&missing),
                (Disposition::Refused, ResultCode::TargetNotInChannel)
            );
            assert!(missing.data.is_none());
            // A served connection runs its fenced release and answers.
            let mut guard = e.desk.register([0x51; 16], id(ACCOUNT), Some(target));
            let serving = tokio::spawn(async move {
                guard.next_kick().await.answer(KickResult::Released);
                guard
            });
            let kicked = e.send(by_character()).await.map_err(debug)?;
            assert_eq!(of(&kicked), COMMITTED);
            assert_eq!(kicked.data, Some(serde_json::json!({"kick": "RELEASED"})));
            let mut guard = serving.await?;
            // By account reference; the logout rule keeps an in-fight session.
            let serving = tokio::spawn(async move {
                guard.next_kick().await.answer(KickResult::InFight);
                guard
            });
            let kicked = e
                .send(kick(Target::Account {
                    account_ref: live::account_ref("alpha-test", &id(ACCOUNT)),
                }))
                .await
                .map_err(debug)?;
            assert_eq!(of(&kicked), COMMITTED);
            assert_eq!(kicked.data, Some(serde_json::json!({"kick": "IN_FIGHT"})));
            drop(serving.await?);

            assert_eq!(
                e.scalar("SELECT count(*) FROM game_staff_operations")
                    .await?,
                7
            );
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_staff_action_outbox")
                    .await?,
                7
            );
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_staff_action_receipts")
                    .await?,
                6
            );
            Ok(())
        })
    }

    #[test]
    fn staff_pg_executor_refuses_scope_bounds_tier_and_foreign_targets() -> TestResult {
        exec_case("refuse", Some("GM"), async |e| {
            let before = e.f.counts().await?;
            // The node scope is part of the signed request: another channel or
            // World is refused before anything is written.
            for node in [
                StaffNode {
                    world: id(WORLD),
                    channel: id("00000000-0000-7000-8000-000000000012"),
                },
                StaffNode {
                    world: id("00000000-0000-7000-8000-000000000020"),
                    channel: id(CHANNEL),
                },
            ] {
                let refused = e.send_to(inspect(TARGET), node).await;
                assert!(
                    matches!(refused, Err(StaffError::Rejected(Error::Binding))),
                    "{:?}",
                    refused.err()
                );
            }
            let over = e.send(mute(TARGET, MAX_MUTE_MS + 1)).await;
            assert!(
                matches!(over, Err(StaffError::Rejected(Error::Invalid))),
                "{:?}",
                over.err()
            );
            // A roster activation never runs through the executor.
            let roster = e.f.trust.roster_digest().to_owned();
            let activation = e
                .send(Action::RosterActivate {
                    roster_sha256: roster,
                })
                .await;
            assert!(
                matches!(
                    activation,
                    Err(StaffError::UnsupportedAction | StaffError::Rejected(_))
                ),
                "{:?}",
                activation.err()
            );
            assert_eq!(e.f.counts().await?, before);

            // A GM may not clear report text: refused and audited, no effect.
            let tier = e
                .send(Action::ReportClearText {
                    report_id: "00000000-0000-7000-8000-000000000040".into(),
                    text_revision: Decimal(1),
                })
                .await
                .map_err(debug)?;
            assert_eq!(
                of(&tier),
                (Disposition::Refused, ResultCode::TierInsufficient)
            );
            assert!(tier.data.is_none());
            // A Character of another World is not this node's target.
            let foreign = e.send(mute(FOREIGN, 60_000)).await.map_err(debug)?;
            assert_eq!(
                of(&foreign),
                (Disposition::Refused, ResultCode::TargetNotFound)
            );
            assert_eq!(e.desk.mute_seconds(&id(FOREIGN), NOW), None);
            let foreign = e.send(inspect(FOREIGN)).await.map_err(debug)?;
            assert_eq!(
                of(&foreign),
                (Disposition::Refused, ResultCode::TargetNotFound)
            );
            assert!(foreign.data.is_none());
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_character_sanctions")
                    .await?,
                0
            );
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_staff_operations")
                    .await?,
                3
            );
            Ok(())
        })
    }

    #[test]
    fn staff_pg_executor_without_a_roster_role_fails_closed_before_any_write() -> TestResult {
        exec_case("norole", None, async |e| {
            // The node refuses to boot on this roster (`roles()`), and a loaded
            // one still refuses every request before its first write.
            assert!(matches!(e.f.trust.roles(), Err(Error::Binding)));
            let before = e.f.counts().await?;
            for action in [inspect(TARGET), mute(TARGET, 60_000)] {
                let refused = e.send(action).await;
                assert!(
                    matches!(refused, Err(StaffError::Rejected(Error::Binding))),
                    "{:?}",
                    refused.err()
                );
            }
            assert_eq!(e.f.counts().await?, before);
            assert_eq!(e.desk.mute_seconds(&id(TARGET), NOW), None);
            Ok(())
        })
    }

    /// Boundary rows 16-17 at the executor: an audit write failure rolls back
    /// the action's own writes, returns no data and applies no live effect.
    #[test]
    fn staff_pg_executor_audit_failure_rolls_back_the_action_and_its_effect() -> TestResult {
        exec_case("auditfail", Some("GM"), async |e| {
            sqlx::raw_sql("CREATE FUNCTION staff_fixture_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture audit failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER staff_fixture_fail BEFORE INSERT ON game_staff_action_outbox FOR EACH ROW EXECUTE FUNCTION staff_fixture_fail()").execute(&e.f.pool).await?;
            let before = e.f.counts().await?;
            let mut guard = e.desk.register([0x52; 16], id(ACCOUNT), Some(id(TARGET)));
            for action in [
                mute(TARGET, 60_000),
                inspect(TARGET),
                kick(Target::Character {
                    character_id: TARGET.into(),
                }),
            ] {
                let failed = e.send(action).await;
                assert!(
                    matches!(failed, Err(StaffError::Database(_))),
                    "{:?}",
                    failed.err()
                );
            }
            assert_eq!(e.f.counts().await?, before);
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_character_sanctions")
                    .await?,
                0
            );
            assert_eq!(e.desk.mute_seconds(&id(TARGET), NOW), None);
            // No kick reached the served connection.
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(50), guard.next_kick())
                    .await
                    .is_err()
            );
            sqlx::query("DROP TRIGGER staff_fixture_fail ON game_staff_action_outbox")
                .execute(&e.f.pool)
                .await?;
            let seen = e.send(inspect(TARGET)).await.map_err(debug)?;
            assert_eq!(of(&seen), COMMITTED);
            drop(guard);
            Ok(())
        })
    }

    /// REPORT-3 / WRITE0 §6 on the runtime LOGIN: admission, dedup, STALE,
    /// WRITE0-RL-04..06, the audited close and clear, and RL-03 expiry.
    #[test]
    fn staff_pg_item_text_reports_admit_close_clear_and_expire_under_write0_limits() -> TestResult {
        exec_case("reports", Some("ADMIN"), async |e| {
            let report = |reporter: &str, n: u16, text_revision: u64| ItemTextReport {
                world: id(WORLD),
                reporter_character: id(reporter),
                item_instance: id(&item(n)),
                text_revision,
            };
            let root = e.root.clone();
            let pool = e.f.pool.clone();
            let admit = |reporter: &'static str, n: u16, revision: u64, at: u64, capacity: i32| {
                let root = &root;
                async move {
                    root.create_item_text_report(report(reporter, n, revision), at, capacity)
                        .await
                        .map_err(debug)
                }
            };
            let cap = OPEN_SNAPSHOTS_PER_WORLD;
            let ReportAdmission::Open {
                report_id: first,
                first: true,
                alarm: false,
            } = admit(REPORTER, 0, 1, REPORTED, cap).await?
            else {
                return Err("first report not admitted".into());
            };
            // Dedup returns the first OPEN report; a revision that is not the
            // item's current text is STALE and stores nothing.
            assert_eq!(
                admit(REPORTER, 0, 1, REPORTED, cap).await?,
                ReportAdmission::Open {
                    report_id: first.clone(),
                    first: false,
                    alarm: false
                }
            );
            assert_eq!(
                admit(REPORTER, 0, 2, REPORTED, cap).await?,
                ReportAdmission::Stale
            );
            let mut reports = vec![first.clone()];
            for n in 1..10 {
                match admit(REPORTER, n, 1, REPORTED, cap).await? {
                    ReportAdmission::Open {
                        report_id,
                        first: true,
                        ..
                    } => reports.push(report_id),
                    other => return Err(format!("report {n}: {other:?}").into()),
                }
            }
            // WRITE0-RL-04: ten per reporter Character per hour.
            assert_eq!(
                admit(REPORTER, 10, 1, REPORTED, cap).await?,
                ReportAdmission::RateLimited
            );
            for n in 10..20 {
                match admit(REPORTER, n, 1, REPORTED + HOUR, cap).await? {
                    ReportAdmission::Open {
                        report_id,
                        first: true,
                        ..
                    } => reports.push(report_id),
                    other => return Err(format!("report {n}: {other:?}").into()),
                }
            }
            // WRITE0-RL-05: twenty OPEN per reporter account.
            assert_eq!(
                admit(REPORTER, 20, 1, REPORTED + 2 * HOUR, cap).await?,
                ReportAdmission::Quota
            );
            // WRITE0-RL-06: the World's open snapshot capacity.
            assert_eq!(
                admit(TARGET, 20, 1, REPORTED + 2 * HOUR, 20).await?,
                ReportAdmission::Capacity
            );
            // A reporter outside this World cannot report its items.
            assert!(
                e.root
                    .create_item_text_report(report(FOREIGN, 21, 1), REPORTED, cap)
                    .await
                    .is_err()
            );
            let counters = || async {
                sqlx::query_as::<_, (i16, i32)>(
                    "SELECT (SELECT open_reports FROM game_item_text_report_accounts \
                     WHERE account_id=$1::uuid),(SELECT open_snapshots FROM \
                     game_item_text_report_worlds WHERE world_id=$2::uuid)",
                )
                .bind(REPORTER_ACCOUNT)
                .bind(WORLD)
                .fetch_one(&pool)
                .await
            };
            assert_eq!(counters().await?, (20, 20));
            let snapshot: String = sqlx::query_scalar(
                "SELECT snapshot_text FROM game_item_text_reports WHERE report_id=$1::uuid",
            )
            .bind(&first)
            .fetch_one(&e.f.pool)
            .await?;
            assert_eq!(snapshot, format!("Reported text {}", item(0)));

            let listed = e
                .send(Action::ReportsList {
                    state: ReportState::Open,
                })
                .await
                .map_err(debug)?;
            assert_eq!(of(&listed), COMMITTED);
            let data = listed.data.ok_or("list data")?;
            assert_eq!(data["reports"].as_array().map(Vec::len), Some(20));
            assert_eq!(data["truncated"], false);
            assert_eq!(data["reports"][0]["report_id"], first.as_str());
            assert_eq!(data["reports"][0]["text"], snapshot.as_str());

            // Close: one conditional transition; the counters fall once.
            let close = || Action::ReportClose {
                report_id: first.clone(),
            };
            let closed = e.send(close()).await.map_err(debug)?;
            assert_eq!(of(&closed), COMMITTED);
            assert_eq!(counters().await?, (19, 19));
            let row: (String, String) = sqlx::query_as(
                "SELECT state,closed_action_id::text FROM game_item_text_reports \
                 WHERE report_id=$1::uuid",
            )
            .bind(&first)
            .fetch_one(&e.f.pool)
            .await?;
            assert_eq!(row, ("CLOSED".into(), closed.outcome.action_id));
            let stale = e.send(close()).await.map_err(debug)?;
            assert_eq!(of(&stale), (Disposition::Refused, ResultCode::Stale));
            assert_eq!(counters().await?, (19, 19));

            // Clear: the live text goes and the item's revision advances, only
            // while the reported revision is still current.
            let clear = |report: &str, revision: u64| Action::ReportClearText {
                report_id: report.into(),
                text_revision: Decimal(revision),
            };
            let cleared = e.send(clear(reports[1].as_str(), 1)).await.map_err(debug)?;
            assert_eq!(of(&cleared), COMMITTED);
            let item_row: (i64, String, String) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_item_texts WHERE item_instance_id=$1::uuid),\
                 (SELECT last_revision::text FROM game_item_text_revisions WHERE item_instance_id=$1::uuid),\
                 (SELECT cleared_action_id::text FROM game_item_text_reports WHERE report_id=$2::uuid)",
            )
            .bind(item(1))
            .bind(&reports[1])
            .fetch_one(&e.f.pool)
            .await?;
            assert_eq!(item_row, (0, "2".into(), cleared.outcome.action_id));
            for (report, revision) in [(reports[1].as_str(), 1), (reports[2].as_str(), 2)] {
                let stale = e.send(clear(report, revision)).await.map_err(debug)?;
                assert_eq!(of(&stale), (Disposition::Refused, ResultCode::Stale));
            }
            assert_eq!(
                e.scalar(
                    "SELECT count(*) FROM game_item_texts WHERE item_instance_id=\
                     '00000000-0000-7000-8000-000000000102'"
                )
                .await?,
                1
            );
            let seen = e.send(inspect(REPORTER)).await.map_err(debug)?;
            assert_eq!(seen.data.ok_or("inspect")?["open_reports_filed"], "19");

            // WRITE0-RL-03: every OPEN report past retention expires, ended
            // snapshots are deleted and finished hours' counters removed.
            let expired = e
                .root
                .expire_item_text_reports(REPORTED + 2 * HOUR + u64::try_from(REPORT_RETENTION_MS)?)
                .await
                .map_err(debug)?;
            assert_eq!(expired, 19);
            assert_eq!(counters().await?, (0, 0));
            assert_eq!(
                e.scalar(
                    "SELECT count(*) FROM game_item_text_reports WHERE snapshot_text IS NOT NULL"
                )
                .await?,
                0
            );
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_item_text_reports WHERE state='EXPIRED'")
                    .await?,
                19
            );
            assert_eq!(
                e.scalar("SELECT count(*) FROM game_item_text_report_hourly")
                    .await?,
                0
            );
            Ok(())
        })
    }
}
