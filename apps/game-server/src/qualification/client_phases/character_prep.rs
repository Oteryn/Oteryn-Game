//! Seeded rows use the disposable node's root-only control socket and then
//! re-admit to read the durable state through the ordinary player path.
use super::*;

pub(super) fn declare_seeded(row: &str, prerequisites: &str, reason: &str) {
    evidence(&format!(
        "row={row} mode=seeded prerequisites={prerequisites} reason={reason:?}"
    ));
}

pub(super) struct Seeded {
    pub(super) profile: &'static str,
    pub(super) reason: &'static str,
}

impl Seeded {
    pub(super) async fn connect(
        &self,
        clients: &SeamClients<'_>,
        tag: u8,
        index: usize,
    ) -> TestResult<oteryn_dev_client::DevClientSession> {
        self.connect_at(clients, tag, index, "NODE_BOOT_CONTROL_SOCKET")
            .await
    }

    pub(super) async fn connect_at(
        &self,
        clients: &SeamClients<'_>,
        tag: u8,
        index: usize,
        socket_env: &str,
    ) -> TestResult<oteryn_dev_client::DevClientSession> {
        evidence(&format!(
            "character_preparation mode=Seeded profile={} reason={:?}",
            self.profile, self.reason
        ));
        let mut client = connect_character(clients, tag, index).await?;
        let session = client.join_snapshot().game_session_id;
        request_fixture_control_at(
            socket_env,
            uuid_text(session.as_bytes()),
            "--profile",
            self.profile.into(),
        )
        .await
        .map_err(|error| format!("character preparation {} refused: {error}", self.profile))?;
        if let Some((dx, dy)) = match self.profile {
            "f5-c17a" => Some((-1, -1)),
            "f5-c17c" => Some((0, -1)),
            "f5-c13" => Some((1, -1)),
            "f5-c18a" => Some((-1, 1)),
            "f5-c17d" => Some((1, 1)),
            _ => None,
        } {
            let world = full_e2ef7::qualification_world(clients)?;
            let from = tile(client.world_spatial().actor_position)?;
            let basis = spatial_basis::current(&mut client)?;
            let path = route(world, from, |at| {
                at.floor == TOWN.floor
                    && at.x.abs_diff(TOWN.x).max(at.y.abs_diff(TOWN.y)) <= 12
                    && at.x.abs_diff(TOWN.x).max(at.y.abs_diff(TOWN.y)) >= 4
                    && (i32::from(at.x) - i32::from(TOWN.x)).signum() == dx
                    && (i32::from(at.y) - i32::from(TOWN.y)).signum() == dy
                    && world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| tile.protection_zone())
                    && client.world_entities().is_some_and(|view| {
                        view.iter()
                            .all(|actor| actor.position != basis.position(at))
                    })
            });
            if let Some(path) = path {
                walk(&mut client, world, &path).await?;
            } else {
                evidence(&format!(
                    "character_preparation profile={} no reachable protected staging tile from={from:?}",
                    self.profile
                ));
            }
        }
        let (result, detail, _, _) = logout_retrying_busy(&mut client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("character preparation logout refused: {detail}").into());
        }
        drop(client);
        let fresh = connect_character(clients, tag, index).await?;
        if fresh.join_snapshot().game_session_id == session {
            return Err("character preparation reused its original session".into());
        }
        Ok(fresh)
    }
}

pub(super) struct DedicatedCharacter {
    accounts: [String; 1],
    characters: [[u8; 16]; 1],
}

impl DedicatedCharacter {
    pub(super) fn replace_slot(
        &self,
        scope: &SeamClients<'_>,
        index: usize,
    ) -> TestResult<(Vec<String>, Vec<[u8; 16]>)> {
        let mut accounts = scope.accounts.to_vec();
        let mut characters = scope.characters.to_vec();
        *accounts
            .get_mut(index)
            .ok_or("preparation account slot absent")? = self.accounts[0].clone();
        *characters
            .get_mut(index)
            .ok_or("preparation character slot absent")? = self.characters[0];
        Ok((accounts, characters))
    }

    pub(super) fn clients<'a>(&'a self, scope: &SeamClients<'a>) -> SeamClients<'a> {
        SeamClients {
            accounts: &self.accounts,
            characters: &self.characters,
            ..*scope
        }
    }
}

