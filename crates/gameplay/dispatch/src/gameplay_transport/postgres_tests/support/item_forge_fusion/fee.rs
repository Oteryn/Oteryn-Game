//! The Forge cause composes real coin burns and the remaining native bank debit.

use super::*;

const FEE: i64 = 25_000;

const COINS: i64 = 10_000 + 7 * 100 + 13;

const BANK_PART: i64 = FEE - COINS;

pub(in super::super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_bank(h).await?;
    seed_items(
        h,
        &[
            (101, SWORD, 1, 0),
            (102, SWORD, 1, 0),
            (104, "oteryn:item.tibia.i3043", 1, 0),
            (105, "oteryn:item.tibia.i3035", 7, 0),
            (106, "oteryn:item.tibia.i3031", 13, 0),
        ],
    )
    .await?;
    let bank_before = bank(h).await?;
    let mut action = request(Operation::Fusion, 146);
    action.occurrence = occurrence_for(h, 146, |rolls| rolls[0] < 50 && rolls[2] < 7400).await?;
    let (action, (status, reply)) = exchange_pair(
        owner,
        admitted,
        action,
        PairSelection {
            indices: Some((4, 3)), // three newer coin stacks, donor, receiver
            ..PairSelection::default()
        },
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(reply["applied"], true, "{reply}");
    assert_eq!(reply["succeeded"], true);
    assert_eq!(reply["bonus"], 0);
    assert_eq!(reply["result"]["first_tier"], 1);
    assert_eq!(reply["result"]["second_tier"], Value::Null);
    assert_eq!(reply["result"]["gold_spent"], FEE);
    assert_eq!(support::balances(h).await?, (900, 1000, 0, 0));
    assert_eq!(bank(h).await?, bank_before - BANK_PART);
    item_state(h, id(101), Some(1)).await?;
    item_state(h, id(102), None).await?;
    for coin in [104, 105, 106] {
        item_state(h, id(coin), None).await?;
    }
    evidence(h, action, bank_before).await?;
    receipt(h, action, &reply, Some((100, 0, FEE))).await?;

    let after = assets(h).await?;
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, reply.clone())
    );
    assert_eq!(
        assets(h).await?,
        after,
        "replay cannot burn coins or debit bank twice"
    );
    evidence(h, action, bank_before).await?;
    receipt(h, action, &reply, Some((100, 0, FEE))).await
}

pub(in super::super) async fn failure_then_success(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    const DAGGER: &str = "oteryn:item.tibia.i3267";
    // The first fee spends exactly 25,000. The second spends three crystal
    // coins and must mint change alongside the Exaltation Chest.
    seed_items(h, &[
        (104, "oteryn:item.tibia.i3035", 50, 0),
        (105, "oteryn:item.tibia.i3043", 5, 0),
        (101, DAGGER, 1, 0),
        (102, DAGGER, 1, 0),
        (103, DAGGER, 1, 0),
    ]).await?;
    let mut failure = request(Operation::Fusion, 147);
    failure.occurrence = occurrence_for(h, 147, |rolls| rolls[0] >= 50).await?;
    let mut success = request(Operation::Fusion, 148);
    success.occurrence = occurrence_for(h, 148, |rolls| rolls[0] < 50 && rolls[2] < 7400).await?;
    let replies = super::super::item_forge_fixtures::pair_wire::paid_fusion_sequence(
        h, owner, admitted, [failure, success],
    ).await?;
    for (index, (action, reply)) in replies.iter().enumerate() {
        let tier = u8::try_from(index)?;
        assert_eq!(reply["bonus"], 0);
        assert_eq!(reply["result"]["first_tier"], tier);
        assert_eq!(reply["result"]["second_tier"], Value::Null);
        assert_eq!(reply["result"]["dust"], if index == 0 { 100 } else { 0 });
        assert_eq!(reply["result"]["cores_spent"], 0);
        receipt(h, *action, reply, Some((100, 0, FEE))).await?;
    }

    let rows: Vec<(Vec<u8>, String, i64, i64)> = sqlx::query_as(
        "SELECT uuid_send(p.game_session_id),p.command_id::text,f.change_gold_units,
                jsonb_array_length(p.outputs)::bigint
         FROM game_forge_operations p JOIN game_item_fee_burns f USING(transaction_id)
         WHERE p.character_id=encode($1,'hex')::uuid ORDER BY p.command_id LIMIT 3")
        .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
    let first = admitted.continuity.next_command_id;
    let session = admitted.game_session_id.as_bytes().to_vec();
    assert_eq!(rows, [
        (session.clone(), first.to_string(), 0, 0),
        (session, (first + 1).to_string(), 5_000, 1),
    ]);
    assert_eq!(bank(h).await?, 0);
    assert_eq!(support::balances(h).await?, (0, 200, 0, 0));
    item_state(h, id(101), Some(1)).await?;
    item_state(h, id(102), None).await?;
    item_state(h, id(103), None).await?;
    let children: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(e.item_instance_id)
           FROM game_forge_operations p JOIN game_item_nested_entries e
             ON e.parent_item_instance_id=(p.outputs->0->>'item')::uuid
          WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid
          LIMIT 2")
        .bind(id(41).as_slice())
        .bind(success.occurrence.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(children, [id(101).to_vec()]);
    Ok(())
}

async fn evidence(h: &Harness, action: Request, bank_before: i64) -> TestResult {
    let fee: (i16, i64, i64, i64, i64, bool) = sqlx::query_as("SELECT f.cause_kind,f.fee_gold_units,f.burned_gold_units,f.change_gold_units,f.bank_debit_gold_units,f.cause_occurrence_id=p.occurrence_id AND f.event_id=p.event_id AND f.envelope_sha256=p.envelope_sha256 FROM game_item_fee_burns f JOIN game_forge_operations p USING(transaction_id) WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    // GOLD-FEE-2 burns every eligible stack when bank pays the remainder; it
    // creates no change. Check that rule through the Forge cause and its event.
    assert_eq!(fee, (14, FEE, COINS, 0, BANK_PART, true));
    let lines: Vec<(i64, i64, i64)> = sqlx::query_as("SELECT l.coin_worth,l.quantity_before::bigint,l.quantity_after::bigint FROM game_item_fee_burn_lines l JOIN game_forge_operations p USING(transaction_id) WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid ORDER BY l.coin_worth")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(lines, [(1, 13, 0), (100, 7, 0), (10_000, 1, 0)]);
    let bank_entry: (i16, i64, i64, i64, bool) = sqlx::query_as("SELECT e.kind,e.amount,e.balance_before,e.balance_after,b.last_entry_id=e.entry_id AND b.balance=e.balance_after FROM game_account_bank_entries e JOIN game_forge_operations p ON p.transaction_id=e.fee_transaction_id JOIN game_account_bank_balances b ON b.account_id=e.account_id AND b.world_id=e.world_id WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        bank_entry,
        (
            i16::try_from(crate::durability::item_fee_burn_audit::LEDGER_FEE_DEBIT)?,
            BANK_PART,
            bank_before,
            bank_before - BANK_PART,
            true
        )
    );
    let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_forge_operations p ON p.transaction_id=i.minted_transaction_id WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(minted, 0, "mixed payment cannot mint a change stack");
    Ok(())
}
