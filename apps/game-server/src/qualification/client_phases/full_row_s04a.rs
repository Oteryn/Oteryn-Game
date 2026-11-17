//! S04a full-row scenario, derived from the matrix/re-audit and the native VIP owner:
//! 1. Admit two ordinary node-boot characters, discover the target's served player name, and
//!    send `Query`; expect an accepted code-1 snapshot with a revisioned list and groups.
//! 2. Send `Add(target name)`; expect code 1 and a following `Query` delta containing exactly
//!    the returned list-local entry number and target name.
//! 3. Send `GroupAdd`, then `Edit(entry, comment, icon, notify, [group])`, `GroupRename`, and
//!    `Query`; expect each accepted code-1 reply and the queried entry/group metadata changes.
//! 4. Send duplicate `Add(target name)` as the refusal path; expect code 3 (ALREADY_LISTED),
//!    no revision change, and a following query identical to the pre-refusal view.
//! 5. While the target is admitted, consume the native VIP-presence delta and expect ONLINE;
//!    logout the target and expect OFFLINE, freshly admit it and expect ONLINE plus the login
//!    notification bit. These are server-pushed deltas, not inferred socket state.
//! 6. Logout the watcher, make a fresh admission, and `Query` again; expect the durable entry,
//!    group, comment/icon/notify assignment and revision to match the pre-logout view.
//! 7. Send `GroupRemove` while the entry still uses that group; expect code 1 and a query delta
//!    removing the group from both the catalogue and entry, then logout both characters normally.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::vip::{self, vip as wire};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::vip_presence::Status;
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "query",
    "add",
    "groups-and-metadata",
    "duplicate-refusal",
    "online-presence",
    "offline-presence",
    "login-notification",
    "fresh-admission-durability",
    "group-remove",
    "logout",
];

#[derive(Default)]
struct Row {
    passed: BTreeSet<&'static str>,
    invalid: bool,
    started: Option<std::time::Instant>,
}

impl Row {
    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if !ACTIONS.contains(&action) || !self.passed.insert(action) {
            self.invalid = true;
            return;
        }
        evidence(&format!(
            "S04A_CHECK step={action} status=PASS observed={observed:?}"
        ));
    }

    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("S04A_GAP status=FAIL observed={error}"));
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("S04A_GAP step={action} observed=not-completed"));
            }
        }
        let complete = !self.invalid && ACTIONS.iter().all(|item| self.passed.contains(item));
        evidence(&format!(
            "e2e_all scenario=S04a.full status={} seconds={:.3}{}",
            if result.is_ok() && complete {
                "PASS"
            } else {
                "FAIL"
            },
            self.started
                .map_or(0.0, |started| started.elapsed().as_secs_f64()),
            result
                .err()
                .map(|error| format!(" evidence={}", compact(&error.to_string())))
                .unwrap_or_default()
        ));
    }
}

fn compact(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:=,".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: Some(std::time::Instant::now()),
        ..Row::default()
    };
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => {
            Err("step=deadline observed=timeout-180s need=responsive-VIP-owner-and-relay".into())
        }
    });
}

fn request(operation: wire::Operation, number: u64, text: &str) -> wire::Request {
    wire::Request {
        operation,
        number,
        text: text.to_owned(),
        icon: 0,
        notify: false,
        groups: Vec::new(),
    }
}

async fn command(
    client: &mut DevClientSession,
    request: &wire::Request,
    step: &str,
) -> TestResult<wire::ResultView> {
    let command = vip::VipCommand::new(request)
        .ok_or_else(|| format!("step={step} observed=client-encoder-refused-request"))?;
    let response = vip::send(client.session_mut(), &command)
        .await
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step={step} observed=transport-error:{error}"), error) })?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "step={step} observed=status:{:?},code:{} need=accepted-command-frame",
            response.status, response.result.code
        )
        .into());
    }
    Ok(response.result)
}

async fn ok(
    client: &mut DevClientSession,
    request: &wire::Request,
    step: &str,
) -> TestResult<wire::ResultView> {
    let result = command(client, request, step).await?;
    if result.code != 1 {
        return Err(format!(
            "step={step} observed=accepted-code:{}-revision:{} need=code-1-server-reply",
            result.code, result.revision
        )
        .into());
    }
    Ok(result)
}

fn view(result: &wire::ResultView, step: &str) -> TestResult<serde_json::Value> {
    serde_json::from_str(&result.view)
        .map_err(|error| format!("step={step} observed=invalid-view:{error}").into())
}

