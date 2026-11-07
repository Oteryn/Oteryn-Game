//! Actual command669 targets a current Ground handle, never an inventory stand-in.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    reference: std::num::NonZeroU32,
    after: &str,
) -> TestResult {
    let before:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (mut next, result, actual) = super::super::rust_lottery::exchange_rotate(
        owner,
        admitted,
        1,
        &body,
        Some(reference),
        reference,
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "actual Ground command669 rotates the retained physical statue"
    );
    let changed:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'ground',to_jsonb(g)) FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(changed["ground"], before["ground"]);
    assert_eq!(
        changed["item"]["item_instance_id"],
        before["item"]["item_instance_id"]
    );
    assert_eq!(
        changed["item"]["minted_transaction_id"],
        before["item"]["minted_transaction_id"]
    );
    assert_eq!(changed["item"]["definition_production_key"], after);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,0,"inactive orientation cannot gain a free charge/clock");
    owner.states.command_ingress.forget(next.game_session_id);
    next.continuity.next_command_id = 1;
    let (_, replayed, _) =
        super::super::rust_lottery::exchange_rotate(owner, next, 1, &actual, None, reference)
            .await?;
    assert_eq!(replayed, UseDisposition::Committed);
    assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,changed["item"],"native replay rotates only once");
    Ok(())
}
