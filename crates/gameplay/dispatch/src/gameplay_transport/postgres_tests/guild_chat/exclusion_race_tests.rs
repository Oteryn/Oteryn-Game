use super::*;

pub(super) async fn exclusion_wins(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    leader: CurrentCharacterItemFence,
) -> TestResult {
    let exclude = request(
        4,
        gid(91)?,
        leader,
        GuildCommand::Exclude(admitted.item_fence.ok_or("fence")?.character_id),
    )?;
    let binding = binding(&leader, &exclude).map_err(debug)?;
    let recovery = owner.character.record_for(&h.root).map_err(debug)?;
    // The real Guild owner writes remain invisible until commit, and rollback restores them.
    let mut rollback = h.pool.begin().await?;
    execute_fenced_operation(
        &mut rollback,
        &recovery,
        &h.node,
        &leader,
        &exclude,
        &binding,
        None,
    )
    .await
    .map_err(debug)?;
    let visible: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(visible, 1);
    rollback.rollback().await?;
    owner
        .states
        .chat
        .deliver(&payload(gid(91)?, "rollback retains membership")?);
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_some());
    owner.chat_delivered(admitted.game_session_id).await;

    let (server, mut client) = tokio::io::duplex(64);
    let serving = serve(server, admitted, owner);
    let driving = async {
        wire_tests::member_login(&mut client).await?;
        let mut exclusion = h.pool.begin().await?;
        let result = execute_fenced_operation(
            &mut exclusion,
            &recovery,
            &h.node,
            &leader,
            &exclude,
            &binding,
            None,
        )
        .await
        .map_err(debug)?;
        assert!(
            matches!(result,GuildOutcome::Committed(receipt) if receipt.result==GuildResult::Ok)
        );
        owner
            .states
            .chat
            .deliver(&payload(gid(91)?, "must lose to exclusion")?);
        // Observe the actual connection dequeue waiting behind the native writer's
        // admission relation custody. No timing guess determines who won.
        tokio::time::timeout(Duration::from_secs(5),async {
            loop {
                let waiting: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE NOT granted AND relation='game_durability_admission_account_guards'::regclass)").fetch_one(&h.pool).await?;
                if waiting { return Ok::<(),Box<dyn std::error::Error>>(()); }
                tokio::task::yield_now().await;
            }
        }).await??;
        exclusion.commit().await?;
        custody_tests::send_not_allowed(&mut client, admitted.continuity.next_command_id).await?;
        client.shutdown().await?;
        Ok(())
    };
    let (ended, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(20), fixture::join_two(serving, driving)).await?;
    result?;
    assert_eq!(
        ended?.continuity.next_command_id,
        admitted.continuity.next_command_id + 1
    );
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
    Ok(())
}
