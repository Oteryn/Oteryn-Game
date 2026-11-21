#![allow(clippy::expect_used)]
#![allow(
    unexpected_cfgs,
    reason = "game-server uses the same fixture and always includes its tests"
)]
// CHAR-REV-SEQ-1 cases: revision-advancing writers run one at a time per Character through the
// channel runtime's revision sequencer (QUEST-STATE-0 §5.2). Any wrapper that path-loads the
// crate root of `durability_postgres.rs` can include this file. The harness follows the CHARM-3
// cases (one live session, lease, scope assignment and readiness for Character 41).

#[path = "character_revision_node_registration.rs"]
mod node_registration;
pub(super) use node_registration::registered_node;

use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
use crate::domain::{CharacterId, CharacterRevision};
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmStateError,
};
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Executor};
use std::collections::BTreeMap;
use std::future::Future;
use std::task::Poll;
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) async fn join_two<A, B>(first: A, second: B) -> (A::Output, B::Output)
where
    A: Future,
    B: Future,
{
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut first_output = None;
    let mut second_output = None;
    std::future::poll_fn(move |context| {
        if first_output.is_none()
            && let Poll::Ready(output) = first.as_mut().poll(context)
        {
            first_output = Some(output);
        }
        if second_output.is_none()
            && let Poll::Ready(output) = second.as_mut().poll(context)
        {
            second_output = Some(output);
        }
        match (first_output.take(), second_output.take()) {
            (Some(first), Some(second)) => Poll::Ready((first, second)),
            (first, second) => {
                first_output = first;
                second_output = second;
                Poll::Pending
            }
        }
    })
    .await
}

pub(super) fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

