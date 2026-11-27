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

async fn global_period(database: &mut sqlx::PgConnection) -> TestResult<u64> {
    // Global Weekly Tasks reset on Monday at 10:00 local server time.
    let period: i64 = sqlx::query_scalar(
        "SELECT (date_trunc('week', (clock_timestamp()
                    AT TIME ZONE 'Europe/Berlin') - interval '10 hours')::date
                    - DATE '1970-01-01')::bigint",
    )
    .fetch_one(database)
    .await?;
    Ok(u64::try_from(period)?)
}

async fn expected_period(database: &mut sqlx::PgConnection) -> TestResult<u64> {
    use sqlx::Connection as _;

    // Each historical probe uses a private World publication rolled back before live play.
    const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334499";
    for (instant, expected) in [
        ("2026-11-23 08:59:59+00", 20773),
        ("2026-11-23 09:00:00+00", 20780),
        ("2026-07-20 07:59:59+00", 20647),
        ("2026-07-20 08:00:00+00", 20654),
    ] {
        let mut tx = database.begin().await?;
        sqlx::query(
            "INSERT INTO game_world_reset_epochs
              (world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,
               frame_digest,calendar,calendar_revision,activated_at,reset_month)
             VALUES($1::uuid,0,$2,'map-1',$2,$2,$2,
               'TIBIA_SERVER_SAVE_BERLIN_06',1,$3::text::timestamptz,'2026-11')",
        )
        .bind(WORLD)
        .bind([9_u8; 32].as_slice())
        .bind(instant)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES($1::uuid,0)")
            .bind(WORLD)
            .execute(&mut *tx)
            .await?;
        let actual: i64 = sqlx::query_scalar("SELECT game_task_board_period($1::uuid)")
            .bind(WORLD)
            .fetch_one(&mut *tx)
            .await?;
        tx.rollback().await?;
        if actual != expected {
            return Err(format!("P04 production period at {instant}: expected {expected}, got {actual}").into());
        }
        evidence(&format!("P04_PRODUCTION_PERIOD instant={instant:?} period={actual}"));
    }
    global_period(database).await
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
    .map(
        |(index, kind, primary, secondary, addon, price)| ShopOffer {
            index,
            kind,
            primary,
            secondary,
            addon,
            price,
            bought: false,
        },
    )
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
        lists: (0..5)
            .map(|i| ListSlot {
                unlocked: i == 0,
                preferred: String::new(),
                unwanted: String::new(),
            })
            .collect(),
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
                && kills.iter().all(|task| {
                    !task.race.is_empty()
                        && (300..=600).contains(&task.required)
                        && task.current == 0
                })
                && kills
                    .iter()
                    .map(|task| &task.race)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == 5
                && week.deliveries.len() == 6
                && week
                    .deliveries
                    .iter()
                    .all(|task| task.item != 0 && task.required > 0 && !task.delivered)
                && week
                    .deliveries
                    .iter()
                    .map(|task| task.item)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == 6
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
    let ack = client.session_mut().logout().await?;
    if ack.result != Some(LogoutResult::Accepted) {
        return Err(format!("P04 first logout was refused: {ack:?}").into());
    }
    let log = std::fs::read_to_string(required("NODE_BOOT_LOGOUT_ACK_LOG")?)?;
    let acknowledged_at = logout_ack_write_time(
        &log,
        &uuid_text(session.as_bytes()),
        ack.command_id,
        ack.result_server_sequence,
    )?;
    let after = release_snapshot(database, session.as_bytes(), &receipt_key).await?;
    verify_release(after, acknowledged_at)
}

