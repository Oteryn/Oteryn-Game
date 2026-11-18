//! P05 client replay capture: real TASK_BOARD replies for the Query, a Talisman purchase and the
//! repeated purchase that the server refuses. The consumer is `talisman::request`,
//! `apply_full_result` and `panel_text` in `crates/client-ui/src/features/0072-task-board`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::task_board;
use task_board::task_board::full::{Reply, Request, decode_reply, encode_request};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P05_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn send(client: &mut DevClientSession, action: u8, name: &str) -> TestResult<(u64, Reply)> {
    let payload = encode_request(&Request {
        action,
        selection: 0,
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
    capture(name, &result.payload);
    let outcome =
        decode_reply(&result.payload).map_err(|error| format!("reply decode: {error:?}"))?;
    Ok((result.command_id, outcome))
}

fn talisman_minted(before: u64, after: u64, refusal: u8) -> bool {
    refusal == 0 && before.checked_add(1) == Some(after)
}

fn repeat_unchanged(refusal: u8, same_gold: bool, same_items: bool, same_levels: bool) -> bool {
    refusal != 0 && same_gold && same_items && same_levels
}

#[test]
fn talisman_purchase_requires_grant_and_repeat_preserves_custody() {
    assert!(!talisman_minted(1, 1, 0));
    assert!(talisman_minted(1, 2, 0));
    assert!(!repeat_unchanged(1, false, true, true));
    assert!(!repeat_unchanged(1, true, false, true));
    assert!(repeat_unchanged(1, true, true, true));
}

fn talisman_count(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("talisman inventory absent")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == reference)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

pub(super) async fn gold(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($3,'hex')::uuid AND i.lifecycle=1),0) + COALESCE((SELECT balance FROM game_account_bank_balances WHERE account_id=$2::uuid AND world_id=encode($3,'hex')::uuid),0)")
        .bind(clients.characters[0].as_slice()).bind(&clients.accounts[0]).bind(clients.world.as_slice()).fetch_one(connection).await?)
}

fn second_purchase_matches(
    first_id: u64,
    second_id: u64,
    gold: (i64, i64),
    items: (u64, u64),
    refusal: u8,
) -> bool {
    first_id.checked_add(1) == Some(second_id)
        && if gold.0 >= 5_000 {
            gold.0.checked_sub(5_000) == Some(gold.1) && talisman_minted(items.0, items.1, refusal)
        } else {
            matches!(refusal, 4 | 14)
                && repeat_unchanged(refusal, gold.0 == gold.1, items.0 == items.1, true)
        }
}

#[test]
fn fresh_funded_purchase_is_not_a_replay() {
    assert!(second_purchase_matches(8, 9, (6000, 1000), (1, 2), 0));
    assert!(!second_purchase_matches(8, 8, (6000, 1000), (1, 2), 0));
    assert!(!second_purchase_matches(8, 9, (6000, 6000), (1, 1), 1));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x27, 0).await?;
    let (_, before) = send(&mut client, 4, "p05-query.bin").await?;
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let reference = provisioning::item_reference(&provisioning::item_index()?, 51_978)?;
    let items_before = talisman_count(&client, reference)?;
    let gold_before = gold(&mut connection, clients).await?;
    let (first_id, bought) = send(&mut client, 17, "p05-buy.bin").await?;
    if bought.refusal != 0 {
        return Err(format!("Talisman purchase refused: {bought:?}").into());
    }
    if bought.bounty_points != before.bounty_points || bought.talisman != before.talisman {
        return Err(format!(
            "Talisman purchase altered the upgrade projection: {before:?} -> {bought:?}"
        )
        .into());
    }
    let items_after = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let count = talisman_count(&client, reference)?;
            if talisman_minted(items_before, count, bought.refusal) {
                return Ok::<_, Box<dyn std::error::Error>>(count);
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
    })
    .await??;
    let gold_after = gold(&mut connection, clients).await?;
    if gold_before.checked_sub(5_000) != Some(gold_after) {
        return Err(format!(
            "talisman purchase gold={gold_before}->{gold_after}; expected debit 5000"
        )
        .into());
    }
    let (second_id, repeated) = send(&mut client, 17, "p05-repeat.bin").await?;
    let second_gold = gold(&mut connection, clients).await?;
    let second_items = if gold_after >= 5_000 {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let count = talisman_count(&client, reference)?;
                if count != items_after {
                    return Ok::<_, Box<dyn std::error::Error>>(count);
                }
                client.service_liveness(Duration::from_millis(50)).await?;
                client.take_events();
            }
        })
        .await??
    } else {
        talisman_count(&client, reference)?
    };
    if !second_purchase_matches(
        first_id,
        second_id,
        (gold_after, second_gold),
        (items_after, second_items),
        repeated.refusal,
    ) || repeated.talisman != bought.talisman
        || repeated.bounty_points != bought.bounty_points
    {
        return Err(format!("second purchase id={second_id} gold={gold_after}->{second_gold} items={items_after}->{second_items}: {repeated:?}").into());
    }
    let level = |reply: &Reply| {
        reply
            .talisman
            .iter()
            .map(|p| format!("{},{},{}", p.level, p.bonus, p.next_cost))
            .collect::<Vec<_>>()
            .join("|")
    };
    let expected = format!(
        "bounty_points_before={}\nbounty_points_after={}\ntalisman_before={}\ntalisman_after={}\nrefusal={}\n",
        before.bounty_points,
        bought.bounty_points,
        level(&before),
        level(&bought),
        repeated.refusal
    );
    capture("p05-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P05.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P05.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P05.client status=FAIL evidence=timeout"),
    }
}
