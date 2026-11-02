use super::super::fixture::{Harness, id};
use super::TestResult;
use super::support::{self, Case, count, exchange};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::item_forge as wire;

#[test]
fn client_e2e_item_forge() -> TestResult {
    support::scenario(Case::ItemForge)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // Seed only the resource precondition, using the native Forge fixture's chain shape.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_forge_dust(character_id,balance,dust_limit) VALUES(encode($1,'hex')::uuid,0,100)").bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'GAIN','creature_kill',encode($3,'hex')::uuid,encode($4,'hex')::uuid,100,0,0,100,100,100)")
        .bind(id(225).as_slice()).bind(id(41).as_slice()).bind(id(226).as_slice()).bind(id(227).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "UPDATE game_character_forge_dust SET balance=100,last_entry_id=encode($1,'hex')::uuid",
    )
    .bind(id(225).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    // PlayLink::item_forge_request -> item_forge::command forwards this protocol
    // Request unchanged; item_forge::apply_result retains the returned JSON bytes.
    let payload = wire::Request {
        operation: wire::Operation::DustToSlivers,
        occurrence: id(110),
        first_item: None,
        second_item: None,
        boost: false,
        protect: false,
        history_limit: 0,
    }
    .encode()
    .ok_or("forge request")?;
    let first = exchange(owner, admitted, 1, 181, &payload).await?;
    let receipt: serde_json::Value = serde_json::from_slice(&first)?;
    assert_eq!(receipt["applied"], true);
    assert_eq!(
        super::super::item_forge::support::balances(h).await?,
        (40, 100, 3, 0)
    );
    // A new transport command carrying the SAME occurrence reaches durable replay.
    assert_eq!(exchange(owner, admitted, 2, 181, &payload).await?, first);
    assert_eq!(
        super::super::item_forge::support::balances(h).await?,
        (40, 100, 3, 0)
    );
    assert_eq!(
        count(
            h,
            "SELECT count(*) FROM game_forge_operations WHERE applied"
        )
        .await?,
        1
    );
    Ok(())
}
