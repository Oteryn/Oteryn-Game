use super::*;

pub(super) async fn move_and_disband(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    leader: CurrentCharacterItemFence,
    old_line: RelayLine,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("fence")?;
    let invite = request(
        2,
        gid(91)?,
        leader,
        GuildCommand::Invite(fence.character_id),
    )?;
    assert!(
        matches!(h.root.commit_guild_membership(owner.character,&h.node,leader,invite).await.map_err(debug)?,GuildOutcome::Committed(r) if r.result==GuildResult::Ok)
    );
    let join = request(3, gid(91)?, fence, GuildCommand::Accept)?;
    assert!(
        matches!(h.root.commit_guild_membership(owner.character,&h.node,fence,join).await.map_err(debug)?,GuildOutcome::Committed(r) if r.result==GuildResult::Ok)
    );
    owner.chat.deliver(&old_line);
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
    owner.chat.deliver(&payload(gid(91)?, "new Guild only")?);
    assert!(
        matches!(owner.chat_next_bound(admitted, &mut 64).await,Some(ChatLine::Guild(line)) if line.guild_id==*gid(91)?.as_bytes())
    );
    owner.chat_delivered(admitted.game_session_id).await;
    exclusion_race_tests::exclusion_wins(h, owner, admitted, leader).await?;
    let invite = request(
        5,
        gid(91)?,
        leader,
        GuildCommand::Invite(fence.character_id),
    )?;
    h.root
        .commit_guild_membership(owner.character, &h.node, leader, invite)
        .await
        .map_err(debug)?;
    let join = request(6, gid(91)?, fence, GuildCommand::Accept)?;
    h.root
        .commit_guild_membership(owner.character, &h.node, fence, join)
        .await
        .map_err(debug)?;
    let disband = GuildDisbandRequest::new(
        request(7, gid(91)?, leader, GuildCommand::Disband)?,
        "Second Guild",
    )
    .map_err(debug)?;
    h.root
        .commit_guild_disband(owner.character, &h.node, leader, disband)
        .await
        .map_err(debug)?;
    // DISBANDING itself does not close the guild room. Each committed removal does.
    owner
        .chat
        .deliver(&payload(gid(91)?, "still a committed member")?);
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_some());
    owner.chat_delivered(admitted.game_session_id).await;
    let step = GuildDisbandJobRequest {
        world_id: admitted.world_id,
        guild_id: gid(91)?,
    };
    assert!(matches!(
        h.root
            .advance_guild_disband(owner.character, &h.node, step)
            .await
            .map_err(debug)?,
        GuildDisbandOutcome::Advanced { removed: 1, .. }
    ));
    owner
        .chat
        .deliver(&payload(gid(91)?, "removed by disband step")?);
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
    assert_eq!(
        owner.chat_guild_admit(admitted).await,
        GuildRoomState::Absent
    );
    // A failed owner read is distinct from non-membership, and queued data grants nothing.
    owner.chat.guild.take_drops();
    sqlx::query("ALTER TABLE game_guild_members RENAME TO guild_chat_unavailable_members")
        .execute(&h.pool)
        .await?;
    assert_eq!(
        owner.chat_guild_admit(admitted).await,
        GuildRoomState::Unavailable
    );
    owner.chat.deliver(&payload(gid(91)?, "unavailable")?);
    assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
    assert_eq!(owner.chat.guild.take_drops(), 1);
    sqlx::query("ALTER TABLE guild_chat_unavailable_members RENAME TO game_guild_members")
        .execute(&h.pool)
        .await?;
    Ok(())
}
