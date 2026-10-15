use std::error::Error;
use std::future::Future;
use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as SeqOrdering};

use oteryn_protocol_oteryn::chat::{ChatIntent, ChatLine, ChatRoom};
use oteryn_simulation_determinism::SemanticTimeMicros;
use sqlx::{Connection as _, PgConnection};

use super::*;
use crate::durability::MigrationExecutor;
use crate::foundation::GameSessionId;

type TestResult = Result<(), Box<dyn Error>>;

const WORLD: [u8; 16] = [
    0x01, 0x90, 0x0a, 0x5d, 0xac, 0x96, 0x7a, 0xb2, 0x80, 0xe7, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
];
const BASE: SemanticTimeMicros = SemanticTimeMicros::from_micros(1_000_000);
static DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn key_line(id: u8, activate: u64) -> String {
    format!("{id} {activate} {}\n", STANDARD.encode([id; 32]))
}

fn ring() -> RelayKeyRing {
    parse_keys(key_line(1, 0).as_bytes()).expect("ring")
}

fn session_of(tag: u8) -> GameSessionId {
    GameSessionId::decode(&[
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ])
    .expect("session")
}

#[test]
fn the_key_file_is_parsed_and_malformed_files_are_refused() {
    let two = format!("{}\n  {}", key_line(1, 0), key_line(2, 5_000));
    assert!(parse_keys(two.as_bytes()).is_ok());
    for bad in [
        String::new(),
        "1 0\n".to_owned(),
        format!("{} extra", key_line(1, 0).trim_end()),
        format!("256 0 {}", STANDARD.encode([1_u8; 32])),
        format!("1 -1 {}", STANDARD.encode([1_u8; 32])),
        format!("1 0 {}", STANDARD.encode([1_u8; 31])),
        "1 0 not-base64!\n".to_owned(),
    ] {
        assert!(parse_keys(bad.as_bytes()).is_err(), "{bad:?}");
    }
    assert_eq!(
        parse_keys(&[0xff, 0xfe]).err(),
        Some(RelayConfigError::Format)
    );
    let duplicate = format!("{}{}", key_line(1, 0), key_line(1, 9));
    assert_eq!(
        parse_keys(duplicate.as_bytes()).err(),
        Some(RelayConfigError::Ring)
    );
}

#[test]
fn the_world_channel_follows_the_uuid_text() {
    assert_eq!(
        uuid_text(&WORLD),
        "01900a5d-ac96-7ab2-80e7-0123456789ab".to_owned()
    );
    assert_eq!(
        channel_name(&uuid_text(&WORLD)).expect("channel"),
        "oteryn_chat_01900a5dac967ab280e70123456789ab"
    );
}

#[test]
fn configuration_errors_name_a_reason_only() {
    for (error, reason) in [
        (RelayConfigError::File, "key_file_unreadable"),
        (RelayConfigError::Format, "key_file_malformed"),
        (RelayConfigError::Ring, "key_ring_invalid"),
        (RelayConfigError::World, "world_id_invalid"),
        (RelayConfigError::Endpoint, "database_endpoint_unavailable"),
    ] {
        assert_eq!(error.to_string(), reason);
    }
}

#[test]
fn a_relay_that_never_listened_refuses_to_send() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let options =
            PgConnectOptions::from_str("postgresql://nobody@127.0.0.1:1/none").expect("options");
        let relay = ChatRelay::new(ring(), WORLD, options).expect("relay");
        assert!(!relay.is_ready());
        let sent = relay
            .send(
                RelayDestination::Room {
                    room: ChatRoom::Help,
                },
                CharacterName::parse("Aela").expect("name"),
                ChatText::parse("hello").expect("text"),
            )
            .await;
        assert_eq!(sent, Err(ChatDisposition::ChatUnavailable));
    });
}

// ---- PostgreSQL: the relay against a real LISTEN/NOTIFY server --------------------------------

struct IsolatedDatabase {
    admin_url: String,
    database_name: String,
}

