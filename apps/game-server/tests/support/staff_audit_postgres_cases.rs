#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// D1343 STAFF_ACTION PostgreSQL cases shared by the focused wrapper and the
// canonical durability aggregate. The crate root supplies one durability type
// universe and the public production staff module.

use crate::durability::staff_action::{self as owner, Begin, StaffError, StoredOutcome};
use crate::staff::{
    audit::{Contract, Reason, ResultCode},
    signature::{
        self, Action, Decimal, Disposition, Error, Outcome, RECEIPT_NAMESPACE, REQUEST_NAMESPACE,
        Receipt, VerifierTrust,
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
        signer.roster =
            format!("operator namespaces=\"{REQUEST_NAMESPACE},{RECEIPT_NAMESPACE}\" {key}\n")
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
}
impl Fixture {
    async fn create(tag: &str) -> TestResult<Option<Self>> {
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
        let signer = Signer::new();
        let (trust, activation) = signer.activation();
        let checked = trust.verify_request(activation.clone()).map_err(debug)?;
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,link_sha256,previous_link_sha256,roster_sha256,link_kind,signed_bytes,principal,key_fingerprint,signature,created_xact_id) VALUES('alpha-test',1,$1,$2,$3,'ACTIVATION',$4,$5,$6,$7,txid_current())")
            .bind(checked.digest()).bind("0".repeat(64)).bind(trust.roster_digest()).bind(&activation.bytes).bind(&activation.principal).bind(checked.fingerprint()).bind(&activation.signature).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_staff_control_heads(environment,control_seq,link_sha256,activation_sha256) VALUES('alpha-test',1,$1,$1)").bind(trust.activation_digest()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some(Self {
            admin,
            name,
            url,
            pool,
            signer,
            trust: Arc::new(trust),
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
        let request = self.trust.verify_request(signed)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        match owner::fixture_begin_in_transaction(&mut tx, &self.trust, request, now).await? {
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
        let updated = sqlx::query("UPDATE game_staff_control_heads SET control_seq=2,link_sha256=$1 WHERE environment='alpha-test'")
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
            let result = owner::fixture_begin_with_clock(&mut tx, &trust, request, move || {
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
