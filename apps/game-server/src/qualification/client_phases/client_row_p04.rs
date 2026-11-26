//! P04 client replay capture: real TASK_BOARD replies for the Query and a refused StartWeekly with
//! a difficulty above the character’s level. The consumer is `apply_full_result` and `panel_text` in
//! `crates/client-ui/src/features/0072-task-board`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;
use oteryn_dev_client::client_runtime::task_board;
use task_board::task_board::full::{
    Bounty, ListSlot, Reply, Request, ShopOffer, TalismanPath, Weekly, decode_reply, encode_request,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    let root = required("E2E_ALL_CAPTURE_DIR")?;
    let mut file = std::fs::File::create_new(Path::new(&root).join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    action: u8,
    selection: u16,
    name: &str,
) -> TestResult<(Reply, GatedCommandResult)> {
    let payload = encode_request(&Request {
        action,
        selection,
        race: String::new(),
    })
    .map_err(|error| format!("request encoder: {error:?}"))?;
    let result = client
        .session_mut()
        .gated_command(
            task_board::task_board::CAPABILITY_TASK_BOARD_V1,
            task_board::task_board::COMMAND_TYPE_TASK_BOARD,
            &payload,
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("task-board action={action} status={:?}", result.status).into());
    }
    let session = client.join_snapshot().game_session_id;
    capture(name, &frame_envelope(session.as_bytes(), &result)?)?;
    let reply =
        decode_reply(&result.payload).map_err(|error| format!("reply decode: {error:?}"))?;
    Ok((reply, result))
}

fn frame_envelope(session: &[u8; 16], result: &GatedCommandResult) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec(&serde_json::json!({
        "command_id": result.command_id,
        "game_session_id": uuid_text(session),
        "payload": result.payload,
        "server_sequence": result.server_sequence,
        "status": result.status as u32,
    }))
}

async fn expected_period(clients: &SeamClients<'_>) -> TestResult<u64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let period: i64 = sqlx::query_scalar(
        "SELECT (date_trunc('week', (e.activated_at AT TIME ZONE 'Europe/Berlin')
                    - interval '6 hours')::date - DATE '1970-01-01')::bigint
           FROM game_world_reset_active a
           JOIN game_world_reset_epochs e USING (world_id, reset_epoch)
          WHERE a.world_id = $1::uuid AND e.calendar = 'TIBIA_SERVER_SAVE_BERLIN_06'",
    )
    .bind(uuid_text(&clients.world))
    .fetch_one(&mut connection)
    .await?;
    Ok(u64::try_from(period)?)
}

fn expected_shop() -> Vec<ShopOffer> {
    // Crystal 201189f8 data/scripts/lib/task_board_shop.lua:33–248 and the
    // approved portable grappling-hook substitution. Values are fixture data.
    [
        (0, 2, 1907, 0, 0, 100000),
        (1, 2, 1907, 0, 1, 35000),
        (2, 2, 1907, 0, 2, 35000),
        (3, 2, 1282, 0, 0, 100000),
        (4, 2, 1282, 0, 1, 35000),
        (5, 2, 1282, 0, 2, 35000),
        (6, 1, 1908, 0, 0, 145000),
        (7, 1, 1909, 0, 0, 145000),
        (8, 1, 1910, 0, 0, 145000),
        (9, 1, 1281, 0, 0, 145000),
        (10, 0, 36753, 0, 0, 135000),
        (11, 0, 31465, 0, 0, 75000),
        (12, 0, 32753, 0, 0, 80000),
        (13, 0, 32756, 0, 0, 80000),
        (14, 0, 36748, 0, 0, 80000),
        (15, 0, 36749, 0, 0, 50000),
        (16, 0, 32752, 0, 0, 15000),
        (17, 0, 32755, 0, 0, 15000),
        (18, 0, 32754, 0, 0, 3000),
        (19, 0, 32751, 0, 0, 3000),
        (20, 0, 32918, 0, 0, 37500),
        (21, 0, 32943, 0, 0, 37500),
        (22, 3, 32795, 32796, 0, 35000),
        (23, 4, 0, 0, 0, 100),
    ]
    .into_iter()
    .map(|(index, kind, primary, secondary, addon, price)| ShopOffer {
        index,
        kind,
        primary,
        secondary,
        addon,
        price,
        bought: false,
    })
        .collect()
}