fn logout_ack_write_time(
    log: &str,
    session_id: &str,
    command_id: u64,
    sequence: u64,
) -> TestResult<i64> {
    let binding = format!(
        "event=qualification_logout_ack_write session_id={session_id} command_id={command_id} server_sequence={sequence} unix_us="
    );
    let mut matches = log.lines().filter_map(|line| {
        // The structured logger may insert a connection trace after the event name.
        let (_, fields) = line.split_once(" event=qualification_logout_ack_write ")?;
        let fields = fields
            .strip_prefix("trace=")
            .and_then(|rest| rest.split_once(' '))
            .map_or(fields, |(_, fields)| fields);
        let expected = binding.strip_prefix("event=qualification_logout_ack_write ")?;
        fields.strip_prefix(expected)?.split_once(' ')
    });
    let (at, proof) = matches
        .next()
        .ok_or("P04 missing bound server logout acknowledgement write marker")?;
    if matches.next().is_some() {
        return Err("P04 duplicate server logout acknowledgement write marker".into());
    }
    if proof.trim() != "commit_visible=true" {
        return Err("P04 logout commit was not visible before the acknowledgement write".into());
    }
    Ok(at.parse()?)
}

fn verify_release(after: Release, acknowledged_at: i64) -> TestResult {
    if !matches!(after, (3, false, true, Some(committed), false, false) if committed <= acknowledged_at)
    {
        return Err(format!(
            "P04 accepted logout without committed orderly release on first observation: {after:?}"
        )
        .into());
    }
    Ok(())
}

struct MasterCatalogue {
    races: std::collections::BTreeSet<String>,
    amounts: std::collections::BTreeMap<u32, (u32, u32)>,
}

impl MasterCatalogue {
    fn load() -> TestResult<Self> {
        let races = include_str!("p04_master_creature_oracle.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(str::to_owned)
            .collect();
        let amounts = delivery_oracle()?;
        Ok(Self { races, amounts })
    }

    fn verify(
        &self,
        week: &Weekly,
        available: &std::collections::BTreeMap<u32, u32>,
    ) -> TestResult {
        for kill in week.kills.iter().filter(|kill| !kill.race.is_empty()) {
            if !self.races.contains(&kill.race) {
                return Err(format!("P04 Master ineligible creature assignment: {kill:?}").into());
            }
        }
        for delivery in &week.deliveries {
            let expected = self.amounts.get(&delivery.item);
            let current = available.get(&delivery.item).copied().unwrap_or(0);
            if !expected.is_some_and(|&(min, max)| (min..=max).contains(&delivery.required))
                || delivery.current != current
            {
                return Err(format!("P04 Master delivery outside authored eligibility: {delivery:?}; expected amount={expected:?} carried={current}").into());
            }
        }
        Ok(())
    }
}

type DeliveryPool = std::collections::BTreeMap<u32, (u32, u32)>;

fn delivery_oracle() -> TestResult<DeliveryPool> {
    include_str!("p04_master_delivery_oracle.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| -> TestResult<_> {
            let mut fields = line.split(',');
            let mut number = || -> TestResult<u32> {
                Ok(fields
                    .next()
                    .ok_or("incomplete P04 delivery oracle")?
                    .parse()?)
            };
            Ok((number()?, (number()?, number()?)))
        })
        .collect()
}

fn production_delivery_pool() -> TestResult<DeliveryPool> {
    let mut pool = DeliveryPool::new();
    for item in oteryn_gameplay_quests::quest::task_board::full::DELIVERY_ITEMS {
        insert_delivery_range(
            &mut pool,
            u32::from(item.item_id),
            (*item.quantity.start(), *item.quantity.end()),
        )?;
    }
    Ok(pool)
}

fn insert_delivery_range(pool: &mut DeliveryPool, id: u32, range: (u32, u32)) -> TestResult {
    if pool.insert(id, range).is_some() {
        return Err(format!("P04 duplicate delivery pool entry: {id}").into());
    }
    Ok(())
}

