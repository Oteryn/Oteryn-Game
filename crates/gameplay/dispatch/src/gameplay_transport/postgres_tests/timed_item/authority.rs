use super::*;
use crate::durability::item_timed_runtime::{TimedOperation, TimedRequest};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let definition = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3081".into(),
        revision_ref: "definition-r1".into(),
    };
    let (definition, target) =
        crate::gameplay_transport::timed_item_content::facts(native, &definition)
            .ok_or("timed facts")?;
    for case in 0..6 {
        let mut fence = current;
        let mut request = TimedRequest {
            transaction_id: id(150 + case * 2),
            event_id: id(151 + case * 2),
            content_digest: native.source_digest(),
            binding: [case; 32],
            item: id(100),
            expected_revision: 0,
            definition: definition.clone(),
            target: target.clone(),
            operation: TimedOperation::Use { elapsed_ms: 0 },
            occurred_at_unix_ms: crate::gameplay_transport::npc_trade_command::unix_ms_now(),
        };
        match case {
            0 => {
                fence.connection_generation =
                    crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
            }
            1 => fence.character_lease_generation += 1,
            2 => {
                fence.scope_ownership_generation =
                    crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?
            }
            3 => {
                fence.expected_character_revision =
                    crate::domain::CharacterRevision::new(2).map_err(debug)?
            }
            4 => request.content_digest = [8; 32],
            _ => {
                request.item = id(90);
                (request.definition, request.target) =
                    crate::gameplay_transport::timed_item_content::facts(
                        native,
                        &crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i6529".into(),
                            revision_ref: "definition-r1".into(),
                        },
                    )
                    .ok_or("boots facts")?;
                request.operation = TimedOperation::Advance(1_000);
            }
        }
        let result = owner
            .root
            .commit_timed_action(owner.character, owner.holder, fence, request)
            .await;
        assert!(
            matches!(
                result,
                Err(crate::durability::item_timed_state::TimedWriteError::AuthorityRejected)
            ) && case < 5
                || matches!(
                    result,
                    Err(crate::durability::item_timed_state::TimedWriteError::NotHeld)
                ) && case == 5,
            "case {case}: {result:?}"
        );
    }
    let writes: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_state_writes")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(writes, 0);
    Ok(())
}

pub(super) async fn concurrency(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("current")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let definition = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3081".into(),
        revision_ref: "definition-r1".into(),
    };
    let (definition, target) =
        crate::gameplay_transport::timed_item_content::facts(native, &definition)
            .ok_or("timed facts")?;
    let first = TimedRequest {
        transaction_id: id(170),
        event_id: id(171),
        content_digest: native.source_digest(),
        binding: [170; 32],
        item: id(120),
        expected_revision: 1,
        definition,
        target,
        operation: TimedOperation::Use { elapsed_ms: 0 },
        occurred_at_unix_ms: crate::gameplay_transport::npc_trade_command::unix_ms_now(),
    };
    let second = TimedRequest {
        transaction_id: id(172),
        event_id: id(173),
        binding: [172; 32],
        ..first.clone()
    };
    use std::future::Future;
    use std::task::Poll;
    let retry_first = first.clone();
    let retry_second = second.clone();
    let mut left = std::pin::pin!(owner.root.commit_timed_action(
        owner.character,
        owner.holder,
        fence,
        first
    ));
    let mut right = std::pin::pin!(owner.root.commit_timed_action(
        owner.character,
        owner.holder,
        fence,
        second
    ));
    let (mut a, mut b) = (None, None);
    let (a, b) = std::future::poll_fn(|context| {
        if a.is_none()
            && let Poll::Ready(value) = left.as_mut().poll(context)
        {
            a = Some(value);
        }
        if b.is_none()
            && let Poll::Ready(value) = right.as_mut().poll(context)
        {
            b = Some(value);
        }
        if a.is_some() && b.is_some() {
            Poll::Ready((
                a.take().expect("left completed"),
                b.take().expect("right completed"),
            ))
        } else {
            Poll::Pending
        }
    })
    .await;
    // The accepted root has one ready holder. A competing attempt may be refused before
    // starting a transaction; retry exactly that known-not-issued request after its peer.
    let mut results = Vec::new();
    for (result, request) in [(a, retry_first), (b, retry_second)] {
        let result = match result {
            Err(crate::durability::item_timed_state::TimedWriteError::Unavailable(
                crate::durability::DurabilityError::RootUnavailable,
            )) => {
                owner
                    .root
                    .commit_timed_action(owner.character, owner.holder, fence, request)
                    .await
            }
            other => other,
        };
        results.push(result.map_err(debug)?);
    }
    let a = results[0];
    let b = results[1];
    assert_eq!(
        [a, b]
            .iter()
            .filter(|value| value.disposition == TimedDisposition::Ok)
            .count(),
        1
    );
    assert_eq!(
        [a, b]
            .iter()
            .filter(|value| value.disposition == TimedDisposition::Stale)
            .count(),
        1
    );
    let row:(Option<i32>,String)=sqlx::query_as("SELECT charges,state_revision::text FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(120).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(row, (Some(3), "2".into()));
    Ok(())
}