impl IsolatedDatabase {
    async fn create(test_name: &str) -> Result<Self, Box<dyn Error>> {
        let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        if !(admin_url.starts_with("postgresql://oteryn_test_admin:")
            && (admin_url.contains("@127.0.0.1:5432/") || admin_url.contains("@localhost:5432/"))
            && admin_url.ends_with("/postgres")
            && !admin_url.contains(['?', '#', '\n', '\r']))
        {
            return Err("unsafe PostgreSQL test-admin URL".into());
        }
        let ordinal = DB_SEQUENCE.fetch_add(1, SeqOrdering::Relaxed);
        let database_name = format!(
            "oteryn_game_review_{}_{}_{test_name}",
            std::process::id(),
            ordinal
        );
        let mut admin = PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {database_name}"
            ))))
            .await?;
        Ok(Self {
            admin_url,
            database_name,
        })
    }

    fn database_url(&self) -> Result<String, Box<dyn Error>> {
        let prefix = self
            .admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid PostgreSQL test-admin URL")?;
        Ok(format!("{prefix}/{}", self.database_name))
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = PgConnection::connect(&self.admin_url).await?;
        sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = $1 AND pid <> pg_backend_pid()",
        )
        .bind(&self.database_name)
        .execute(&mut admin)
        .await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE IF EXISTS {}",
                self.database_name
            ))))
            .await?;
        Ok(())
    }
}

fn run_postgres_test<F>(future: F) -> TestResult
where
    F: Future<Output = TestResult>,
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(Box::pin(future))
}

/// A migrated database; `quiet` makes it assert that no bind parameter is logged.
async fn relay_database(
    test_name: &str,
    quiet: bool,
) -> Result<(IsolatedDatabase, String), Box<dyn Error>> {
    let database = IsolatedDatabase::create(test_name).await?;
    let url = database.database_url()?;
    MigrationExecutor::connect_migration(&url)
        .await?
        .apply_embedded_ledger()
        .await?;
    if quiet {
        let mut admin = PgConnection::connect(&database.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "ALTER DATABASE {} SET log_parameter_max_length = 0",
                database.database_name
            ))))
            .await?;
    }
    Ok((database, url))
}

/// A node: its relay installed in its chat runtime, with one admitted session.
fn node(url: &str, session: GameSessionId, name: &str, character: [u8; 16]) -> ChatRuntime {
    let chat = ChatRuntime::default();
    let options = PgConnectOptions::from_str(url).expect("options");
    chat.install_relay(Arc::new(
        ChatRelay::new(ring(), WORLD, options).expect("relay"),
    ));
    chat.admit_as(session, name, Some(character), BASE);
    chat
}

fn relay_of(chat: &ChatRuntime) -> &ChatRelay {
    chat.relay().expect("installed relay")
}