fn peer_name(client: &DevClientSession, peer: &DevClientSession) -> TestResult<String> {
    let identity = peer
        .world_entities()
        .ok_or("step=discover-target observed=peer-entity-domain-missing")?
        .own_identity();
    client
        .world_entities()
        .ok_or("step=discover-target observed=watcher-entity-domain-missing")?
        .iter()
        .find(|entity| &entity.entity.identity == identity)
        .and_then(|entity| entity.display_name)
        .ok_or("step=discover-target observed=visible-peer-name-missing need=served-player-name")?
        .as_str()
        .map(str::to_owned)
        .map_err(|error| {
            format!("step=discover-target observed=invalid-player-name:{error:?}").into()
        })
}

async fn wait_presence(
    client: &mut DevClientSession,
    entry: u64,
    status: Status,
    announce: Option<bool>,
    step: &str,
) -> TestResult<oteryn_protocol_oteryn::vip_presence::PresenceEntry> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::VipPresence(delta) = event
                && let Some(value) = delta.value.entries.into_iter().find(|item| {
                    item.entry == entry
                        && item.status == status
                        && announce.is_none_or(|expected| item.announce == expected)
                })
            {
                return Ok(value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            let current = client.session_mut().vip_presence();
            return Err(format!(
                "step={step} observed=presence-timeout:current={current:?} need=VIP-presence-delta:{status:?}"
            )
            .into());
        }
    }
}

