//! The full CharacterRevision-bearing fence, separately from the cooldown's
//! item-only fence. Channel controls do not qualify an admitted Instance.
use super::{TestResult, fixture, id};
use crate::durability::character_authority::assert_recovery_fence;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::runtime_scope_assignment::{BootstrapSecret, NodeIncarnationProof};
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};

async fn full_fence(
    h: &fixture::Harness,
    fence: CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
) -> TestResult<Result<u64, CharacterProgressionError>> {
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    let node = node.clone();
    Ok(h.root
        .try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let outcome = assert_gameplay_fence(&mut tx, &fence, &node)
                    .await?
                    .map(|root| root.revision);
                commit_semantic_transaction(tx, deadline).await?;
                Ok(outcome)
            })
        })
        .await?)
}

async fn qualify(h: &fixture::Harness) -> TestResult {
    let current = fixture::fence(1)?;
    assert!(matches!(full_fence(h, current, &h.node).await?, Ok(1)));
    let stale_revision = fixture::fence(2)?;
    assert!(matches!(
        full_fence(h, stale_revision, &h.node).await?,
        Err(CharacterProgressionError::CharacterRevisionMismatch)
    ));
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = current.runtime_scope
    else {
        return Err("fixture must start in a Channel".into());
    };
    // Each case starts with the same valid participant. A wrong revision too
    // must never turn a failed session/runtime check into a root-only check.
    let mut cases = Vec::new();
    let mut wrong = current;
    wrong.character_id = crate::domain::CharacterId::from_bytes(id(91)).map_err(fixture::debug)?;
    cases.push(("Character", wrong));
    let mut wrong = current;
    wrong.game_session_id = GameSessionId::decode(&id(91)).map_err(fixture::debug)?;
    cases.push(("session", wrong));
    let mut wrong = current;
    wrong.connection_generation = ConnectionGeneration::new(2).map_err(fixture::debug)?;
    cases.push(("connection generation", wrong));
    let mut wrong = current;
    wrong.character_lease_generation = 2;
    cases.push(("lease generation", wrong));
    let mut wrong = current;
    wrong.scope_ownership_generation = ScopeOwnershipGeneration::new(2).map_err(fixture::debug)?;
    cases.push(("ownership generation", wrong));
    let mut wrong = current;
    wrong.runtime_scope = RuntimeScopeRefV1::channel(
        WorldId::decode(&id(91)).map_err(fixture::debug)?,
        channel_id,
    );
    cases.push(("World", wrong));
    let mut wrong = current;
    wrong.runtime_scope = RuntimeScopeRefV1::channel(
        world_id,
        ChannelId::decode(&id(91)).map_err(fixture::debug)?,
    );
    cases.push(("Channel", wrong));
    let mut wrong = current;
    wrong.runtime_scope =
        RuntimeScopeRefV1::instance(world_id, *channel_id.as_bytes()).map_err(fixture::debug)?;
    cases.push(("Instance with identical UUID bytes", wrong));
    for (label, wrong) in cases {
        for expected_character_revision in [
            current.expected_character_revision,
            stale_revision.expected_character_revision,
        ] {
            let wrong = CurrentCharacterGameplayFence {
                expected_character_revision,
                ..wrong
            };
            assert!(
                matches!(
                    full_fence(h, wrong, &h.node).await?,
                    Err(CharacterProgressionError::AuthorityRejected)
                ),
                "{label}, revision {expected_character_revision:?}"
            );
        }
        assert!(matches!(full_fence(h, current, &h.node).await?, Ok(1)));
    }
    let false_proof =
        NodeIncarnationProof::new(h.node.fact(), BootstrapSecret::from_bytes([2; 32]));
    assert!(matches!(
        full_fence(h, current, &false_proof).await?,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    assert!(matches!(full_fence(h, current, &h.node).await?, Ok(1)));
    Ok(())
}

#[test]
fn boss_activity_full_character_fence_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h = fixture::Harness::create(admin, "boss_full_fence", true).await?;
        eprintln!("Canonical PostgreSQL 18.6; full Character fence Channel controls and typed Instance refusal (no activity admission)");
        let result = qualify(&h).await;
        h.cleanup().await?;
        result
    })
}
