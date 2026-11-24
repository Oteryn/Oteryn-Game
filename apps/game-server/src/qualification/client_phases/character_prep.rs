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
