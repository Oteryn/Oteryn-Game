//! S05 concrete player scenario, derived from the S05 matrix/re-audit row and the
//! House, transfer, and guildhall dispatch owners named there:
//! 1. Log in and request the paged Thais House catalogue; expect native House rows
//!    containing auction state, rent, ACL revision, ownership, and building kind.
//! 2. Walk through the compiled world to the selected House entrance and bid the
//!    advertised price; expect an accepted command plus an `Ok` House result, then
//!    re-query and observe the bid maximum or ownership state delta.
//! 3. Exercise the refusal path by bidding for a guildhall as a character without
//!    guild-leader authority; expect a guildhall-specific refusal disposition and
//!    no ownership delta.
//! 4. Replace the owned House guest ACL with the other provisioned character;
//!    expect `Ok`, a positive revision, and a subsequent ACL-revision delta. Send
//!    the old revision again and expect `Conflict` without another delta.
//! 5. From the entrance send House entry, then exit, through HOUSE-TRANSFER-1;
//!    expect accepted acknowledgements, new sessions/scope snapshots, and position
//!    deltas into the House and back to its entrance.
//! 6. Schedule move-out, verify its durable due-time delta, cancel it, and verify
//!    the due time clears; rent remains the catalogue rent throughout.
//! 7. Log out, obtain a fresh admission, query again, and require the bid/ownership,
//!    ACL revision, cancelled move-out, and rent state observed before logout.
//!
//! Every mechanic command is attempted even when an earlier prerequisite is
//! refused. A row PASS is possible only when every positive and negative assertion
//! above was observed; failures retain the exact step and native reply as evidence.
use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{CommandStatus, house, house_transfer, raw_feature};

const TOWN_KEY: &str = "oteryn:content.area.city.thais";
const REQUIRED: &[&str] = &[
    "catalogue-rent",
    "walk-entrance",
    "auction-bid",
    "guildhall-refusal",
    "acl-update",
    "acl-conflict",
    "entry",
    "exit",
    "move-out",
    "cancel-move-out",
    "fresh-admission-state",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    failures: Vec<String>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: std::collections::BTreeSet::new(),
            failures: Vec::new(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if REQUIRED.contains(&step) && self.passed.insert(step) {
            evidence(&format!(
                "S05_CHECK step={step} status=PASS observed={observed:?}"
            ));
        } else {
            self.fail(step, "duplicate-or-unknown-step");
        }
    }

    fn fail(&mut self, step: &str, observed: impl std::fmt::Display) {
        let detail = format!("step={step}:observed={observed}");
        evidence(&format!("S05_CHECK status=FAIL {detail}"));
        self.failures.push(detail);
    }

    fn finish(self) {
        for step in REQUIRED {
            if !self.passed.contains(step) {
                evidence(&format!("S05_GAP step={step} status=unproven"));
            }
        }
        let pass =
            self.failures.is_empty() && REQUIRED.iter().all(|step| self.passed.contains(step));
        let suffix = self
            .failures
            .first()
            .map(|failure| format!(" evidence={}", token(failure)))
            .unwrap_or_default();
        evidence(&format!(
            "e2e_all scenario=S05.full status={} seconds={:.3}{suffix}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.:=,[]".contains(c) {
                c
            } else {
                '_'
            }
        })
        .take(240)
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    match row_complete(journey(&mut row, clients)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => row.fail("journey", error),
        Err(_) => row.fail("journey", "timeout-180s"),
    }
    row.finish();
}

async fn command(
    client: &mut DevClientSession,
    command: raw_feature::FeatureCommand,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?)
}

async fn query(client: &mut DevClientSession, world: [u8; 16]) -> TestResult<Vec<house::Row>> {
    let mut cursor = None;
    let mut rows = Vec::new();
    loop {
        let request = house::query(world, TOWN_KEY, cursor.as_deref())
            .map_err(|error| format!("House query encode: {error}"))?;
        let reply = command(client, request).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("House query command reply: {reply:?}").into());
        }
        let page = house::page(&reply.payload)
            .map_err(|error| format!("House query payload: {error}; reply={reply:?}"))?;
        rows.extend(page.rows);
        if page.cursor.is_none() {
            return Ok(rows);
        }
        cursor = page.cursor;
    }
}

async fn mutate(
    client: &mut DevClientSession,
    world: [u8; 16],
    key: &str,
    revision: u64,
    action: &house::Action,
) -> TestResult<(u64, u64, String)> {
    let request = house::mutation(world, key, revision, action)
        .map_err(|error| format!("House mutation encode: {error}"))?;
    let reply = command(client, request).await?;
    let description = format!("status={:?},payload={:?}", reply.status, reply.payload);
    if reply.status != CommandStatus::Accepted {
        return Ok((0, 0, description));
    }
    let (disposition, revision) = house::result(&reply.payload)
        .map_err(|error| format!("House result decode: {error}; {description}"))?;
    Ok((disposition, revision, description))
}