pub(super) async fn dedicated_character(
    scope: &SeamClients<'_>,
    account_env: &str,
) -> TestResult<DedicatedCharacter> {
    dedicated_named_character(scope, account_env, None).await
}

pub(super) async fn dedicated_named_character(
    scope: &SeamClients<'_>,
    account_env: &str,
    name: Option<&str>,
) -> TestResult<DedicatedCharacter> {
    let account = required(account_env)?;
    let account_id = crate::node::uuid_bytes(&account).ok_or("invalid preparation account")?;
    if scope
        .accounts
        .iter()
        .any(|existing| crate::node::uuid_bytes(existing) == Some(account_id))
    {
        return Err("preparation account overlaps a shared fixture".into());
    }
    let mut database = sqlx::PgConnection::connect(scope.url).await?;
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid
           AND ($3::text IS NULL OR name=$3) LIMIT 2",
    )
    .bind(&account)
    .bind(scope.world.as_slice())
    .bind(name)
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    let [row] = rows.as_slice() else {
        return Err("preparation account must have exactly one character in this world".into());
    };
    Ok(DedicatedCharacter {
        accounts: [account],
        characters: [row.as_slice().try_into()?],
    })
}

pub(super) async fn grant_item(
    client: &oteryn_dev_client::DevClientSession,
    id: u32,
    count: u32,
) -> TestResult {
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        format!("item-{id}-{count}"),
    )
    .await
    .map_err(|error| format!("backpack grant for item {id} x{count}: {error}"))?;
    Ok(())
}

pub(super) async fn grant_premium(client: &oteryn_dev_client::DevClientSession) -> TestResult {
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "premium".into(),
    )
    .await?;
    Ok(())
}

pub(super) fn item_id(key: &str) -> TestResult<u32> {
    Ok(key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("seeded item has no numeric Tibia id")?
        .parse()?)
}

pub(super) async fn reload(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    let (result, detail, _, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("inventory preparation logout refused: {detail}").into());
    }
    *client = connect_character(clients, 0x70, 1).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("inventory preparation reused its original session".into());
    }
    Ok(())
}

/// Bootstrap a row-owned character through the disposable Platform and node
/// control socket. No gameplay state is granted by this preparation.
pub(super) async fn bootstrap_row_character(
    scope: &SeamClients<'_>,
    account: &str,
    operation: &str,
    name: &str,
) -> TestResult<DedicatedCharacter> {
    let work = required("NODE_BOOT_PLATFORM_GRANT_DIR")?;
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let world = uuid_text(&scope.world);
    let account_id = account.to_owned();
    let operation = operation.to_owned();
    let name = name.to_owned();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("python3")
            .args([
                "-B",
                "-c",
                r#"
import json, os, secrets, subprocess, sys, tempfile
from pathlib import Path

source, work, socket, account, world, operation, name = sys.argv[1:]
work, socket = Path(work), Path(socket)
sys.path.insert(0, str(Path(source) / 'tools/qualification/game_platform'))
from composition import bootstrap

stage = 'fixture inputs'
try:
    node = socket.parent.parent
    env = json.loads((work / 'portal/secrets/env.json').read_text())
    binary = json.loads((work / 'identity.json').read_text())['portal_binary']
    roster = work / 'row-accounts.json'
    accounts = json.loads(roster.read_text())
    if account in accounts:
        raise ValueError('row account already exists')
    credentials = dict(email=f'nb-{account}@example.invalid', password=secrets.token_urlsafe(48))
    stage = 'Platform account'
    subprocess.run([binary, 'seed', 'account', json.dumps(dict(accountId=account, **credentials))],
                   env=env, capture_output=True, timeout=20, check=True)
    stage = 'Platform character intent'
    bootstrap(work, account, world, operation,
              ['nb-profile-1', 'nb-ruleset-1', 's3b-content-1', 'newhaven-starter-r1'],
              name, 'Sorcerer')
    stage = 'node character bootstrap'
    operator = [] if os.environ.get('NODE_BOOT_UNPRIVILEGED') == '1' else ['sudo', '-n']
    subprocess.run([*operator, node / 'bin/oteryn-game-ops', '--config', node / 'ops/ops.toml',
                    'character', 'bootstrap', '--socket', socket, '--operation-id', operation],
                   capture_output=True, timeout=20, check=True)
    stage = 'Platform grant credentials'
    accounts[account] = credentials
    with tempfile.NamedTemporaryFile(mode='w', dir=work, delete=False) as updated:
        json.dump(accounts, updated)
    os.replace(updated.name, roster)
except (OSError, KeyError, ValueError, subprocess.SubprocessError):
    print(stage)
    sys.exit(1)
"#,
                concat!(env!("CARGO_MANIFEST_DIR"), "/../.."),
            ])
            .args([work, socket, account_id, world, operation, name])
            .output()
    })
    .await??;
    if !output.status.success() {
        let stage = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "dedicated row character bootstrap failed at {}",
            stage.trim()
        )
        .into());
    }
    let mut database = sqlx::PgConnection::connect(scope.url).await?;
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2",
    )
    .bind(account)
    .bind(scope.world.as_slice())
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    let [character] = rows.as_slice() else {
        return Err("dedicated row account must have exactly one character".into());
    };
    Ok(DedicatedCharacter {
        accounts: [account.to_owned()],
        characters: [character.as_slice().try_into()?],
    })
}