pub(super) fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String, name: &str) -> TestResult<Self> {
        Self::create_from(admin_url, name, None).await
    }

    async fn create_from(
        admin_url: String,
        name: &str,
        template: Option<&str>,
    ) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.rsplit_once('@').is_some_and(|(_, host)| {
                host.strip_prefix("127.0.0.1:")
                    .and_then(|rest| rest.strip_suffix("/postgres"))
                    .is_some_and(|port| {
                        port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u16>().is_ok()
                    })
            })
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("crs_{name}_{suffix}");
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut admin)
            .await?;
        assert_eq!(
            version,
            oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM,
            "canonical target is PostgreSQL 18.6"
        );
        let template = match template {
            Some(template) => template.to_owned(),
            // No tokio timer here: several suites run with paused tokio time, where a timer
            // would fire while this waits on the database. The job timeout bounds it.
            None => Self::migrated_template(&mut admin, &admin_url).await?,
        };
        Self::clone_database(&mut admin, &name, &template).await?;
        // Clones share the template's random decision roots; tests must not depend on them differing between databases.
        // CREATE DATABASE copies schema ACLs, but not the database's own
        // ACL. Preserve migration 0006's cross-database isolation boundary.
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "REVOKE CONNECT ON DATABASE {name} FROM PUBLIC"
            ))))
            .await?;
        admin.close().await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{name}");
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        // The clone already has every migration; retain checksum/ledger validation.
        oteryn_gameplay_durability::GAME_MIGRATOR
            .run(&mut connection)
            .await?;
        Self::refresh_migration_seeds(&mut connection).await?;
        let public_connect: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_database d, \
             LATERAL aclexplode(COALESCE(d.datacl,acldefault('d',d.datdba))) a \
             WHERE d.datname=current_database() AND a.grantee=0 AND a.privilege_type='CONNECT')",
        )
        .fetch_one(&mut connection)
        .await?;
        assert!(
            !public_connect,
            "fixture databases deny cross-database PUBLIC access"
        );
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn refresh_migration_seeds(connection: &mut PgConnection) -> TestResult {
        // Migration 2833 seeds this at migration time; give each clone a fresh resync start.
        sqlx::query(
            "UPDATE game_character_service_feed \
             SET resync_started_at_ms = floor(extract(epoch FROM transaction_timestamp())*1000)::bigint \
             WHERE singleton AND phase = 1",
        )
        .execute(connection)
        .await?;
        Ok(())
    }

    async fn migrated_template(admin: &mut PgConnection, admin_url: &str) -> TestResult<String> {
        let mut hash = Sha256::new();
        for migration in oteryn_gameplay_durability::GAME_MIGRATOR.iter() {
            hash.update(migration.version.to_be_bytes());
            hash.update(migration.checksum.as_ref());
        }
        let digest = hash.finalize();
        let template = format!(
            "oteryn_tpl_{:016x}",
            u64::from_be_bytes(digest[..8].try_into()?)
        );
        // nextest uses separate processes. This session lock serializes publication
        // across all of them; closing the admin connection also releases it on error.
        sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
            .bind(&template)
            .execute(&mut *admin)
            .await?;
        let is_template: Option<bool> =
            sqlx::query_scalar("SELECT datistemplate FROM pg_database WHERE datname = $1")
                .bind(&template)
                .fetch_optional(&mut *admin)
                .await?;
        if is_template.is_none() {
            let building = format!("{template}_building");
            // Recover an interrupted build without ever publishing a partial schema.
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE IF EXISTS {building} WITH (FORCE)"
                ))))
                .await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {building}"
                ))))
                .await?;
            let prefix = admin_url
                .strip_suffix("/postgres")
                .ok_or("invalid admin URL")?;
            let mut connection = PgConnection::connect(&format!("{prefix}/{building}")).await?;
            oteryn_gameplay_durability::GAME_MIGRATOR
                .run(&mut connection)
                .await?;
            connection.close().await?;
            // Keep future connections (including autovacuum) off the clone source.
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "ALTER DATABASE {building} ALLOW_CONNECTIONS false"
                ))))
                .await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "ALTER DATABASE {building} RENAME TO {template}"
                ))))
                .await?;
        }
        // Also finishes publication if the creating process died just after rename.
        // Templates live for the cluster's lifetime; CI clusters are throwaway.
        if is_template != Some(true) {
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "ALTER DATABASE {template} IS_TEMPLATE true"
                ))))
                .await?;
        }
        sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
            .bind(&template)
            .execute(&mut *admin)
            .await?;
        eprintln!(
            "PGTPL {} {template}",
            if is_template.is_some() {
                "reused"
            } else {
                "created"
            }
        );
        Ok(template)
    }

    async fn clone_database(admin: &mut PgConnection, name: &str, template: &str) -> TestResult {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            match admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {name} TEMPLATE {template}"
                ))))
                .await
            {
                Ok(_) => return Ok(()),
                Err(error)
                    if error
                        .as_database_error()
                        .and_then(|error| error.code())
                        .as_deref()
                        == Some("55006")
                        && Instant::now() < deadline =>
                {
                    // A just-closed source connection may still be exiting on the server.
                    // A real sleep: tokio time may be paused in the calling test.
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = sqlx::PgConnection::connect(&self.admin_url).await?;
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

fn fence_parent(tag: &str) -> TestResult<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let parent = std::env::temp_dir().join(format!(
        "oteryn-revision-sequencer-parent-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&parent)?;
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))?;
    let retained = parent.join(tag);
    let _ = std::fs::remove_dir_all(&retained);
    std::fs::create_dir(&retained)?;
    std::fs::set_permissions(&retained, std::fs::Permissions::from_mode(0o700))?;
    Ok(retained)
}

async fn register(root: &DurabilityRoot, tag: u8) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("sequencer-launch-{tag}")).map_err(debug)?;
    let node = crate::foundation::NodeId::decode(&id(tag)).map_err(debug)?;
    root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(debug)?;
    root.register_node_incarnation(&secret, &launch, node)
        .await
        .map_err(debug)
}

/// A fully migrated, unseeded database, scoped to one catalogue sweep. Every
/// clone is an independent database and still verifies the migration ledger.
#[allow(
    dead_code,
    reason = "only catalogue sweep targets share migration setup"
)]
pub(super) struct MigratedDatabaseTemplate(Database);

#[allow(
    dead_code,
    reason = "only catalogue sweep targets share migration setup"
)]
impl MigratedDatabaseTemplate {
    pub(super) async fn create(admin: String) -> TestResult<Self> {
        Ok(Self(Database::create(admin, "catalogue_template").await?))
    }