fn verify_delivery_pool(expected: &DeliveryPool, actual: &DeliveryPool) -> TestResult {
    let missing_or_wrong: Vec<_> = expected
        .iter()
        .filter(|(id, range)| actual.get(id) != Some(*range))
        .collect();
    let extra: Vec<_> = actual
        .keys()
        .filter(|id| !expected.contains_key(id))
        .collect();
    if !missing_or_wrong.is_empty() || !extra.is_empty() {
        return Err(format!("P04 Master delivery pool mismatch missing_or_wrong={missing_or_wrong:?} extra={extra:?}").into());
    }
    Ok(())
}

type Custody = Vec<(String, Option<String>, String, String, String, i64, i16)>;

async fn durable_carried(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<Custody> {
    let rows = sqlx::query_as(
        "WITH RECURSIVE tree(item, parent, slot, depth) AS (
             SELECT item_instance_id, NULL::uuid, '0'::text, 0
               FROM game_item_container_slots WHERE character_id=$1::uuid
             UNION ALL
             SELECT e.item_instance_id, e.parent_item_instance_id, e.placement_ordinal::text, t.depth+1
               FROM tree t JOIN (
                 SELECT item_instance_id,parent_item_instance_id,placement_ordinal
                   FROM game_item_container_entries WHERE character_id=$1::uuid
                 UNION ALL
                 SELECT item_instance_id,parent_item_instance_id,placement_ordinal FROM game_item_nested_entries
               ) e ON e.parent_item_instance_id=t.item WHERE t.depth < 9
         ), carried AS (
             SELECT * FROM tree
             UNION ALL
             SELECT item_instance_id,NULL::uuid,slot::text,0
               FROM game_character_equipment_slots WHERE character_id=$1::uuid
         )
         SELECT c.item::text,c.parent::text,c.slot,i.definition_production_key,
                i.definition_revision_ref,i.quantity::bigint,i.lifecycle::smallint
           FROM carried c JOIN game_item_instances i ON i.item_instance_id=c.item
          ORDER BY c.item,c.parent,c.slot LIMIT 510",
    )
    .bind(uuid_text(character))
    .fetch_all(database)
    .await?;
    if rows.is_empty() || rows.len() >= 510 {
        return Err("P04 durable starter inventory is absent or exceeded the read bound".into());
    }
    Ok(rows)
}

fn verify_durable_carried(before: &Custody, after: &Custody) -> TestResult {
    if before != after {
        return Err("P04 Master start changed durable carried inventory".into());
    }
    Ok(())
}

async fn capture_master_assignments(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
    session: &[u8; 16],
    command_id: u64,
    available: &std::collections::BTreeMap<u32, u32>,
    week: &Weekly,
) -> TestResult {
    let persisted: serde_json::Value = sqlx::query_scalar(
        "SELECT body->'weekly' FROM game_character_task_boards
         WHERE character_id=$1::uuid AND cause_id=$2::uuid AND cause_ordinal=$3::text::numeric",
    )
    .bind(uuid_text(character))
    .bind(uuid_text(session))
    .bind(command_id.to_string())
    .fetch_one(database)
    .await?;
    let persisted: oteryn_gameplay_quests::quest::task_board::full::WeeklyBoard =
        serde_json::from_value(persisted)?;
    let mut kills = persisted
        .kills
        .iter()
        .map(|task| (&*task.race, task.required, task.current))
        .collect::<Vec<_>>();
    kills.push(("", persisted.any_required, persisted.any_kills));
    let deliveries = persisted
        .deliveries
        .iter()
        .map(|task| {
            let item = u32::from(task.item_id);
            (
                item,
                task.required,
                available.get(&item).copied().unwrap_or(0),
                task.delivered,
            )
        })
        .collect::<Vec<_>>();
    let expected = serde_json::json!([kills, deliveries]);
    let actual = serde_json::json!([
        week.kills
            .iter()
            .map(|task| (&task.race, task.required, task.current))
            .collect::<Vec<_>>(),
        week.deliveries
            .iter()
            .map(|task| (task.item, task.required, task.current, task.delivered))
            .collect::<Vec<_>>(),
    ]);
    if actual != expected {
        return Err(format!(
            "P04 wire assignments differ from the committed board: {actual} vs {expected}"
        )
        .into());
    }
    capture(
        "p04-master-assignments.json",
        &serde_json::to_vec(&expected)?,
    )
}

