//! Actual native item-effect journal observation and capability350 wire-codec assertions.
use super::debug;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

pub(super) async fn poff_count(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Result<usize, Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::presentation::{
        PresentationEventKind, decode_item_effect_batch, encode_item_effect_batch,
    };
    let (cursor, events) = owner
        .observe_sounds(actor, session, 0)
        .await
        .ok_or("native presentation observer")?;
    let events: Vec<_> = events
        .into_iter()
        .filter(|event| matches!(event.kind, PresentationEventKind::MagicEffect { .. }))
        .collect();
    if !events.is_empty() {
        let encoded = encode_item_effect_batch(&events).map_err(debug)?;
        assert_eq!(decode_item_effect_batch(&encoded).map_err(debug)?, events);
    }
    let count = events
        .iter()
        .filter(|event| {
            matches!(
                &event.kind,
                PresentationEventKind::MagicEffect { effect_id: 3, .. }
            )
        })
        .count();
    let (_, later) = owner
        .observe_sounds(actor, session, cursor)
        .await
        .ok_or("native presentation cursor")?;
    assert!(
        !later.iter().any(|event| matches!(
            &event.kind,
            PresentationEventKind::MagicEffect { effect_id: 3, .. }
        )),
        "observer cursor must not repeat a consumed effect"
    );
    Ok(count)
}

/// Missing durable receipts retire a prepared cue without publishing it.
pub(super) async fn assert_rollback_reconciled(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        owner.reconcile_pending_item_effects(actor, session).await,
        "the exact absent receipt must reconcile the rolled-back presentation hold"
    );
    assert_eq!(
        poff_count(owner, actor, session).await?,
        0,
        "rollback must not publish the donor effect"
    );
    Ok(())
}