    pub(super) async fn cleanup(self) -> TestResult {
        self.0.cleanup().await
    }
}

pub(super) struct Harness {
    database: Database,
    pub(super) root: DurabilityRoot,
    pub(super) pool: sqlx::PgPool,
    pub(super) recovery: CharacterRecoveryStore,
    retained: std::path::PathBuf,
    pub(super) node: NodeIncarnationProof,
}

#[path = "character_revision_sequencer_postgres_cases/appearance_fixture.rs"]
mod appearance_fixture;
use appearance_fixture::CreationFixture;

impl Harness {
    pub(super) async fn create(admin: String, tag: &str, initialized: bool) -> TestResult<Self> {
        Self::create_with_admission_seed(admin, tag, initialized, true).await
    }
    pub(super) async fn create_with_admission_seed(
        admin: String,
        tag: &str,
        initialized: bool,
        seed_session: bool,
    ) -> TestResult<Self> {
        Self::create_seeded(
            admin,
            tag,
            initialized,
            seed_session,
            CreationFixture::Historical,
        )
        .await
    }
    #[allow(
        dead_code,
        reason = "C19b native dispatch scenarios create a root under the new interpretation"
    )]
    pub(super) async fn create_newhaven(admin: String, tag: &str) -> TestResult<Self> {
        Self::create_seeded_with_starter(
            admin,
            tag,
            false,
            false,
            CreationFixture::Historical,
            "newhaven-starter-r1",
        )
        .await
    }
    async fn create_seeded(
        admin: String,
        tag: &str,
        initialized: bool,
        seed_session: bool,
        creation: CreationFixture,
    ) -> TestResult<Self> {
        Self::create_seeded_with_starter(
            admin,
            tag,
            initialized,
            seed_session,
            creation,
            "starter-1",
        )
        .await
    }
    async fn create_seeded_with_starter(
        admin: String,
        tag: &str,
        initialized: bool,
        seed_session: bool,
        creation: CreationFixture,
        starter: &str,
    ) -> TestResult<Self> {
        let database = Database::create(admin, tag).await?;
        Self::create_on_database(database, initialized, seed_session, creation, starter).await
    }

    #[allow(
        dead_code,
        reason = "only catalogue sweep targets share migration setup"
    )]
    pub(super) async fn create_from_template(
        admin: String,
        tag: &str,
        template: &MigratedDatabaseTemplate,
    ) -> TestResult<Self> {
        if admin != template.0.admin_url {
            return Err("migration template belongs to another PostgreSQL cluster".into());
        }
        let database = Database::create_from(admin, tag, Some(&template.0.name)).await?;
        Self::create_on_database(
            database,
            false,
            false,
            CreationFixture::Historical,
            "starter-1",
        )
        .await
    }

    async fn create_on_database(
        database: Database,
        initialized: bool,
        seed_session: bool,
        creation: CreationFixture,
        starter: &str,
    ) -> TestResult<Self> {
        let root = DurabilityRoot::connect_test_runtime(&database.url)?;
        assert!(root.maintain_ready_once().await?);
        let pool = sqlx::PgPool::connect(&database.url).await?;
        // Scenarios may share a tag, but each database owns its recovery fence.
        let retained = fence_parent(&database.name)?;
        let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
            .map_err(debug)?;
        {
            let fresh = recovery.authorize_fresh_store(id(10), 100).map_err(debug)?;
            root.admit_fresh_character_recovery(&fresh)
                .await
                .map_err(debug)?;
        }
        let node = registered_node(&root, 1).await?;
        seed_character(
            &pool,
            &root,
            &node,
            initialized,
            seed_session,
            creation,
            starter,
        )
        .await?;
        Ok(Self {
            database,
            root,
            pool,
            recovery,
            retained,
            node,
        })
    }

    /// An independent DB-only owner for cross-root concurrency scenarios.
    #[allow(
        dead_code,
        reason = "shared fixture is mounted by targets without the concurrent reward case"
    )]
    pub(super) fn independent_root(&self) -> TestResult<DurabilityRoot> {
        Ok(DurabilityRoot::connect_test_runtime(&self.database.url)?)
    }

    pub(super) async fn cleanup(self) -> TestResult {
        self.pool.close().await;
        self.database.cleanup().await?;
        std::fs::remove_dir_all(self.retained)?;
        Ok(())
    }
}

