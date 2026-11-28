//! P22c (tracker persistence across login):
//! 1. Admit the node-boot character and prove the race is initially absent on the wire and in SQL.
//!    Track it and require the durable tracker row to exist.
//! 2. Log out, freshly admit the character and require the tracker read and the durable row to
//!    still list the race.
//! 3. Remove it, log out, admit again and require it gone from the read and from the table.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_tracker as tr};
use sqlx::Connection;

const BAT: &str = "oteryn:creature.bat";
const RAT: &str = "oteryn:creature.rat";

// The same character is used for all three admissions and the durable lookup.
const CHARACTER_INDEX: usize = 1;

async fn connect_tracker(clients: &SeamClients<'_>, tag: u8) -> TestResult<DevClientSession> {
    let index = CHARACTER_INDEX;
    wait_character_released(clients, index).await?;
    let token = if requires_platform_grant(&clients.accounts[index]) {
        platform_row_grant(clients, index).await?
    } else {
        let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
        let (nonce, attempt) = fresh_grant_material(tag)?;
        let grant = Grant {
            signing: clients.signing,
            key_id: clients.key_id,
            account_id: &clients.accounts[index],
            character_id: clients.characters[index],
            world_id: clients.world,
            channel_id: clients.channel,
            security_generation: generation,
            scope_generation: clients.scope_generation,
            nonce,
            attempt,
        };
        sign_grant(&grant, now_seconds()?)
    };
    let mut capabilities = oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES.to_vec();
    capabilities.push(tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1);
    capabilities.sort_unstable();
    capabilities.dedup();
    let client = oteryn_dev_client::connect_session_with_capabilities(
        oteryn_dev_client::JoinRequest {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
        },
        &capabilities,
    )
    .await?;
    verdict::Harness::global().progress("admission");
    selected(&client)?;
    Ok(client)
}

fn selected(client: &DevClientSession) -> TestResult {
    expect(
        client
            .selected_capabilities()
            .contains(&tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1),
        "capability",
        "needed=capability-197-selected-at-join",
    )
}

/// One tracker command; the reply snapshot is `None` for an empty refusal payload.
async fn call(
    client: &mut DevClientSession,
    command: u32,
    request: &tr::TrackerRequest,
) -> TestResult<(CommandStatus, Option<tr::TrackerSnapshot>)> {
    let payload =
        tr::encode_request(request).ok_or("step=encode;observed=request-not-encodable")?;
    let reply = client
        .session_mut()
        .gated_command(tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1, command, &payload)
        .await?;
    let snapshot = if reply.payload.is_empty() {
        None
    } else {
        Some(
            tr::decode_snapshot(&reply.payload)
                .ok_or("step=decode;observed=undecodable-tracker-snapshot")?,
        )
    };
    expect(
        reply.status != CommandStatus::Accepted || snapshot.as_ref().is_some_and(|list| !list.boss),
        "bestiary-snapshot",
        reply.status,
    )?;
    Ok((reply.status, snapshot))
}

fn change(action: tr::TrackerAction, key: &str) -> tr::TrackerRequest {
    tr::TrackerRequest::Change {
        action,
        creature_key: key.to_owned(),
    }
}

fn expected_bestiary(keys: &[&str]) -> tr::TrackerSnapshot {
    tr::TrackerSnapshot {
        boss: false,
        entries: keys
            .iter()
            .map(|key| tr::TrackerEntry {
                creature_key: (*key).to_owned(),
                kill_count: 0,
                // Both creatures are common, trivial Bestiary entries in Global.
                thresholds: [10, 100, 250],
                completed: false,
            })
            .collect(),
    }
}

fn expect_login_snapshot(
    frame: Option<(u32, u64, &[u8])>,
    expected: &tr::TrackerSnapshot,
) -> TestResult {
    let Some((kind, revision, payload)) = frame else {
        return Err("step=login-tracker;observed=missing-domain-98".into());
    };
    let decoded = tr::decode_domain_snapshot(payload);
    expect(
        kind == tr::SNAPSHOT_TYPE_CYCLOPEDIA_TRACKER_V1
            && revision == 1
            && decoded.as_ref().is_some_and(|(bestiary, bosstiary)| {
                bestiary == expected && bosstiary.entries.is_empty()
            }),
        "login-tracker",
        (kind, revision, decoded),
    )
}

fn check_login(client: &mut DevClientSession, expected: &tr::TrackerSnapshot) -> TestResult {
    let frame = client
        .session_mut()
        .gated_snapshot(tr::STATE_DOMAIN_CYCLOPEDIA_TRACKER);
    expect_login_snapshot(
        frame.map(|frame| {
            (
                frame.snapshot_type,
                frame.revision,
                frame.payload.as_slice(),
            )
        }),
        expected,
    )
}

fn expect_command_snapshot(
    reply: (CommandStatus, Option<tr::TrackerSnapshot>),
    expected: &tr::TrackerSnapshot,
    step: &str,
) -> TestResult {
    expect(
        reply.0 == CommandStatus::Accepted && reply.1.as_ref() == Some(expected),
        step,
        reply,
    )
}

fn expect_durable(rows: &[(i16, String)], keys: &[&str], step: &str) -> TestResult {
    expect(
        rows.iter()
            .map(|(kind, key)| (*kind, key.as_str()))
            .eq(keys.iter().map(|key| (1, *key))),
        step,
        rows,
    )
}