/// Wait for the same two real Platform publications as the native gateway
/// qualification before requesting a grant for a freshly created row character.
async fn wait_row_gateway_ready(clients: &SeamClients<'_>) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let path =
        Path::new(&required("NODE_BOOT_PLATFORM_GRANT_DIR")?).join("portal/secrets/env.json");
    let settings: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let url = settings["OTERYN_DATABASE_URL"]
        .as_str()
        .ok_or("disposable Platform database address absent")?;
    tokio::time::timeout_at(deadline, async {
        let mut database = sqlx::PgConnection::connect(url)
            .await
            .map_err(|_| "disposable Platform readiness database unavailable")?;
        loop {
            let (projected, ready): (bool, bool) = sqlx::query_as(
                "SELECT EXISTS(SELECT 1 FROM account_characters
                   WHERE account_id=$1::uuid AND character_id=encode($2,'hex')::uuid
                     AND world_id=encode($3,'hex')::uuid AND availability='AVAILABLE'),
                 EXISTS(SELECT 1 FROM native_runtime_status_reports
                   WHERE world_id=encode($3,'hex')::uuid AND channel_id=encode($4,'hex')::uuid
                     AND NOT invalid AND content_json::jsonb->>'ready'='true'
                     AND observed_at >= extract(epoch FROM clock_timestamp())::bigint - 10)",
            )
            .bind(&clients.accounts[0])
            .bind(clients.characters[0].as_slice())
            .bind(clients.world.as_slice())
            .bind(clients.channel.as_slice())
            .fetch_one(&mut database)
            .await
            .map_err(|_| "read exact Platform projection and runtime readiness failed")?;
            if projected && ready {
                return database
                    .close()
                    .await
                    .map_err(|_| "close Platform readiness database failed");
            }
            if tokio::time::Instant::now() + Duration::from_millis(100) >= deadline {
                return Err(match (projected, ready) {
                    (true, false) => "Platform runtime status stayed unavailable before row admission",
                    (false, true) => "Platform character projection absent before row admission",
                    _ => "Platform character projection and runtime status absent before row admission",
                });
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| "Platform row admission readiness deadline expired")?
    .map_err(Into::into)
}

/// Admit a row-owned character with an actual Platform grant on every entry.
pub(super) async fn connect_row_character(
    clients: &SeamClients<'_>,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    wait_character_released(clients, 0).await?;
    wait_row_gateway_ready(clients).await?;
    let token = platform_row_grant(clients, 0).await?;
    let client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address: clients.address,
        server_name: "localhost",
        root_certificate: clients.certificate,
        schema_revision: 1,
        character_id: crate::foundation::CharacterId::decode(&clients.characters[0])?,
        admission_material: token.as_bytes(),
        client_build_id: "oteryn-dev-client/world-qualification",
        deadline: Duration::from_secs(20),
    })
    .await?;
    verdict::Harness::global().progress("admission");
    Ok(client)
}