fn bootstrap_binding(creation: CreationFixture, starter: &str) -> Vec<u8> {
    let appearance = creation != CreationFixture::Historical;
    let observer = creation == CreationFixture::Observer;
    let original_id = id;
    let id = |tag| {
        original_id(if observer && ![42, 43].contains(&tag) {
            tag + 60
        } else {
            tag
        })
    };
    let name = if observer {
        "Fixture Watcher"
    } else {
        "Fixture Hero"
    };
    let mut binding = vec![if appearance { 3 } else { 2 }];
    binding.extend_from_slice(&id(31));
    binding.extend_from_slice(&(if observer { 2_i64 } else { 1_i64 }).to_be_bytes());
    binding.extend_from_slice(&id(30));
    binding.extend_from_slice(&id(40));
    binding.extend_from_slice(&id(42));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&120_i64.to_be_bytes());
    for value in ["profile-1", "ruleset-1", "content-1", starter] {
        binding.extend_from_slice(&u16::try_from(value.len()).expect("length").to_be_bytes());
        binding.extend_from_slice(value.as_bytes());
    }
    // Contract version 2 binds the requested name last (CHAR-NAME-1).
    binding.extend_from_slice(
        &u16::try_from(name.len())
            .expect("name length")
            .to_be_bytes(),
    );
    binding.extend_from_slice(name.as_bytes());
    if let Some(sex) = creation.sex() {
        binding.push(sex as u8);
    }
    binding
}

struct RuntimeReadiness(AdmissionAuthorityPublicationChangeV1);

impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for RuntimeReadiness {}

impl AdmissionAuthorityOwningPublisherV1 for RuntimeReadiness {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

async fn publish_readiness(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    ownership_generation: u64,
) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(pool)
            .await?;
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: "game-runtime-publisher".into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: 1,
            decision_identity: "sequencer-runtime-ready-1".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        },
        publication_revision: 1,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready: true,
            route_revision: "route-1".into(),
            runtime_observation_revision: "runtime-1".into(),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "ruleset-1".into(),
            content_revision: "content-1".into(),
            map_revision: "map-1".into(),
            world_policy_revision: "world-policy-1".into(),
            offer_revision: "offer-1".into(),
        },
    };
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&RuntimeReadiness(change), now).map_err(debug)?;
    let disposition = root
        .publish_runtime_readiness(node, &publication)
        .await
        .map_err(debug)?;
    if disposition != GuardPublicationDisposition::Applied {
        return Err(format!("unexpected readiness outcome: {disposition:?}").into());
    }
    Ok(())
}