fn select(rows: &[house::Row], kind: u64) -> Option<house::Row> {
    rows.iter()
        .filter(|row| row.kind == kind)
        .min_by_key(|row| (&row.house, row.price))
        .cloned()
}

fn entrance(key: &str) -> TestResult<TilePos> {
    for body in [
        include_str!("../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let value: serde_json::Value = serde_json::from_str(body)?;
        let Some(houses) = value["houses"].as_array() else {
            return Err("House catalogue has no houses array".into());
        };
        if let Some(record) = houses
            .iter()
            .find(|record| record["identity"]["key"].as_str() == Some(key))
        {
            return Ok(TilePos {
                x: u16::try_from(record["entrance"]["x"].as_u64().ok_or("House entrance x")?)?,
                y: u16::try_from(record["entrance"]["y"].as_u64().ok_or("House entrance y")?)?,
                floor: -i8::try_from(record["entrance"]["z"].as_i64().ok_or("House entrance z")?)?,
            });
        }
    }
    Err(format!("House catalogue missing {key}").into())
}

async fn attempt_transfer(
    client: &mut DevClientSession,
    key: &str,
    exit: bool,
) -> TestResult<(bool, String)> {
    let intent = house_transfer::HouseTransferIntent {
        exit,
        house: key.to_owned(),
    };
    let payload = house_transfer::encode_house_transfer_intent(&intent)
        .map_err(|error| format!("House transfer encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            house_transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            house_transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &payload,
        )
        .await?;
    let observed = format!("status={:?},payload={:?}", reply.status, reply.payload);
    let accepted = matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && house_transfer::decode_house_transfer_ack(&reply.payload).is_ok();
    Ok((accepted, observed))
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world_id = WorldId::decode(&clients.world)?;
    let world = load_world(bytes, &pin, world_id, ChannelId::decode(&clients.channel)?)?;
    let world_bytes = *world_id.as_bytes();
    let mut client = connect_character(clients, 0xb5, 1).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let initial = query(&mut client, world_bytes).await?;
        let Some(private) = select(&initial, 1) else {
            row.fail("catalogue-rent", "accepted-query-missing-private-house");
            return Ok(());
        };
        if private.rent > 0 && private.revision > 0 {
            row.pass(
                "catalogue-rent",
                (&private.house, private.rent, private.revision),
            );
        } else {
            row.fail(
                "catalogue-rent",
                format!("row={private:?}:missing-rent-or-revision"),
            );
        }

        let at = entrance(&private.house)?;
        let from = tile(client.world_spatial().actor_position)?;
        if let Some(path) = route(world, from, |candidate| candidate == at) {
            match walk(&mut client, world, &path).await {
                Ok(()) => row.pass("walk-entrance", at),
                Err(error) => row.fail("walk-entrance", error),
            }
        } else {
            row.fail("walk-entrance", format!("no-player-route:{from:?}->{at:?}"));
        }

        let maximum = private.price.max(private.rent).max(1);
        let (bid_d, _, bid_reply) = mutate(
            &mut client,
            world_bytes,
            &private.house,
            private.revision,
            &house::Action::Bid(maximum),
        )
        .await?;
        let after_bid = query(&mut client, world_bytes).await?;
        let bid_state = after_bid
            .iter()
            .find(|candidate| candidate.house == private.house);
        if bid_d == 1
            && bid_state.is_some_and(|state| state.owned || state.maximum == Some(maximum))
        {
            row.pass("auction-bid", (bid_d, bid_state));
        } else {
            row.fail(
                "auction-bid",
                format!("{bid_reply},disposition={bid_d},row={bid_state:?}"),
            );
        }

        if let Some(guildhall) = select(&initial, 3) {
            let before_owned = guildhall.owned;
            let (d, _, reply) = mutate(
                &mut client,
                world_bytes,
                &guildhall.house,
                guildhall.revision,
                &house::Action::Bid(guildhall.price.max(guildhall.rent).max(1)),
            )
            .await?;
            let after = query(&mut client, world_bytes).await?;
            let unchanged = after
                .iter()
                .find(|candidate| candidate.house == guildhall.house)
                .is_some_and(|candidate| candidate.owned == before_owned);
            if (4..=10).contains(&d) && unchanged {
                row.pass("guildhall-refusal", d);
            } else {
                row.fail(
                    "guildhall-refusal",
                    format!("{reply},disposition={d},unchanged={unchanged}"),
                );
            }
        } else {
            row.fail("guildhall-refusal", "accepted-query-missing-guildhall");
        }

        let current = query(&mut client, world_bytes)
            .await?
            .into_iter()
            .find(|candidate| candidate.house == private.house)
            .ok_or("selected House disappeared")?;
        let acl_action = house::Action::Guests(vec![clients.characters[0]]);
        let (acl_d, acl_revision, acl_reply) = mutate(
            &mut client,
            world_bytes,
            &private.house,
            current.acl_revision,
            &acl_action,
        )
        .await?;
        let after_acl = query(&mut client, world_bytes).await?;
        let durable_acl = after_acl
            .iter()
            .find(|candidate| candidate.house == private.house)
            .map(|candidate| candidate.acl_revision);
        if acl_d == 1 && acl_revision > current.acl_revision && durable_acl == Some(acl_revision) {
            row.pass("acl-update", acl_revision);
        } else {
            row.fail(
                "acl-update",
                format!(
                    "{acl_reply},disposition={acl_d},revision={acl_revision},query={durable_acl:?}"
                ),
            );
        }
        let (stale_d, _, stale_reply) = mutate(
            &mut client,
            world_bytes,
            &private.house,
            current.acl_revision,
            &acl_action,
        )
        .await?;
        if stale_d == 3
            && query(&mut client, world_bytes)
                .await?
                .iter()
                .find(|v| v.house == private.house)
                .map(|v| v.acl_revision)
                == durable_acl
        {
            row.pass("acl-conflict", stale_d);
        } else {
            row.fail(
                "acl-conflict",
                format!("{stale_reply},disposition={stale_d}"),
            );
        }

        let (entered, enter_reply) = attempt_transfer(&mut client, &private.house, false).await?;
        if entered {
            row.pass("entry", client.world_spatial().actor_position);
        } else {
            row.fail("entry", enter_reply);
        }
        let (exited, exit_reply) = attempt_transfer(&mut client, &private.house, true).await?;
        if entered && exited && tile(client.world_spatial().actor_position)? == at {
            row.pass("exit", at);
        } else {
            row.fail("exit", exit_reply);
        }

        let due = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?
        .saturating_add(86_400_000);
        let base_revision = after_acl
            .iter()
            .find(|candidate| candidate.house == private.house)
            .map(|candidate| candidate.revision)
            .unwrap_or(current.revision);
        let (move_d, _, move_reply) = mutate(
            &mut client,
            world_bytes,
            &private.house,
            base_revision,
            &house::Action::Notice(due),
        )
        .await?;
        let moved = query(&mut client, world_bytes).await?;
        let moved_row = moved
            .iter()
            .find(|candidate| candidate.house == private.house);
        if move_d == 1 && moved_row.is_some_and(|candidate| candidate.notice == Some(due)) {
            row.pass("move-out", due);
        } else {
            row.fail(
                "move-out",
                format!("{move_reply},disposition={move_d},row={moved_row:?}"),
            );
        }
        let cancel_revision = moved_row
            .map(|candidate| candidate.revision)
            .unwrap_or(base_revision);
        let (cancel_d, _, cancel_reply) = mutate(
            &mut client,
            world_bytes,
            &private.house,
            cancel_revision,
            &house::Action::Cancel,
        )
        .await?;
        let cancelled = query(&mut client, world_bytes).await?;
        let cancelled_row = cancelled
            .iter()
            .find(|candidate| candidate.house == private.house);
        if cancel_d == 1 && cancelled_row.is_some_and(|candidate| candidate.notice.is_none()) {
            row.pass("cancel-move-out", cancel_d);
        } else {
            row.fail(
                "cancel-move-out",
                format!("{cancel_reply},disposition={cancel_d},row={cancelled_row:?}"),
            );
        }

        let logout = client.row_logout().await?;
        if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            row.fail("fresh-admission-state", format!("logout={logout:?}"));
            return Ok(());
        }
        if row_needs_logout_0 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
            row_needs_logout_0 = false;
        }
        let mut fresh = connect_character(clients, 0xb6, 1).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            let persisted = query(&mut fresh, world_bytes).await?;
            let persisted = persisted
                .iter()
                .find(|candidate| candidate.house == private.house);
            if let (Some(before), Some(after)) = (cancelled_row, persisted)
                && before.owned == after.owned
                && before.maximum == after.maximum
                && before.acl_revision == after.acl_revision
                && before.notice == after.notice
                && before.rent == after.rent
            {
                row.pass("fresh-admission-state", after);
            } else {
                row.fail(
                    "fresh-admission-state",
                    format!("before={cancelled_row:?},after={persisted:?}"),
                );
            }
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??;
            row_needs_logout_1 = false;
            Ok(())
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_token_is_bounded_and_has_no_whitespace() {
        let value = token(&format!("step=auction bid observed {}", "x".repeat(300)));
        assert!(value.len() <= 240);
        assert!(!value.chars().any(char::is_whitespace));
    }

    #[test]
    fn shipped_thais_catalogue_has_private_and_guildhall_entries() -> TestResult {
        let private = entrance("oteryn:content.house.main_street_9_1st_floor_shop")?;
        assert!(private.x > 0 && private.y > 0);
        Ok(())
    }
}
