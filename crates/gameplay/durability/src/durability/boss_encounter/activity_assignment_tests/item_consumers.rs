//! Channel item consumers share the live root proof required before activity
//! admission can enable another scope. These controls grant no Instance session.
use super::{TestResult, fixture, id};
use crate::durability::character_authority::assert_recovery_fence;
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, character_item_fence_is_current, character_item_fence_without_root,
};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};

async fn item_fences(
    h: &fixture::Harness,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    reserved_character: crate::domain::CharacterId,
    reserved_world: [u8; 16],
    reserved_channel: [u8; 16],
) -> TestResult<(bool, bool)> {
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    let node = h.node.clone();
    Ok(h.root
        .try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let session = character_item_fence_without_root(
                    &mut tx,
                    &node,
                    &fence,
                    command,
                    reserved_character,
                    reserved_world,
                    reserved_channel,
                )
                .await?;
                let root = character_item_fence_is_current(
                    &mut tx,
                    &node,
                    &fence,
                    command,
                    reserved_character,
                    reserved_world,
                    reserved_channel,
                )
                .await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok((session, root))
            })
        })
        .await?)
}

async fn qualify(h: &fixture::Harness) -> TestResult {
    let fence = fixture::fence(1)?.item_fence();
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err("Channel fixture required".into());
    };
    let command = CommandRef::new(
        fence.game_session_id,
        CommandId::new(101).map_err(fixture::debug)?,
    );
    let check = |fence, command, character, world, channel| {
        item_fences(h, fence, command, character, world, channel)
    };
    assert_eq!(
        check(
            fence,
            command,
            fence.character_id,
            *world_id.as_bytes(),
            *channel_id.as_bytes()
        )
        .await?,
        (true, true)
    );
    // Every command/reservation dimension is checked before accepting a session
    // proof, including a scope kind with exactly the same UUID bytes.
    let wrong_session =
        crate::foundation::GameSessionId::decode(&id(91)).map_err(fixture::debug)?;
    let wrong_character = crate::domain::CharacterId::from_bytes(id(91)).map_err(fixture::debug)?;
    let mut instance = fence;
    instance.runtime_scope =
        RuntimeScopeRefV1::instance(world_id, *channel_id.as_bytes()).map_err(fixture::debug)?;
    for (changed, command, character, world, channel) in [
        (
            fence,
            CommandRef::new(wrong_session, command.command_id()),
            fence.character_id,
            *world_id.as_bytes(),
            *channel_id.as_bytes(),
        ),
        (
            fence,
            command,
            wrong_character,
            *world_id.as_bytes(),
            *channel_id.as_bytes(),
        ),
        (
            fence,
            command,
            fence.character_id,
            id(91),
            *channel_id.as_bytes(),
        ),
        (
            fence,
            command,
            fence.character_id,
            *world_id.as_bytes(),
            id(91),
        ),
        (
            instance,
            command,
            fence.character_id,
            *world_id.as_bytes(),
            *channel_id.as_bytes(),
        ),
    ] {
        assert_eq!(
            check(changed, command, character, world, channel).await?,
            (false, false)
        );
    }
    for index in 0..4 {
        let mut revisions = ["profile-1", "ruleset-1", "content-1", "starter-1"];
        revisions[index] = "boss-item-incompatible-2";
        sqlx::query("SELECT game_character_configure_interpretation($1,$2,$3,$4)")
            .bind(revisions[0])
            .bind(revisions[1])
            .bind(revisions[2])
            .bind(revisions[3])
            .execute(&h.pool)
            .await?;
        assert_eq!(
            check(
                fence,
                command,
                fence.character_id,
                *world_id.as_bytes(),
                *channel_id.as_bytes()
            )
            .await?,
            (true, false),
            "interpretation column {index}"
        );
        // Restore by appending another interpretation, never rewriting history.
        sqlx::query("SELECT game_character_configure_interpretation('profile-1','ruleset-1','content-1','starter-1')")
            .execute(&h.pool).await?;
        assert_eq!(
            check(
                fence,
                command,
                fence.character_id,
                *world_id.as_bytes(),
                *channel_id.as_bytes()
            )
            .await?,
            (true, true),
            "restored column {index}"
        );
    }
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        revision, "1",
        "fence reads must never advance CharacterRevision"
    );
    Ok(())
}

#[test]
fn boss_activity_item_consumers_postgres() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async move {
        let h = fixture::Harness::create(admin, "boss_item_consumers", true).await?;
        eprintln!("Canonical PostgreSQL 18.6; Channel item consumer interpretation and reserved-scope fences (no activity admission)");
        let result = qualify(&h).await;
        h.cleanup().await?;
        result
    })
}