/// One Character (id 41) with a live session, lease, scope assignment and readiness, and, when
/// `initialized`, progression at revision one (level 50, 1000 experience).
async fn seed_character(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    initialized: bool,
    seed_session: bool,
    creation: CreationFixture,
    starter: &str,
) -> TestResult {
    let observer = creation == CreationFixture::Observer;
    let original_id = id;
    let id = |tag| {
        original_id(if observer && ![42, 43].contains(&tag) {
            tag + 60
        } else {
            tag
        })
    };
    let name = if observer {
        "Fixture Watcher"
    } else {
        "Fixture Hero"
    };
    if !observer {
        if starter == "starter-1" {
            sqlx::query(
                "INSERT INTO game_character_interpretations VALUES \
             (1,'profile-1','ruleset-1','content-1','starter-1',1)",
            )
            .execute(pool)
            .await?;
        } else {
            // The operator's actual append-only interpretation owner selects the new
            // template before root creation; no immutable existing root is rewritten.
            sqlx::query("SELECT game_character_configure_interpretation('profile-1','ruleset-1','content-1',$1)")
                .bind(starter).execute(pool).await?;
        }
    }
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(40).as_slice())
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1',$6,$5,$4)",
    )
    .bind(id(41).as_slice())
    .bind(id(40).as_slice())
    .bind(id(42).as_slice())
    .bind(creation.sex())
    .bind(name)
    .bind(starter)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_operation_receipts(\
           operation_id,command_binding,account_id,character_id,world_id,character_revision,\
           event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,\
           intent_source_revision,issued_at_source,expires_at_source) \
         VALUES (encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
           encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',\
           encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)",
    )
    .bind(id(30).as_slice())
    .bind(bootstrap_binding(creation, starter))
    .bind(id(40).as_slice())
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(id(32).as_slice())
    .bind(id(33).as_slice())
    .bind(id(31).as_slice())
    .bind(if observer { 2_i64 } else { 1_i64 })
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_bootstrap_intent_floors VALUES \
         (1,$3,encode($1,'hex')::uuid,$2) ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding",
    )
    .bind(id(31).as_slice())
    .bind(bootstrap_binding(creation, starter))
    .bind(if observer { 2_i64 } else { 1_i64 })
    .execute(pool)
    .await?;
    if initialized {
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES \
             (encode($1,'hex')::uuid,1,50,1000,'profile-1','ruleset-1','content-1',\
              'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
        )
        .bind(id(41).as_slice())
        .execute(pool)
        .await?;
    }

    if seed_session {
        sqlx::query(
            "INSERT INTO game_durability_reconnect_sessions(\
           game_session_id,account_id,character_id,world_id,runtime_scope_kind,\
           runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,\
           original_grace_deadline,predecessor_generation,character_lease_generation,\
           scope_ownership_generation,current_generation,current_transport_ref,session_state) \
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
           encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,\
           1,999999,1,1,1,1,$6,1)",
        )
        .bind(id(50).as_slice())
        .bind(id(40).as_slice())
        .bind(id(41).as_slice())
        .bind(id(42).as_slice())
        .bind(id(43).as_slice())
        .bind([9_u8; 16].as_slice())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_durability_admission_account_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,'test',1,'account-current',1,0,'{}')",
        )
        .bind(id(40).as_slice())
        .bind(id(41).as_slice())
        .bind(id(50).as_slice())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_durability_admission_character_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
        )
        .bind(id(41).as_slice())
        .bind(id(40).as_slice())
        .bind(id(42).as_slice())
        .bind(id(50).as_slice())
        .execute(pool)
        .await?;
    }
    if observer {
        return Ok(());
    }
    let fact = node.fact();
    sqlx::query(
        "INSERT INTO game_control_scope_grants \
         (control_role, world_id, channel_id, operation) \
         VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1)",
    )
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .execute(pool)
    .await?;
    let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let scope = RuntimeScopeRefV1::channel(
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
    );
    let assignment = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([7_u8; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Assign {
                scope,
                target: fact,
            },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = assignment else {
        return Err(format!("unexpected assignment outcome: {assignment:?}").into());
    };
    if seed_session {
        publish_readiness(
            pool,
            root,
            node,
            scope,
            receipt.assignment.ownership_generation,
        )
        .await?;
    }
    Ok(())
}

pub(super) fn fence(revision: u64) -> TestResult<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(41)).map_err(debug)?,
        game_session_id: crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        expected_character_revision: CharacterRevision::new(revision).map_err(debug)?,
    })
}

fn configured_admin() -> Option<String> {
    match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!(
                "PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured"
            );
            None
        }
    }
}

fn charm(name: &str) -> CharmKey {
    CharmKey::new(format!("oteryn:charm.{name}")).expect("charm key")
}

fn race(name: &str) -> BestiaryRaceKey {
    BestiaryRaceKey::new(format!("oteryn:creature.{name}")).expect("race key")
}

/// Candidate-catalogue costs (#1293) for three majors and two minors.
fn catalogue() -> CharmCatalogue {
    let major = |name: &str, costs| CharmDefinition {
        key: charm(name),
        category: CharmCategory::Major,
        stage_costs: costs,
    };
    let minor = |name: &str| CharmDefinition {
        key: charm(name),
        category: CharmCategory::Minor,
        stage_costs: [100, 150, 225],
    };
    CharmCatalogue::new([
        major("wound", [240, 360, 1200]),
        major("poison", [240, 360, 1200]),
        major("zap", [320, 480, 1600]),
        minor("gut"),
        minor("bless"),
    ])
    .expect("catalogue")
}