async fn durable(clients: &SeamClients<'_>) -> TestResult<Vec<(i16, String)>> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    // The expected list has at most two rows. Fetch one extra to detect any surplus.
    let rows = sqlx::query_as(
        "SELECT tracker_kind, creature_key FROM game_character_cyclopedia_tracker \
         WHERE character_id=encode($1,'hex')::uuid ORDER BY tracker_kind, creature_key LIMIT 3",
    )
    .bind(clients.characters[CHARACTER_INDEX].as_slice())
    .fetch_all(&mut connection)
    .await?;
    Ok(rows)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let best = tr::COMMAND_TYPE_BESTIARY_TRACKER;
    let read = tr::TrackerRequest::Read;
    let empty = expected_bestiary(&[]);
    let baseline = expected_bestiary(&[BAT]);
    let tracked = expected_bestiary(&[BAT, RAT]);
    let mut client = connect_tracker(clients, 0xd3).await?;
    check_login(&mut client, &empty)?;
    expect_command_snapshot(
        call(&mut client, best, &read).await?,
        &empty,
        "read-before-add",
    )?;
    expect_durable(&durable(clients).await?, &[], "durable-before-baseline")?;
    // Track Bat through the same wire command so a faulty Rat mutation cannot clear it.
    expect_command_snapshot(
        call(&mut client, best, &change(tr::TrackerAction::Add, BAT)).await?,
        &baseline,
        "add-bat",
    )?;
    expect_durable(&durable(clients).await?, &[BAT], "durable-before-add")?;
    expect_command_snapshot(
        call(&mut client, best, &change(tr::TrackerAction::Add, RAT)).await?,
        &tracked,
        "add",
    )?;
    expect_durable(&durable(clients).await?, &[BAT, RAT], "durable-after-add")?;
    logout(&mut client).await?;

    let mut client = connect_tracker(clients, 0xd4).await?;
    check_login(&mut client, &tracked)?;
    expect_command_snapshot(
        call(&mut client, best, &read).await?,
        &tracked,
        "read-after-relog",
    )?;
    expect_durable(&durable(clients).await?, &[BAT, RAT], "durable-after-relog")?;
    expect_command_snapshot(
        call(&mut client, best, &change(tr::TrackerAction::Remove, RAT)).await?,
        &baseline,
        "remove",
    )?;
    logout(&mut client).await?;

    let mut client = connect_tracker(clients, 0xd5).await?;
    check_login(&mut client, &baseline)?;
    expect_command_snapshot(
        call(&mut client, best, &read).await?,
        &baseline,
        "read-after-remove-relog",
    )?;
    expect_durable(&durable(clients).await?, &[BAT], "durable-after-remove")?;
    logout(&mut client).await?;
    Ok((
        "PASS",
        "tracker-persists-across-login-and-removal-persists".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P22c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

fn expect_first_logout(result: Option<oteryn_protocol_oteryn::logout::LogoutResult>) -> TestResult {
    expect(
        result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout-first-attempt",
        result,
    )
}

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    expect_first_logout(client.session_mut().logout().await?.result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_checks_reject_missing_stale_and_wrong_lists() {
        let tracked = expected_bestiary(&[BAT, RAT]);
        let empty = expected_bestiary(&[]);
        let boss = tr::TrackerSnapshot {
            boss: true,
            entries: Vec::new(),
        };
        let frame = |list: &tr::TrackerSnapshot, bosses: &tr::TrackerSnapshot| {
            tr::encode_domain_snapshot(list, bosses).unwrap()
        };
        let valid = frame(&tracked, &boss);
        assert!(expect_login_snapshot(Some((1, 1, &valid)), &tracked).is_ok());
        assert!(expect_login_snapshot(None, &tracked).is_err());
        assert!(expect_login_snapshot(Some((1, 2, &valid)), &tracked).is_err());
        assert!(expect_login_snapshot(Some((1, 1, b"bad")), &tracked).is_err());

        let mut duplicate = tracked.clone();
        duplicate.entries.push(tracked.entries[0].clone());
        let mut spurious = duplicate.clone();
        spurious.entries[2].creature_key = "oteryn:creature.cave_rat".to_owned();
        let mut wrong_progress = tracked.clone();
        wrong_progress.entries[0].kill_count = 1;
        for wrong in [
            empty.clone(),
            expected_bestiary(&[RAT]),
            duplicate,
            spurious,
            wrong_progress,
        ] {
            let payload = frame(&wrong, &boss);
            assert!(expect_login_snapshot(Some((1, 1, &payload)), &tracked).is_err());
            assert!(
                expect_command_snapshot((CommandStatus::Accepted, Some(wrong)), &tracked, "test")
                    .is_err()
            );
        }
        assert!(
            expect_command_snapshot((CommandStatus::Accepted, Some(tracked)), &empty, "pre-add")
                .is_err()
        );
        assert!(expect_durable(&[(1, RAT.to_owned())], &[BAT], "pre-add").is_err());
        assert!(
            expect_durable(
                &[(1, RAT.to_owned()), (2, "unexpected".to_owned())],
                &[BAT, RAT],
                "test"
            )
            .is_err()
        );
    }

    #[test]
    fn first_logout_busy_is_a_failure() {
        use oteryn_protocol_oteryn::logout::LogoutResult;

        assert!(expect_first_logout(Some(LogoutResult::Accepted)).is_ok());
        let error = expect_first_logout(Some(LogoutResult::Busy)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "step=logout-first-attempt;observed=Some(Busy)"
        );
    }

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
