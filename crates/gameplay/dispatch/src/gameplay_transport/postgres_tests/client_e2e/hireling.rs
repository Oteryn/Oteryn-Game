use super::super::fixture::{Harness, debug, id};
use super::TestResult;
use super::support::{self, Case, count, exchange};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::hireling as wire;

#[test]
#[ignore = "CLIENT-E2E-1: Hireling Query cannot reach durable read; position.is_some() == dismiss rejects Query (None,false), crates/gameplay/durability/src/durability/house_hireling.rs:14"]
fn client_e2e_hireling() -> TestResult {
    support::scenario(Case::Hireling)
}

/// Admission fixture only: an already committed house resident, so Query reaches the
/// durable read guard rather than failing because the Character is outside a house.
pub(super) async fn prepare(
    pool: sqlx::PgPool,
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    admitted: AdmittedSession,
) -> TestResult<crate::foundation::ExactActorRef> {
    const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
    sqlx::query(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/tests/support/owned_house_fixture.sql"
    )))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&pool)
    .await?;
    let instance: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(game_house_scope_instance_id(encode($1,'hex')::uuid,$2))",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .fetch_one(&pool)
    .await?;
    let scope = crate::foundation::RuntimeScopeRefV1::instance(
        admitted.world_id,
        instance.as_slice().try_into()?,
    )
    .map_err(debug)?;
    let lease = crate::foundation::CharacterLease::new(
        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
        2,
    )
    .map_err(debug)?;
    let position = crate::foundation::LocalPosition {
        x: 7,
        y: 0,
        floor: 7,
    };
    let actor = runtime
        .restore_transfer_actor(
            crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?,
            admitted.game_session_id,
            lease,
            position,
            Some(
                oteryn_gameplay_entities::carrier::house_custody::HouseActorState {
                    house: HOUSE.into(),
                    scope,
                    generation: crate::foundation::ScopeOwnershipGeneration::new(1)
                        .map_err(debug)?,
                    position,
                    lease,
                    origin_position: position,
                },
            ),
        )
        .map_err(debug)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET runtime_scope_kind=2,runtime_scope_channel_id=NULL,runtime_scope_instance_id=game_house_scope_instance_id(runtime_scope_world_id,$1),runtime_scope_house_key=$1,origin_channel_id=encode($2,'hex')::uuid WHERE game_session_id=encode($3,'hex')::uuid")
        .bind(HOUSE).bind(id(43).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(actor)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // PlayLink::hireling_input opens with hireling_request(Intent::Refresh), which
    // builds Request::Query; HirelingResponse uses hireling::decode_reply.
    // A real house owner/session is seeded by support before this request.
    let bytes = exchange(
        owner,
        admitted,
        1,
        wire::COMMAND_TYPE_HIRELING,
        &wire::Request::Query.encode(),
    )
    .await?;
    assert_eq!(wire::decode_reply(&bytes), Some(false));
    assert_eq!(
        count(h, "SELECT count(*) FROM game_house_hirelings").await?,
        0
    );
    Ok(())
}
