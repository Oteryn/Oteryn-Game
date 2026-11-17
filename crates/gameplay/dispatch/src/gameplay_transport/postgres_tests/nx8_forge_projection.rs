//! NX-8 P19: a current balance read is independent of retained mutation history.
use super::{TestResult, fixture::Harness, item_forge::support};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use oteryn_protocol_oteryn::{CommandStatus, item_forge::COMMAND_TYPE_FORGE_DUST_QUERY};

#[test]
fn nx8_forge_postgres_current_dust_without_history_and_generation_fence() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("nx8-current-dust", || {
        support::scenario(support::Case::CurrentDust)
    })
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let started = std::time::Instant::now();
    let query =
        async |session: &mut AdmittedSession| -> TestResult<(CommandStatus, serde_json::Value)> {
            let (status, bytes) =
                support::exchange_typed(owner, session, COMMAND_TYPE_FORGE_DUST_QUERY, &[]).await?;
            Ok((
                status,
                if bytes.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::from_slice(&bytes)?
                },
            ))
        };
    let (status, value) = query(admitted).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(value["current_dust"], 100);
    assert_eq!(value["dust_limit"], 100);
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM game_forge_history")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(history, 0, "balance exists without a forge history receipt");
    // The existing dust owner changes the balance, but creates no forge operation receipt.
    support::exchange(
        owner,
        admitted,
        support::request(
            oteryn_protocol_oteryn::item_forge::Operation::DustToSlivers,
            231,
        ),
    )
    .await?;
    support::gain_dust(h, *admitted, 60).await?;
    let (_, current) = query(admitted).await?;
    assert_eq!(
        current["current_dust"], 100,
        "current balance differs from the conversion receipt balance of 40"
    );
    let mut stale = *admitted;
    stale
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .connection_generation =
        crate::foundation::ConnectionGeneration::new(admitted.continuity.connection_generation + 1)
            .map_err(super::fixture::debug)?;
    assert_eq!(
        owner
            .item_forge_balance(stale, stale.continuity.next_command_id)
            .await,
        Some(Vec::new()),
        "the projection body refuses a stale connection fence without exposing a balance"
    );
    let next_command = stale.continuity.next_command_id + 1;
    let generation = stale.continuity.connection_generation;
    let Err(error) = query(&mut stale).await else {
        return Err("stale fence must close at command ingress before a reply".into());
    };
    let error = error.to_string();
    assert!(error.contains("server frame read failed"), "{error}");
    assert!(error.contains("unexpected end of file"), "{error}");
    assert!(
        error.contains(&format!(
            "awaited server outcome: admitted then disconnected; generation={generation}; next_command={next_command}"
        )),
        "stale input must reach the existing ingress disconnection contract: {error}"
    );
    eprintln!(
        "POSTGRES_EXECUTED NX-8 P19 elapsed={:.3}s initial=100 current_after_conversion_and_gain=100 stale_body=refused stale_ingress=disconnected",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