fn occurrence(tag: u8) -> CharmCommandOccurrence {
    CharmCommandOccurrence::from_bytes(id(tag)).expect("occurrence")
}

fn unlock(tag: u8, name: &str) -> CharmCommandRequest {
    CharmCommandRequest {
        occurrence: occurrence(tag),
        command: CharmCommand::UnlockNextStage { charm: charm(name) },
        catalogue_revision: "content-1".into(),
        catalogue: catalogue(),
    }
}

/// Test double for the facts CHARM-2 and later slices own. It reads nothing from the
/// transaction except a liveness probe on it, so a closed connection still fails the command.
#[derive(Clone)]
struct Facts {
    stages: BTreeMap<BestiaryRaceKey, u8>,
    points: Vec<u32>,
    promoted: bool,
    slots: CharmSlotEntitlement,
}

impl Facts {
    fn new(points: &[u32], stages: &[(&str, u8)]) -> Self {
        Self {
            stages: stages
                .iter()
                .map(|(name, stage)| (race(name), *stage))
                .collect(),
            points: points.to_vec(),
            promoted: false,
            slots: CharmSlotEntitlement::Free,
        }
    }
}

impl CharmFacts for Facts {
    async fn completed_stage(
        &self,
        connection: &mut PgConnection,
        _character: CharacterId,
        race: &BestiaryRaceKey,
    ) -> Result<BestiaryStage, DurabilityError> {
        sqlx::query("SELECT 1").execute(&mut *connection).await?;
        BestiaryStage::new(self.stages.get(race).copied().unwrap_or(0))
            .map_err(|_| DurabilityError::InvalidStoredState)
    }

    async fn completed_entry_charm_points(
        &self,
        connection: &mut PgConnection,
        _character: CharacterId,
    ) -> Result<Vec<u32>, DurabilityError> {
        sqlx::query("SELECT 1").execute(&mut *connection).await?;
        Ok(self.points.clone())
    }

    async fn promoted(
        &self,
        _connection: &mut PgConnection,
        _character: CharacterId,
    ) -> Result<bool, DurabilityError> {
        Ok(self.promoted)
    }

    async fn slot_entitlement(
        &self,
        _connection: &mut PgConnection,
        _character: CharacterId,
    ) -> Result<CharmSlotEntitlement, DurabilityError> {
        Ok(self.slots)
    }
}

pub(super) fn xp_request(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
    let context = ProgressionRevisionContext {
        profile: "profile-1".into(),
        ruleset: "ruleset-1".into(),
        content: "content-1".into(),
        simulation: "simulation-1".into(),
        evidence: "evidence-1".into(),
        declaration: "declaration-1".into(),
    };
    Ok(ExperienceAwardRequest {
        occurrence: ExperienceRewardOccurrence::from_bytes(id(tag)).map_err(debug)?,
        amount: ExactI64::new(5),
        context: context.clone(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: FiniteProgressionPolicy {
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            death_policy_revision: "death-1".into(),
            declared_difference_revision: "declaration-1".into(),
            thresholds: [
                LevelThreshold {
                    level: 50,
                    minimum_experience: ExactI64::new(1000),
                },
                LevelThreshold {
                    level: 51,
                    minimum_experience: ExactI64::new(1100),
                },
            ],
            terminal_exclusive_experience: ExactI64::new(1200),
            death_loss_numerator: 1,
            death_loss_denominator: 10,
            death_loss_rounding: RoundingMode::Floor,
        },
    })
}

pub(super) fn run<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    // Keep the executor's stack frame and monomorphization independent of the
    // scenario's future size. Every scenario still runs to completion here.
    let future: std::pin::Pin<Box<dyn Future<Output = TestResult> + '_>> = Box::pin(body(admin));
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future)
}

#[cfg(not(oteryn_pg_shard_secondary))]
include!("character_revision_sequencer_pg_tests.rs");

#[allow(
    dead_code,
    unused_imports,
    reason = "test-only wrapper consumed by the composed dispatch tests"
)]
#[path = "pvp_source_death_fixture.rs"]
pub(super) mod pvp_source_death_fixture;
#[path = "character_revision_sequencer_postgres_cases/tutorial_vitals_fixture.rs"]
pub(super) mod tutorial_vitals_fixture;