async fn logout(client: &mut DevClientSession, step: &str) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step={step} observed=logout-reply:{reply:?}").into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut watcher = connect_character(clients, 0xb0, 0).await?;
    let mut target = connect_character(clients, 0xb1, 1).await?;
    for capability in [7, 43, 258] {
        if !watcher.selected_capabilities().contains(&capability) {
            return Err(format!(
                "step=admission observed=capability-{capability}-not-selected need=VIP-list-chat-presence"
            )
            .into());
        }
    }
    watcher.service_liveness(Duration::from_millis(100)).await?;
    target.service_liveness(Duration::from_millis(100)).await?;
    let name = peer_name(&watcher, &target)?;

    let initial = ok(
        &mut watcher,
        &request(wire::Operation::Query, 0, ""),
        "query",
    )
    .await?;
    let initial_view = view(&initial, "query")?;
    if !initial_view["entries"].is_array() || !initial_view["groups"].is_array() {
        return Err(
            "step=query observed=view-missing-entries-or-groups need=complete-list-view".into(),
        );
    }
    row.pass("query", (initial.revision, &initial_view));

    let added = ok(
        &mut watcher,
        &request(wire::Operation::Add, 0, &name),
        "add",
    )
    .await?;
    if added.number == 0 || added.revision <= initial.revision {
        return Err(
            format!("step=add observed=reply:{added:?} need=new-entry-and-revision-delta").into(),
        );
    }
    let entry = added.number;
    let after_add = ok(
        &mut watcher,
        &request(wire::Operation::Query, added.revision, ""),
        "query-after-add",
    )
    .await?;
    let after_add_view = view(&after_add, "query-after-add")?;
    if !after_add_view["entries"].as_array().is_some_and(|entries| {
        entries.iter().any(|item| {
            item["number"].as_u64() == Some(entry) && item["name"].as_str() == Some(&name)
        })
    }) {
        return Err(format!(
            "step=add observed=entry-absent:{after_add_view} need=added-list-delta"
        )
        .into());
    }
    row.pass("add", (entry, added.revision));

    let group = ok(
        &mut watcher,
        &request(wire::Operation::GroupAdd, 0, "Raid Friends"),
        "group-add",
    )
    .await?;
    if group.number == 0 {
        return Err(
            format!("step=group-add observed=reply:{group:?} need=group-local-number").into(),
        );
    }
    let group_number = u8::try_from(group.number).map_err(|_| {
        format!(
            "step=group-add observed=out-of-range-number:{}",
            group.number
        )
    })?;
    let edited = ok(
        &mut watcher,
        &wire::Request {
            operation: wire::Operation::Edit,
            number: entry,
            text: "node-boot friend".into(),
            icon: 7,
            notify: true,
            groups: vec![group_number],
        },
        "edit-entry",
    )
    .await?;
    let renamed = ok(
        &mut watcher,
        &request(wire::Operation::GroupRename, group.number, "Raid Team"),
        "group-rename",
    )
    .await?;
    let retained = ok(
        &mut watcher,
        &request(wire::Operation::Query, renamed.revision, ""),
        "query-after-group-edit",
    )
    .await?;
    let retained_view = view(&retained, "query-after-group-edit")?;
    let entry_ok = retained_view["entries"].as_array().is_some_and(|entries| {
        entries.iter().any(|item| {
            item["number"].as_u64() == Some(entry)
                && item["description"].as_str() == Some("node-boot friend")
                && item["icon"].as_u64() == Some(7)
                && item["notify"].as_bool() == Some(true)
                && item["groups"].as_array().is_some_and(|groups| {
                    groups
                        .iter()
                        .any(|value| value.as_u64() == Some(group.number))
                })
        })
    });
    let group_ok = retained_view["groups"].as_array().is_some_and(|groups| {
        groups.iter().any(|item| {
            item["number"].as_u64() == Some(group.number)
                && item["name"].as_str() == Some("Raid Team")
        })
    });
    if !entry_ok || !group_ok || edited.revision > retained.revision {
        return Err(format!("step=groups-and-metadata observed=view:{retained_view} need=edited-entry-and-renamed-group").into());
    }
    row.pass("groups-and-metadata", (group.number, retained.revision));

    let duplicate = command(
        &mut watcher,
        &request(wire::Operation::Add, 0, &name),
        "duplicate-add-refusal",
    )
    .await?;
    if duplicate.code != 3 || duplicate.revision != retained.revision {
        return Err(format!(
            "step=duplicate-refusal observed=reply:{duplicate:?} need=code-3-no-revision-delta"
        )
        .into());
    }
    let unchanged = ok(
        &mut watcher,
        &request(wire::Operation::Query, retained.revision, ""),
        "query-after-refusal",
    )
    .await?;
    if view(&unchanged, "query-after-refusal")? != retained_view {
        return Err("step=duplicate-refusal observed=view-changed need=atomic-refusal".into());
    }
    row.pass("duplicate-refusal", duplicate.code);

    let online = wait_presence(
        &mut watcher,
        entry,
        Status::Online,
        Some(false),
        "online-presence",
    )
    .await?;
    row.pass("online-presence", online);
    logout(&mut target, "target-logout").await?;
    let offline = wait_presence(
        &mut watcher,
        entry,
        Status::Offline,
        Some(true),
        "offline-presence",
    )
    .await?;
    row.pass("offline-presence", offline);
    drop(target);
    let mut target = connect_character(clients, 0xb2, 1).await?;
    let login = wait_presence(
        &mut watcher,
        entry,
        Status::Online,
        Some(true),
        "login-notification",
    )
    .await?;
    row.pass("login-notification", login);

    let old_session = watcher.join_snapshot().game_session_id;
    logout(&mut watcher, "watcher-logout-for-relog").await?;
    drop(watcher);
    let mut watcher = connect_character(clients, 0xb3, 0).await?;
    if watcher.join_snapshot().game_session_id == old_session {
        return Err(
            "step=fresh-admission-durability observed=session-reused need=fresh-game-session"
                .into(),
        );
    }
    let fresh = ok(
        &mut watcher,
        &request(wire::Operation::Query, retained.revision, ""),
        "fresh-query",
    )
    .await?;
    if fresh.revision != retained.revision || view(&fresh, "fresh-query")? != retained_view {
        return Err(format!(
            "step=fresh-admission-durability observed=reply:{fresh:?} need=identical-durable-view"
        )
        .into());
    }
    row.pass("fresh-admission-durability", fresh.revision);

    let remove = ok(
        &mut watcher,
        &request(wire::Operation::GroupRemove, group.number, ""),
        "group-remove",
    )
    .await?;
    if remove.revision <= fresh.revision {
        return Err(format!(
            "step=group-remove observed=reply:{remove:?} need=revisioned-code-1-delta"
        )
        .into());
    }
    let final_view = ok(
        &mut watcher,
        &request(wire::Operation::Query, fresh.revision, ""),
        "query-after-group-remove",
    )
    .await?;
    let final_value = view(&final_view, "query-after-group-remove")?;
    let removed_from_catalogue = final_value["groups"].as_array().is_some_and(|groups| {
        !groups
            .iter()
            .any(|item| item["number"].as_u64() == Some(group.number))
    });
    let removed_from_entry = final_value["entries"].as_array().is_some_and(|entries| {
        entries
            .iter()
            .find(|item| item["number"].as_u64() == Some(entry))
            .is_some_and(|item| {
                item["groups"].as_array().is_some_and(|groups| {
                    !groups
                        .iter()
                        .any(|value| value.as_u64() == Some(group.number))
                })
            })
    });
    if !removed_from_catalogue || !removed_from_entry {
        return Err(format!(
            "step=group-remove observed=view:{final_value} need=group-and-membership-removed"
        )
        .into());
    }
    row.pass("group-remove", remove.revision);
    logout(&mut watcher, "watcher-final-logout").await?;
    logout(&mut target, "target-final-logout").await?;
    row.pass("logout", "both-accepted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_needs_every_unique_action() {
        let mut row = Row::default();
        for action in ACTIONS {
            row.pass(action, "test");
        }
        assert!(!row.invalid);
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        row.pass(ACTIONS[0], "duplicate");
        assert!(row.invalid);
    }

    #[test]
    fn evidence_compaction_is_single_token() {
        assert_eq!(compact("step=x observed=code 5"), "step=x_observed=code_5");
    }
}