/// Waits up to `seconds` for `done`.
async fn wait_for(seconds: u64, mut done: impl FnMut() -> bool) -> bool {
    for _ in 0..seconds * 20 {
        if done() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    done()
}

fn character(marker: u8) -> [u8; 16] {
    let mut bytes = [marker; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

#[test]
fn two_nodes_of_a_world_exchange_room_and_private_lines() -> TestResult {
    run_postgres_test(async {
        let (database, url) = relay_database("chat_relay_exchange", true).await?;
        let (aela, bryn) = (session_of(0x71), session_of(0x72));
        let first = node(&url, aela, "Aela", character(0x31));
        let second = node(&url, bryn, "Bryn", character(0x32));
        let shutdown = CancellationToken::new();
        // A window with no startup floor: the rules of the floor are tested in `chat::relay`.
        let listeners = async {
            let _ = first_of(
                relay_of(&first).run_from(Some(ReplayWindow::starting_at(0)), &first, &shutdown),
                relay_of(&second).run_from(Some(ReplayWindow::starting_at(0)), &second, &shutdown),
            )
            .await;
        };
        let exchange = async {
            assert!(
                wait_for(10, || relay_of(&first).is_ready()
                    && relay_of(&second).is_ready())
                .await,
                "both relays listen"
            );
            for (chat, session) in [(&first, aela), (&second, bryn)] {
                assert!(
                    chat.pre_gate(session, &ChatIntent::OpenRoom(ChatRoom::Help), BASE)
                        .is_some_and(|done| done.disposition == ChatDisposition::Ok)
                );
            }
            relay_of(&first)
                .send(
                    RelayDestination::Room {
                        room: ChatRoom::Help,
                    },
                    CharacterName::parse("Aela").expect("name"),
                    ChatText::parse("hello world").expect("text"),
                )
                .await
                .expect("sent");
            let mut heard = Vec::new();
            assert!(
                wait_for(10, || {
                    heard.extend(second.drain(bryn));
                    !heard.is_empty()
                })
                .await
            );
            assert_eq!(
                heard,
                vec![ChatLine::Room {
                    room: ChatRoom::Help,
                    speaker_name: "Aela".to_owned(),
                    text: "hello world".to_owned(),
                }]
            );
            // The sender's own listener delivers the line back to its session too.
            assert!(wait_for(10, || !first.drain(aela).is_empty()).await);

            let recipient =
                oteryn_protocol_oteryn::CharacterId::decode(&character(0x32)).expect("character");
            relay_of(&first)
                .send(
                    RelayDestination::Private { recipient },
                    CharacterName::parse("Aela").expect("name"),
                    ChatText::parse("psst").expect("text"),
                )
                .await
                .expect("sent");
            let mut heard = Vec::new();
            assert!(
                wait_for(10, || {
                    heard.extend(second.drain(bryn));
                    !heard.is_empty()
                })
                .await
            );
            assert_eq!(
                heard,
                vec![ChatLine::Private {
                    speaker_name: "Aela".to_owned(),
                    text: "psst".to_owned(),
                }]
            );
            // Only the recipient's node delivers a private line.
            tokio::time::sleep(Duration::from_millis(300)).await;
            assert!(first.drain(aela).is_empty());
        };
        let finished = first_of(Box::pin(exchange), Box::pin(listeners)).await;
        shutdown.cancel();
        drop((first, second));
        database.cleanup().await?;
        finished.ok_or("the relays stopped")?;
        Ok(())
    })
}

#[test]
fn a_listening_relay_stores_its_world_lease() -> TestResult {
    run_postgres_test(async {
        let (database, url) = relay_database("chat_relay_lease", true).await?;
        let chat = node(&url, session_of(0x71), "Aela", character(0x31));
        let shutdown = CancellationToken::new();
        let check = async {
            assert!(wait_for(10, || relay_of(&chat).is_ready()).await);
            let mut connection = PgConnection::connect(&url).await.expect("connect");
            let leased: i64 = sqlx::query_scalar(
                "SELECT leased_until_micros FROM game_chat_relay_leases \
                 WHERE world_id = '01900a5d-ac96-7ab2-80e7-0123456789ab'",
            )
            .fetch_one(&mut connection)
            .await
            .expect("lease row");
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000000)::bigint",
            )
            .fetch_one(&mut connection)
            .await
            .expect("clock");
            assert!(leased > now, "the lease runs ahead of the database clock");
        };
        let finished = first_of(
            Box::pin(check),
            Box::pin(relay_of(&chat).run(&chat, &shutdown)),
        )
        .await;
        shutdown.cancel();
        drop(chat);
        database.cleanup().await?;
        finished.ok_or("the relay stopped")?;
        Ok(())
    })
}

#[test]
fn a_database_that_may_log_bind_parameters_leaves_the_relay_not_ready() -> TestResult {
    run_postgres_test(async {
        let (database, url) = relay_database("chat_relay_logged", false).await?;
        let chat = node(&url, session_of(0x71), "Aela", character(0x31));
        let shutdown = CancellationToken::new();
        let check = async {
            assert!(!wait_for(3, || relay_of(&chat).is_ready()).await);
            assert_eq!(
                chat.pre_gate(
                    session_of(0x71),
                    &ChatIntent::Room {
                        room: ChatRoom::Help,
                        text: "x".to_owned(),
                    },
                    BASE,
                )
                .map(|done| done.disposition),
                Some(ChatDisposition::ChatUnavailable)
            );
        };
        let finished = first_of(
            Box::pin(check),
            Box::pin(relay_of(&chat).run(&chat, &shutdown)),
        )
        .await;
        shutdown.cancel();
        drop(chat);
        database.cleanup().await?;
        finished.ok_or("the relay stopped")?;
        Ok(())
    })
}

/// `super::first` with a stop future that never ends on its own.
async fn first_of<T>(
    primary: impl Future<Output = T>,
    other: impl Future<Output = ()>,
) -> Option<T> {
    first(primary, other).await
}