fn fresh_weekly() -> Weekly {
    Weekly {
        started: false,
        difficulty: 0,
        expanded: false,
        multiplier: 1,
        kill_experience: 0,
        delivery_experience: 0,
        pending_points: 0,
        pending_soulseals: 0,
        kills: Vec::new(),
        deliveries: Vec::new(),
    }
}

fn fresh_bounty() -> Bounty {
    Bounty {
        elite: false,
        state: 0,
        difficulty: 0,
        selected: 0,
        current: 0,
        rerolls: 0,
        daily_reroll_at: 0,
        offers: Vec::new(),
        active: None,
        lists: (0..5).map(|i| ListSlot {
            unlocked: i == 0,
            preferred: String::new(),
            unwanted: String::new(),
        }).collect(),
    }
}

const FRESH_TALISMAN: [TalismanPath; 4] = [TalismanPath {
    level: 0,
    bonus: 0,
    next_cost: 5,
}; 4];

fn verify_fresh_query(reply: &Reply, period: u64, shop: &[ShopOffer]) -> TestResult {
    if (
        reply.refusal,
        reply.period,
        reply.points,
        reply.bounty_points,
        reply.soulseals,
    ) != (0, period, 0, 0, 0)
        || reply.weekly != fresh_weekly()
        || reply.weekly_results.is_some()
        || reply.bounty != fresh_bounty()
        || reply.talisman != FRESH_TALISMAN
        || reply.shop != shop
    {
        return Err(format!(
            "unexpected fresh Task Board: expected period={period}; observed={reply:?}"
        )
        .into());
    }
    Ok(())
}

fn verify_start(before: &Reply, reply: &Reply, level: u32) -> TestResult {
    let mut baseline = reply.clone();
    baseline.weekly = before.weekly.clone();
    baseline.refusal = before.refusal;
    let week = &reply.weekly;
    let success = match level {
        399 => reply.refusal == 3 && week == &before.weekly,
        400 => {
            let Some((any, kills)) = week.kills.split_last() else {
                return Err("Master start omitted its kill assignments".into());
            };
            reply.refusal == 0
                && week.started
                && week.difficulty == 3
                && !week.expanded
                && week.multiplier == 1
                && (week.kill_experience, week.delivery_experience) == (797_603, 797_603)
                && (week.pending_points, week.pending_soulseals) == (0, 0)
                && (any.race.as_str(), any.required, any.current) == ("", 4000, 0)
                && kills.len() == 5
                && kills.iter().all(|task| !task.race.is_empty() && (300..=600).contains(&task.required) && task.current == 0)
                && kills.iter().map(|task| &task.race).collect::<std::collections::BTreeSet<_>>().len() == 5
                && week.deliveries.len() == 6
                && week.deliveries.iter().all(|task| task.item != 0 && task.required > 0 && !task.delivered)
                && week.deliveries.iter().map(|task| task.item).collect::<std::collections::BTreeSet<_>>().len() == 6
        }
        _ => false,
    };
    if !success || baseline != *before {
        return Err(format!("P04 live Master start at level {level}: {reply:?}").into());
    }
    Ok(())
}

type Release = (i32, bool, bool, Option<i64>, bool, bool);

const ACTIVE: Release = (2, true, true, None, true, true);

async fn release_snapshot(
    database: &mut sqlx::PgConnection,
    session: &[u8; 16],
    receipt_key: &[u8],
) -> TestResult<Release> {
    Ok(sqlx::query_as::<_, Release>(
        "SELECT s.session_state::int, s.current_transport_ref IS NOT NULL,
             s.control_loss_epoch IS NULL AND s.original_grace_deadline IS NULL,
             (SELECT (extract(epoch FROM pg_xact_commit_timestamp(r.xmin)) * 1000000)::bigint
              FROM game_durability_admission_lifecycle_receipts r WHERE r.operation_key=$2),
             EXISTS(SELECT 1 FROM game_durability_admission_character_guards c
                 WHERE c.character_id=s.character_id AND c.holder_game_session_id=s.game_session_id),
             EXISTS(SELECT 1 FROM game_durability_admission_account_guards a
                 WHERE a.account_id=s.account_id AND a.presence_character_id=s.character_id)
         FROM game_durability_reconnect_sessions s WHERE s.game_session_id=$1::uuid",
    )
    .bind(uuid_text(session))
    .bind(receipt_key)
    .fetch_one(database)
    .await?)
}