async fn durable_experience(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state WHERE character_id=$1::uuid",
    )
    .bind(uuid_text(character))
    .fetch_one(database)
    .await?)
}

fn verify_experience(before: i64, after: i64) -> TestResult {
    if before != after {
        return Err(
            format!("P04 StartWeekly changed actual experience from {before} to {after}").into(),
        );
    }
    Ok(())
}

fn durable_delivery_amounts(custody: &Custody) -> TestResult<std::collections::BTreeMap<u32, u32>> {
    let mut amounts = std::collections::BTreeMap::<u32, u32>::new();
    for (_, _, _, key, _, quantity, _) in custody {
        if let Some(id) = key.strip_prefix("oteryn:item.tibia.i") {
            let total = amounts.entry(id.parse()?).or_default();
            *total = total
                .checked_add(u32::try_from(*quantity)?)
                .ok_or("P04 durable inventory quantity overflow")?;
        }
    }
    Ok(amounts)
}

fn seeded_carried(client: &DevClientSession) -> TestResult<std::collections::BTreeMap<u32, u32>> {
    let inventory = client.inventory().ok_or("P04 seed inventory absent")?;
    let index = provisioning::item_index()?;
    let mut amounts = std::collections::BTreeMap::<u32, u32>::new();
    // Fresh accounts enter with a starter backpack and no prior stash, depot or nested bags.
    for item in inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|slot| &slot.item))
    {
        let (key, _) = index
            .definition_at(item.item_definition_ref)
            .ok_or("P04 carried reference absent")?;
        if let Some(id) = key.strip_prefix("oteryn:item.tibia.i") {
            let total = amounts.entry(id.parse()?).or_default();
            *total = total
                .checked_add(item.count.get())
                .ok_or("P04 inventory quantity overflow")?;
        }
    }
    Ok(amounts)
}

fn verify_delivery_availability(before: &Reply, after: &Reply, item: u32) -> TestResult {
    let mut expected = before.clone();
    let assignment = expected.weekly.deliveries.iter_mut()
        .find(|assignment| assignment.item == item && assignment.current == 0)
        .ok_or("P04 availability fixture requires a zero-count assigned item")?;
    assignment.current = 1;
    if *after != expected {
        return Err(format!("P04 availability query did not expose the one carried item {item}: {:?}", after.weekly.deliveries).into());
    }
    Ok(())
}

