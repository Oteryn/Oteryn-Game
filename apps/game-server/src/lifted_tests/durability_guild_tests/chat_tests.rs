use super::*;
#[test]
fn guild_chat_native_login_motd_and_send_rank_are_current_committed_data() -> TestResult {
    text_run("guild_chat_current", async |h| {
        resign_fixture(h).await?;
        guard(h).await?;
        let original = chat_read(h, None).await?.ok_or("missing initial member")?;
        assert_eq!(original.guild_id, gid(1));
        assert_eq!(original.rank, GuildRank::LEADER);
        assert_eq!(original.motd, "");
        assert_eq!(original.state, GuildState::Active);
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(1, GuildCommand::SetMessage, 1, "Welcome today")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let login = chat_read(h, None).await?.ok_or("missing login member")?;
        assert_eq!(login.motd, "Welcome today");
        assert_eq!(
            result(resign_apply(h, resign_request(2, 91, 2), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let sender = chat_read(h, None).await?.ok_or("missing current sender")?;
        assert_eq!(sender.rank, GuildRank::VICE);
        assert_eq!(sender.motd, "Welcome today");
        assert_eq!(original.rank, GuildRank::LEADER); // retained DATA never changes into current authority
        assert_eq!(original.motd, "");
        Ok(())
    })
}
#[test]
fn guild_chat_native_payload_guild_never_reroutes_after_committed_move() -> TestResult {
    text_run("guild_chat_move", async |h| {
        historical_character(h, 90, id(WORLD)).await?;
        historical_character(h, 91, id(WORLD)).await?;
        initial_guild(h, 1, 90).await?;
        initial_guild(h, 2, 91).await?;
        initial_member(h, 1, 0, 3).await?;
        initial_invitation(h, 2, 0, 91, false).await?;
        guard(h).await?;
        let sent = chat_read(h, None)
            .await?
            .ok_or("sender not in original Guild")?;
        assert_eq!(sent.guild_id, gid(1));
        assert!(chat_read(h, Some(gid(2))).await?.is_none());
        assert_eq!(
            result(apply(h, request(1, 1, GuildCommand::Leave), fence()?).await?).result,
            GuildResult::Ok
        );
        assert!(chat_read(h, None).await?.is_none());
        assert!(chat_read(h, Some(sent.guild_id)).await?.is_none());
        assert_eq!(
            result(apply(h, request(2, 2, GuildCommand::Accept), fence()?).await?).result,
            GuildResult::Ok
        );
        assert_eq!(
            chat_read(h, None)
                .await?
                .ok_or("missing new Guild")?
                .guild_id,
            gid(2)
        );
        assert!(chat_read(h, Some(sent.guild_id)).await?.is_none());
        assert_eq!(
            chat_read(h, Some(gid(2)))
                .await?
                .ok_or("missing new recipient")?
                .guild_id,
            gid(2)
        );
        Ok(())
    })
}
#[test]
fn guild_chat_native_committed_exclusion_removes_recipient_observation() -> TestResult {
    text_run("guild_chat_exclude", async |h| {
        historical_character(h, 90, id(WORLD)).await?;
        initial_guild(h, 1, 90).await?;
        initial_member(h, 1, 0, 3).await?;
        let leader = historical_session(h, 90).await?;
        guard(h).await?;
        assert!(chat_read(h, Some(gid(1))).await?.is_some());
        let mut exclude = request(1, 1, GuildCommand::Exclude(character(0)));
        exclude.command = CommandRef::new(leader.game_session_id, CommandId::new(1)?);
        assert_eq!(
            result(apply(h, exclude, leader).await?).result,
            GuildResult::Ok
        );
        assert!(chat_read(h, Some(gid(1))).await?.is_none());
        assert!(chat_read(h, None).await?.is_none());
        Ok(())
    })
}
#[test]
fn guild_chat_native_both_read_boundaries_require_current_session_lease_scope_node() -> TestResult {
    text_run("guild_chat_fences", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let wrong_node = fixture::register(&h.root, 2).await?;
        assert!(
            h.root
                .read_guild_chat_membership(&authority, &h.node, fence()?)
                .await
                .map_err(debug)?
                .is_some()
        );
        let before = text_snapshot(h).await?;
        for field in 0..8 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                2 => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => stale.character_id = character(999),
                4 => stale.game_session_id = GameSessionId::decode(&sid(999, 20))?,
                5 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            crate::foundation::WorldId::decode(&sid(999, 70))?,
                            channel_id,
                        );
                    }
                }
                6 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { world_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            world_id,
                            crate::foundation::ChannelId::decode(&sid(999, 71))?,
                        );
                    }
                }
                _ => {}
            }
            let node = if field == 7 { &wrong_node } else { &h.node };
            assert!(matches!(
                h.root
                    .read_guild_chat_membership(&authority, node, stale)
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert!(matches!(
                h.root
                    .read_guild_chat_recipient(&authority, node, stale, gid(1))
                    .await,
                Err(GuildError::NotCurrent)
            ));
        }
        assert_eq!(text_snapshot(h).await?, before);
        assert!(
            h.root
                .read_guild_chat_recipient(&authority, &h.node, fence()?, gid(1))
                .await
                .map_err(debug)?
                .is_some()
        );
        Ok(())
    })
}
#[test]
fn guild_chat_native_root_recovery_and_failed_reads_are_fail_closed() -> TestResult {
    text_run("guild_chat_recovery", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .read_guild_chat_membership(&authority, &h.node, fence()?)
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert!(matches!(
            other
                .read_guild_chat_recipient(&authority, &h.node, fence()?, gid(1))
                .await,
            Err(GuildError::NotCurrent)
        ));
        let mut stale = authority.record_for(&h.root).map_err(debug)?;
        stale.recovery_generation += 1;
        for payload in [None, Some(gid(1))] {
            let mut tx = h.pool.begin().await?;
            assert!(
                read_current_guild_chat_data(&mut tx, &stale, &h.node, &fence()?, payload)
                    .await
                    .is_err()
            );
            tx.rollback().await?;
        }
        assert!(chat_read(h, None).await?.is_some());
        // A real DB read failure is an error, never a permissive/empty successful proof.
        h.pool
            .execute("ALTER TABLE game_guild_members RENAME TO guild_chat_unavailable_members")
            .await?;
        assert!(
            h.root
                .read_guild_chat_membership(&authority, &h.node, fence()?)
                .await
                .is_err()
        );
        assert!(
            h.root
                .read_guild_chat_recipient(&authority, &h.node, fence()?, gid(1))
                .await
                .is_err()
        );
        h.pool
            .execute("ALTER TABLE guild_chat_unavailable_members RENAME TO game_guild_members")
            .await?;
        assert!(chat_read(h, Some(gid(1))).await?.is_some());
        Ok(())
    })
}
#[test]
fn guild_chat_native_disbanding_state_is_observed_without_delivery_policy() -> TestResult {
    text_run("guild_chat_state", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        assert_eq!(
            result(disband_apply(h, disband_request(1, fence()?, &name(1)), fence()?).await?)
                .result,
            GuildResult::Ok
        );
        let data = chat_read(h, None)
            .await?
            .ok_or("membership survives disband start")?;
        assert_eq!(data.state, GuildState::Disbanding);
        assert_eq!(chat_read(h, Some(gid(1))).await?, Some(data));
        // No CHAT delivery/custody authorization is returned or inferred here.
        Ok(())
    })
}