async fn logout_released(
    client: &mut DevClientSession,
    database: &mut sqlx::PgConnection,
) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let session = client.join_snapshot().game_session_id;
    let mut receipt_key = b"claim-lifecycle-v1".to_vec();
    receipt_key.extend_from_slice(session.as_bytes());
    receipt_key.push(1);
    let before = release_snapshot(database, session.as_bytes(), &receipt_key).await?;
    if before != ACTIVE {
        return Err(format!("P04 logout did not start with an active lease: {before:?}").into());
    }
    if client.session_mut().logout().await?.result != Some(LogoutResult::Accepted) {
        return Err("P04 first logout was refused".into());
    }
    let acknowledged_at = i64::try_from(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_micros()
    )?;
    // Read once before dropping the transport or issuing a grant. The committing
    // transaction also cleared both guards. Its time must precede wire acceptance.
    let after = release_snapshot(database, session.as_bytes(), &receipt_key).await?;
    verify_release(after, acknowledged_at)
}

fn verify_release(after: Release, acknowledged_at: i64) -> TestResult {
    if !matches!(after, (3, false, true, Some(committed), false, false) if committed <= acknowledged_at) {
        return Err(format!("P04 accepted logout without committed orderly release on first observation: {after:?}").into());
    }
    Ok(())
}

