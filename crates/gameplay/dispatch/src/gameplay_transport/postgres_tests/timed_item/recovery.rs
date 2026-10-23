//! A retained ambiguous request must reconcile even after its item leaves custody.
use super::*;
use crate::durability::item_timed_runtime::{TimedOperation, TimedRequest};
use crate::gameplay_transport::timed_item::ItemClock;
use sha2::{Digest, Sha256};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    payload: &[u8],
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let definition = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3081".into(),
        revision_ref: native
            .item_index()
            .and_then(|index| index.revision("oteryn:item.tibia.i3081"))
            .ok_or("revision")?
            .into(),
    };
    let (definition, target) = crate::gameplay_transport::timed_item::facts(native, &definition)
        .ok_or("timed definition")?;
    let request = TimedRequest {
        transaction_id: crate::gameplay_transport::npc_trade_command::derived(
            b"oteryn:timed-command:v1",
            admitted.game_session_id,
            7,
        ),
        event_id: crate::gameplay_transport::npc_trade_command::derived(
            b"oteryn:timed-event:v1",
            admitted.game_session_id,
            7,
        ),
        content_digest: native.source_digest(),
        binding: Sha256::digest(payload).into(),
        item: id(100),
        expected_revision: 4,
        definition,
        target,
        operation: TimedOperation::Use { elapsed_ms: 0 },
        occurred_at_unix_ms: 1,
    };
    // Model a commit whose acknowledgement was lost: the real dispatch above already
    // burned this item, while the owner's pending request still names its old custody.
    owner
        .states
        .timed_items
        .sessions
        .lock()
        .await
        .entry(admitted.game_session_id)
        .or_default()
        .items
        .insert(
            id(100),
            ItemClock {
                at_ms: 0,
                pending: Some((request, 1)),
            },
        );
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_state_writes")
        .fetch_one(&h.pool)
        .await?;
    owner.tick_timed_items().await;
    assert!(
        !owner
            .states
            .timed_items
            .sessions
            .lock()
            .await
            .get(&admitted.game_session_id)
            .ok_or("session clock")?
            .items
            .contains_key(&id(100)),
        "committed burn receipt retires its pending clock"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_timed_state_writes")
            .fetch_one(&h.pool)
            .await?,
        before,
        "reconciliation never consumes twice"
    );
    Ok(())
}
