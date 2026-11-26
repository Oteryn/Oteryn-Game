//! P04 client replay capture: real TASK_BOARD replies for the Query and a refused StartWeekly with
//! a difficulty above the character’s level. The consumer is `apply_full_result` and `panel_text` in
//! `crates/client-ui/src/features/0072-task-board`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;
use oteryn_dev_client::client_runtime::task_board;
use oteryn_gameplay_quests::quest::task_board::full::{SHOP_OFFERS, TaskBoardShopKind};
use task_board::task_board::full::{
    Reply, Request, ShopOffer, Weekly, decode_reply, encode_request,
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
    SHOP_OFFERS
        .iter()
        .map(|item| ShopOffer {
            index: item.id,
            kind: match item.kind {
                TaskBoardShopKind::Item => 0,
                TaskBoardShopKind::Mount => 1,
                TaskBoardShopKind::Outfit => 2,
                TaskBoardShopKind::ItemDouble => 3,
                TaskBoardShopKind::Promotion => 4,
            },
            price: item.points,
            primary: u32::from(item.appearance_or_item),
            secondary: u32::from(item.second_item.unwrap_or(0)),
            addon: item.addon,
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

fn verify_fresh_query(reply: &Reply, period: u64, shop: &[ShopOffer]) -> TestResult {
    if (
        reply.refusal,
        reply.period,
        reply.points,
        reply.bounty_points,
        reply.soulseals,
    ) != (0, period, 0, 0, 0)
        || reply.weekly != fresh_weekly()
        || reply.shop != shop
    {
        return Err(format!(
            "unexpected fresh Task Board: expected period={period}; observed={reply:?}"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let period = expected_period(clients).await?;
    let shop = expected_shop();
    let mut client = connect_world(clients, 0x26).await?;
    let (initial, query_frame) = send(&mut client, 4, 0, "p04-query.json").await?;
    verify_fresh_query(&initial, period, &shop)?;
    let (refused, refused_frame) = send(&mut client, 5, 3, "p04-refused.json").await?;
    let mut unchanged = refused.clone();
    unchanged.refusal = 0;
    if refused.refusal != 3 || unchanged != initial {
        return Err(format!("StartWeekly(3) was not refused unchanged: {refused:?}").into());
    }
    let offer = shop.first().ok_or("authored shop offer")?;
    let expected = format!(
        "period={period}\npoints=0\nbounty_points=0\nsoulseals=0\nstarted=false\nkills=0\ndeliveries=0\nshop_count={}\nshop_first={},{},{}\nrefusal=3\n",
        shop.len(),
        offer.index,
        offer.kind,
        offer.price
    );
    capture("p04-expected.txt", expected.as_bytes())?;
    let admitted = client.join_snapshot().game_session_id;
    let session = uuid_text(admitted.as_bytes());
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P04 client capture logout was refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x26).await?;
    let readmission = fresh.join_snapshot().game_session_id;
    let (after, after_frame) = send(&mut fresh, 4, 0, "p04-readmission.json").await?;
    if readmission == admitted || after != initial {
        return Err("P04 client refusal changed the board after readmission".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P04 client verification logout was refused".into());
    }
    Ok(format!(
        "session={session};readmission={};query={},{};refused={},{};after={},{};{}",
        uuid_text(readmission.as_bytes()),
        query_frame.command_id,
        query_frame.server_sequence,
        refused_frame.command_id,
        refused_frame.server_sequence,
        after_frame.command_id,
        after_frame.server_sequence,
        expected.replace('\n', ";"),
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
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
    use task_board::task_board::full::{Bounty, DeliveryTask, KillTask, TalismanPath};

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
            bounty: Bounty {
                elite: false,
                state: 0,
                difficulty: 0,
                selected: 0,
                current: 0,
                rerolls: 0,
                daily_reroll_at: 0,
                offers: Vec::new(),
                active: None,
                lists: Vec::new(),
            },
            talisman: [TalismanPath::default(); 4],
            shop: shop.clone(),
        };
        verify_fresh_query(&initial, 20780, &shop)?;
        for mutate in [
            (|r: &mut Reply| r.period += 7) as fn(&mut Reply),
            |r| r.points += 1,
            |r| r.bounty_points += 1,
            |r| r.soulseals += 1,
            |r| r.weekly.expanded = true,
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