async fn observe_positive_delivery(
    client: &mut DevClientSession,
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
    board: &Reply,
) -> TestResult<String> {
    let item = board.weekly.deliveries.iter()
        .find(|assignment| assignment.current == 0 && assignment.required > 1)
        .ok_or("P04 positive availability needs an unfulfilled delivery assignment")?.item;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        format!("item-{item}-1"),
    ).await?;
    let available = durable_delivery_amounts(&durable_carried(database, character).await?)?;
    if available.get(&item) != Some(&1) {
        return Err(format!("P04 availability fixture did not commit exactly one item {item}").into());
    }
    let (after, frame) = send(client, 4, 0, "p04-master-availability.json").await?;
    verify_delivery_availability(board, &after, item)?;
    evidence(&format!("P04_DELIVERY_AVAILABILITY item={item} before=0 after=1"));
    Ok(format!("{},{},{item}", frame.command_id, frame.server_sequence))
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
    let mut setup = connect_character_once(&clients, 0x26, 0).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(setup_session.as_bytes()),
        "--profile",
        case.profile.into(),
    )
    .await?;
    logout_released(&mut setup, database).await?;
    drop(setup);
    let mut client = connect_character_once(&clients, 0x26, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("P04 seed readmission reused the setup session".into());
    }
    let (hp, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        case.level,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("P04 fixture maxima")?;
    if full_e2ef7::player_level(&mut client).await? != case.level
        || !client
            .actor_vitals()
            .is_some_and(|v| (v.max_health, v.max_mana) == (hp, mana))
    {
        return Err(format!(
            "P04 fixture did not admit at level {} with derived vitals",
            case.level
        )
        .into());
    }
    let session = client.join_snapshot().game_session_id;
    let (initial, query) = send(&mut client, 4, 0, case.files[0]).await?;
    verify_fresh_query(&initial, period, shop)?;
    if client
        .selected_capabilities()
        .contains(&task_board::task_board::full::CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1)
    {
        return Err("P04 authored delivery oracle requires legacy Task Board identifiers".into());
    }
    let projected_before = seeded_carried(&client)?;
    let character = &clients.characters[0];
    let durable_before = durable_carried(database, character).await?;
    let available = durable_delivery_amounts(&durable_before)?;
    let experience_before = durable_experience(database, character).await?;
    let catalogue = (case.level == 400)
        .then(MasterCatalogue::load)
        .transpose()?;
    let (start, frame) = send(&mut client, 5, 3, case.files[1]).await?;
    verify_start(&initial, &start, case.level)?;
    if let Some(catalogue) = &catalogue {
        catalogue.verify(&start.weekly, &available)?;
        capture_master_assignments(
            database,
            character,
            session.as_bytes(),
            frame.command_id,
            &available,
            &start.weekly,
        )
        .await?;
    }
    if seeded_carried(&client)? != projected_before {
        return Err("P04 StartWeekly changed the delivery inventory".into());
    }
    verify_durable_carried(
        &durable_before,
        &durable_carried(database, character).await?,
    )?;
    verify_experience(
        experience_before,
        durable_experience(database, character).await?,
    )?;
    logout_released(&mut client, database).await?;
    drop(client);
    let mut fresh = connect_character_once(&clients, 0x27, 0).await?;
    let readmission = fresh.join_snapshot().game_session_id;
    let (after, after_frame) = send(&mut fresh, 4, 0, case.files[2]).await?;
    let start_refusal = start.refusal;
    let mut expected_after = start;
    expected_after.refusal = 0;
    if readmission == session || after != expected_after {
        return Err(format!(
            "P04 Master board at level {} changed after relog",
            case.level
        )
        .into());
    }
    verify_durable_carried(
        &durable_before,
        &durable_carried(database, character).await?,
    )?;
    verify_experience(
        experience_before,
        durable_experience(database, character).await?,
    )?;
    let availability = if let Some(catalogue) = catalogue {
        let proof = observe_positive_delivery(&mut fresh, database, character, &after).await?;
        verify_delivery_pool(&catalogue.amounts, &production_delivery_pool()?)?;
        format!(";availability={proof}")
    } else {
        String::new()
    };
    logout_released(&mut fresh, database).await?;
    evidence(&format!(
        "P04_DURABLE_ASSERTIONS level={} custody_entries={} total_experience={} refusal={}",
        case.level,
        durable_before.len(),
        experience_before,
        start_refusal,
    ));
    Ok(format!(
        "{},{},{},{},{},{},{},{}{availability}",
        uuid_text(session.as_bytes()),
        uuid_text(readmission.as_bytes()),
        query.command_id,
        query.server_sequence,
        frame.command_id,
        frame.server_sequence,
        after_frame.command_id,
        after_frame.server_sequence
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    // Global Weekly Tasks: https://www.tibiawiki.com.br/wiki/Weekly_Tasks
    character_prep::declare_seeded(
        "P04.client",
        "sorcerer-level-399,400",
        "Global Master boundary",
    );
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let period = expected_period(&mut database).await?;
    let shop = expected_shop();
    let below = verify_boundary(
        clients,
        &mut database,
        period,
        &shop,
        Boundary {
            level: 399,
            account: "NODE_BOOT_Q_P04_399_ACCOUNT",
            profile: "p04-master-399",
            files: ["p04-query.json", "p04-refused.json", "p04-readmission.json"],
        },
    )
    .await?;
    let at = verify_boundary(
        clients,
        &mut database,
        period,
        &shop,
        Boundary {
            level: 400,
            account: "NODE_BOOT_Q_P04_400_ACCOUNT",
            profile: "p04-master-400",
            files: [
                "p04-master-query.json",
                "p04-master-start.json",
                "p04-master-readmission.json",
            ],
        },
    )
    .await?;
    let expected = format!(
        "period={period}\npoints=0\nbounty_points=0\nsoulseals=0\nstarted=false\nkills=0\ndeliveries=0\nshop_count=24\nrefusal=3\n"
    );
    capture("p04-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "below={below};at={at};{}",
        expected.replace('\n', ";")
    ))
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
    fn p04_logout_proof_rejects_commit_after_server_write_and_wrong_binding() -> TestResult {
        let log = "oteryn-game-server event=qualification_logout_ack_write trace=trace session_id=session command_id=2 server_sequence=9 unix_us=100 commit_visible=true";
        let at = logout_ack_write_time(log, "session", 2, 9)?;
        let early = log.replace("commit_visible=true", "commit_visible=false");
        assert!(logout_ack_write_time(&early, "session", 2, 9).is_err());
        assert!(logout_ack_write_time(log, "session", 2, 8).is_err());
        assert!(logout_ack_write_time(log, "other", 2, 9).is_err());
        assert!(logout_ack_write_time(&format!("{log}\n{log}"), "session", 2, 9).is_err());
        verify_release((3, false, true, Some(99), false, false), at)?;
        assert!(verify_release((3, false, true, Some(101), false, false), at).is_err());
        Ok(())
    }

    #[test]
    fn p04_nonzero_availability_rejects_a_constant_zero_report() -> TestResult {
        let mut board = Reply {
            refusal: 0,
            period: 20780,
            points: 0,
            bounty_points: 0,
            soulseals: 0,
            weekly: fresh_weekly(),
            weekly_results: None,
            bounty: fresh_bounty(),
            talisman: FRESH_TALISMAN,
            shop: expected_shop(),
        };
        board.weekly.deliveries.push(DeliveryTask {
            item: 813,
            required: 10,
            current: 0,
            delivered: false,
        });
        assert!(verify_delivery_availability(&board, &board, 813).is_err());
        let mut changed = board.clone();
        changed.weekly.deliveries[0].current = 1;
        verify_delivery_availability(&board, &changed, 813)
    }

    #[test]
    fn p04_delivery_requires_items_omitted_by_the_projection() -> TestResult {
        let custody = vec![(
            "item".into(),
            Some("backpack".into()),
            "1".into(),
            "oteryn:item.tibia.i28568".into(),
            "definition-r1".into(),
            7,
            1,
        )];
        let available = durable_delivery_amounts(&custody)?;
        let catalogue = MasterCatalogue::load()?;
        let mut week = fresh_weekly();
        week.deliveries.push(DeliveryTask {
            item: 28568,
            required: 250,
            current: 0,
            delivered: false,
        });
        assert!(catalogue.verify(&week, &available).is_err());
        week.deliveries[0].current = 7;
        catalogue.verify(&week, &available)
    }

    #[test]
    fn p04_pool_rejects_the_audited_inkwell_and_basalt_omissions() -> TestResult {
        let oracle = delivery_oracle()?;
        assert_eq!(oracle.get(&28568), Some(&(250, 500)));
        assert_eq!(oracle.get(&43859), Some(&(5, 10)));
        assert!(!oracle.contains_key(&3509));
        verify_delivery_pool(&oracle, &oracle)?;
        let mut legacy = oracle.clone();
        legacy.remove(&28568);
        legacy.remove(&43859);
        legacy.insert(3509, (50, 50));
        let message = verify_delivery_pool(&oracle, &legacy)
            .unwrap_err()
            .to_string();
        for id in ["28568", "43859", "3509"] {
            assert!(message.contains(id), "{message}");
        }
        Ok(())
    }

    #[test]
    fn p04_durable_inventory_rejects_a_debit_hidden_by_an_unchanged_projection() -> TestResult {
        let before = vec![(
            "instance".into(),
            None,
            "1".into(),
            "item".into(),
            "r1".into(),
            2,
            1,
        )];
        verify_durable_carried(&before, &before)?;
        let mut debited = before.clone();
        debited[0].5 -= 1;
        assert!(verify_durable_carried(&before, &debited).is_err());
        verify_durable_carried(&before, &before[..0].to_vec()).unwrap_err();
        Ok(())
    }

    #[test]
    fn p04_rejects_duplicate_delivery_rows() -> TestResult {
        let mut pool = DeliveryPool::new();
        insert_delivery_range(&mut pool, 28568, (250, 500))?;
        assert!(insert_delivery_range(&mut pool, 28568, (250, 500)).is_err());
        Ok(())
    }

    #[test]
    fn p04_master_creature_oracle_rejects_a_medium_assignment() -> TestResult {
        let catalogue = MasterCatalogue::load()?;
        let mut week = fresh_weekly();
        week.kills.push(KillTask {
            race: "oteryn:creature.demon".into(),
            required: 300,
            current: 0,
        });
        let available = std::collections::BTreeMap::new();
        catalogue.verify(&week, &available)?;
        for ineligible in [
            "oteryn:creature.dragon",
            "oteryn:creature.moonspawn_juggernaut",
        ] {
            week.kills[0].race = ineligible.into();
            assert!(catalogue.verify(&week, &available).is_err());
        }
        Ok(())
    }

    #[test]
    fn p04_rejects_reordered_inventory_and_a_real_xp_award() -> TestResult {
        let before = vec![
            (
                "first".into(),
                Some("backpack".into()),
                "1".into(),
                "item".into(),
                "r1".into(),
                2,
                1,
            ),
            (
                "second".into(),
                Some("backpack".into()),
                "2".into(),
                "item".into(),
                "r1".into(),
                2,
                1,
            ),
        ];
        let mut reordered = before.clone();
        reordered[0].2 = "2".into();
        reordered[1].2 = "1".into();
        assert!(verify_durable_carried(&before, &reordered).is_err());
        verify_experience(1000, 1000)?;
        assert!(verify_experience(1000, 1001).is_err());
        Ok(())
    }

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
    fn p04_master_catalogue_rejects_unknown_creatures_wrong_quantities_and_progress() -> TestResult
    {
        let catalogue = MasterCatalogue {
            races: ["oteryn:creature.demon".into()].into_iter().collect(),
            amounts: [(813, (100, 200))].into_iter().collect(),
        };
        let mut week = fresh_weekly();
        week.kills.push(KillTask {
            race: "oteryn:creature.demon".into(),
            required: 300,
            current: 0,
        });
        week.deliveries.push(DeliveryTask {
            item: 813,
            required: 100,
            current: 0,
            delivered: false,
        });
        let available = std::collections::BTreeMap::new();
        catalogue.verify(&week, &available)?;
        for mutate in [
            (|week: &mut Weekly| week.kills[0].race = "oteryn:creature.rat".into())
                as fn(&mut Weekly),
            |week| week.deliveries[0].item = 999_999,
            |week| week.deliveries[0].required = 20,
            |week| week.deliveries[0].current = 1,
        ] {
            let mut reply = week.clone();
            mutate(&mut reply);
            assert!(catalogue.verify(&reply, &available).is_err());
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