struct Boundary {
    level: u32,
    account: &'static str,
    profile: &'static str,
    files: [&'static str; 3],
}

async fn verify_boundary(
    scope: &SeamClients<'_>,
    database: &mut sqlx::PgConnection,
    period: u64,
    shop: &[ShopOffer],
    case: Boundary,
) -> TestResult<String> {
    let identity = character_prep::dedicated_character(scope, case.account).await?;
    let clients = identity.clients(scope);
    let mut client = character_prep::Seeded {
        profile: case.profile,
        reason: "Global Master eligibility at level 400",
    }.connect(&clients, 0x26, 0).await?;
    let (hp, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer", case.level, oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    ).ok_or("P04 fixture maxima")?;
    if full_e2ef7::player_level(&mut client).await? != case.level
        || !client.actor_vitals().is_some_and(|v| (v.max_health, v.max_mana) == (hp, mana))
    {
        return Err(format!("P04 fixture did not admit at level {} with derived vitals", case.level).into());
    }
    let session = client.join_snapshot().game_session_id;
    let (initial, query) = send(&mut client, 4, 0, case.files[0]).await?;
    verify_fresh_query(&initial, period, shop)?;
    let (start, frame) = send(&mut client, 5, 3, case.files[1]).await?;
    verify_start(&initial, &start, case.level)?;
    logout_released(&mut client, database).await?;
    drop(client);
    let mut fresh = connect_character_once(&clients, 0x27, 0).await?;
    let readmission = fresh.join_snapshot().game_session_id;
    let (after, after_frame) = send(&mut fresh, 4, 0, case.files[2]).await?;
    let mut expected_after = start;
    expected_after.refusal = 0;
    if readmission == session || after != expected_after {
        return Err(format!("P04 Master board at level {} changed after relog", case.level).into());
    }
    logout_released(&mut fresh, database).await?;
    Ok(format!("{},{},{},{},{},{},{},{}",
        uuid_text(session.as_bytes()), uuid_text(readmission.as_bytes()),
        query.command_id, query.server_sequence, frame.command_id, frame.server_sequence,
        after_frame.command_id, after_frame.server_sequence))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    // Global Weekly Tasks: https://www.tibiawiki.com.br/wiki/Weekly_Tasks
    character_prep::declare_seeded("P04.client", "sorcerer-level-399,400", "Global Master boundary");
    let period = expected_period(clients).await?;
    let shop = expected_shop();
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let below = verify_boundary(clients, &mut database, period, &shop, Boundary {
        level: 399, account: "NODE_BOOT_Q_P04_399_ACCOUNT", profile: "p04-master-399",
        files: ["p04-query.json", "p04-refused.json", "p04-readmission.json"],
    }).await?;
    let at = verify_boundary(clients, &mut database, period, &shop, Boundary {
        level: 400, account: "NODE_BOOT_Q_P04_400_ACCOUNT", profile: "p04-master-400",
        files: ["p04-master-query.json", "p04-master-start.json", "p04-master-readmission.json"],
    }).await?;
    let expected = format!(
        "period={period}\npoints=0\nbounty_points=0\nsoulseals=0\nstarted=false\nkills=0\ndeliveries=0\nshop_count=24\nrefusal=3\n"
    );
    capture("p04-expected.txt", expected.as_bytes())?;
    Ok(format!("below={below};at={at};{}", expected.replace('\n', ";")))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P04.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P04.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P04.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use task_board::task_board::full::{DeliveryTask, KillTask};

    #[test]
    fn p04_writer_preserves_the_five_original_frame_fields() -> TestResult {
        let session = v7(4, 2);
        let raw = GatedCommandResult {
            command_id: 11,
            status: CommandStatus::Rejected,
            server_sequence: 31,
            payload: vec![0, 255, 2],
        };
        let envelope: serde_json::Value = serde_json::from_slice(&frame_envelope(&session, &raw)?)?;
        assert_eq!(
            envelope,
            serde_json::json!({
                "command_id": 11,
                "game_session_id": uuid_text(&session),
                "payload": [0, 255, 2],
                "server_sequence": 31,
                "status": 2,
            })
        );
        Ok(())
    }

    #[test]
    fn p04_logout_rejects_active_loss_and_uncleared_guards() -> TestResult {
        verify_release((3, false, true, Some(10), false, false), 11)?;
        for wrong in [
            ACTIVE,
            (1, false, false, None, true, true),
            (3, false, false, Some(10), false, false),
            (3, false, true, None, false, false),
            (3, false, true, Some(10), true, false),
            (3, false, true, Some(10), false, true),
            (3, false, true, Some(12), false, false),
        ] {
            assert!(verify_release(wrong, 11).is_err());
        }
        Ok(())
    }

    #[test]
    fn p04_fresh_expectations_reject_self_consistent_wrong_replies() -> TestResult {
        let shop = expected_shop();
        let initial = Reply {
            refusal: 0,
            period: 20780,
            points: 0,
            bounty_points: 0,
            soulseals: 0,
            weekly: fresh_weekly(),
            weekly_results: None,
            bounty: fresh_bounty(),
            talisman: FRESH_TALISMAN,
            shop: shop.clone(),
        };
        verify_fresh_query(&initial, 20780, &shop)?;
        let mut refused = initial.clone();
        refused.refusal = 3;
        verify_start(&initial, &refused, 399)?;
        assert!(verify_start(&initial, &refused, 400).is_err());
        assert!(verify_start(&initial, &initial, 400).is_err());
        for mutate in [
            (|r: &mut Reply| r.period += 7) as fn(&mut Reply),
            |r| r.points += 1,
            |r| r.bounty_points += 1,
            |r| r.soulseals += 1,
            |r| r.weekly.expanded = true,
            |r| r.bounty.state = 2,
            |r| r.bounty.lists[1].unlocked = true,
            |r| r.talisman[1].level = 1,
            |r| {
                r.weekly_results = Some(oteryn_protocol_oteryn::task_board::full::WeeklyResults {
                    period: 20773,
                    points: 1,
                    soulseals: 1,
                })
            },
            |r| {
                r.weekly.kills.push(KillTask {
                    race: "oteryn:creature.rat".into(),
                    required: 1,
                    current: 0,
                })
            },
            |r| {
                r.weekly.deliveries.push(DeliveryTask {
                    item: 1,
                    required: 1,
                    current: 0,
                    delivered: false,
                })
            },
            |r| r.shop[0].kind = 1,
            |r| {
                r.shop.pop();
            },
            |r| r.shop[0].price -= 1,
        ] {
            let mut reply = initial.clone();
            mutate(&mut reply);
            assert!(verify_fresh_query(&reply, 20780, &shop).is_err());
        }
        Ok(())
    }
}
