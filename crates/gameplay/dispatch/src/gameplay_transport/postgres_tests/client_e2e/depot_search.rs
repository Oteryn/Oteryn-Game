use super::super::fixture::{Harness, debug, id};
use super::TestResult;
use super::support::{self, Case, count, exchange};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::depot_search as wire;

#[test]
fn client_e2e_depot_search() -> TestResult {
    support::scenario(Case::DepotSearch)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // Put three native slivers in depot custody; retrieval must use the real item loop.
    super::super::item_forge::support::seed_items(h, &[(90, "oteryn:item.tibia.i37109", 3, 0)])
        .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(90).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_character_depot_locations(character_id,world_id,item_instance_id,town_id,box_number,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0,1,1)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(90).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_depot_box_counters(character_id,town_id,box_number,next_ordinal,committed) VALUES(encode($1,'hex')::uuid,0,1,2,1)").bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    // PlayLink::depot_search_request -> depot_search::command forwards command 182.
    let search = wire::encode_request(wire::Request::Search { client_id: 37109 }).map_err(debug)?;
    let found = wire::decode_reply(
        &exchange(owner, admitted, 1, wire::COMMAND_TYPE_DEPOT_SEARCH, &search).await?,
    )
    .map_err(debug)?;
    assert_eq!(found.outcome, wire::Outcome::Found);
    assert_eq!(found.results.len(), 1);
    assert_eq!(found.results[0].depot, 3);
    let payload = wire::encode_request(wire::Request::Retrieve {
        client_id: 37109,
        all: true,
    })
    .map_err(debug)?;
    let first = exchange(
        owner,
        admitted,
        2,
        wire::COMMAND_TYPE_DEPOT_SEARCH,
        &payload,
    )
    .await?;
    let reply = wire::decode_reply(&first).map_err(debug)?;
    assert_eq!(
        (reply.outcome, reply.retrieved),
        (wire::Outcome::Retrieved, 3)
    );
    assert_eq!(
        count(
            h,
            "SELECT count(*) FROM game_item_character_depot_locations"
        )
        .await?,
        0
    );
    assert_eq!(count(h,"SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=(SELECT item_instance_id FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i37109')").await?,1);
    let receipts = count(h, "SELECT count(*) FROM game_item_loop_move_receipts").await?;
    assert!(receipts > 0);
    // Replay the original command id over a new connection; neither custody nor receipt
    // count may change a second time. Keep the current stream continuity while replaying
    // command 2; rewinding next_command_id would refuse the connection at setup.
    let mut replay = *admitted;
    assert_eq!(
        exchange(
            owner,
            &mut replay,
            2,
            wire::COMMAND_TYPE_DEPOT_SEARCH,
            &payload
        )
        .await?,
        first
    );
    assert_eq!(
        count(h, "SELECT count(*) FROM game_item_loop_move_receipts").await?,
        receipts
    );
    assert_eq!(count(h,"SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i37109' AND lifecycle=1").await?,3);
    Ok(())
}
