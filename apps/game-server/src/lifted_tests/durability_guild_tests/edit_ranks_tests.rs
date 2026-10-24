use super::*;
#[test]
fn guild_edit_ranks_native_data_bounds_and_ordered_binding() -> TestResult {
    let common = request(1, 1, GuildCommand::EditRanks);
    for names in [vec!["Alpha".into(); 2], vec!["Alpha".into(); 21]] {
        assert!(matches!(
            GuildEditRanksRequest::new(common.clone(), 1, names),
            Err(GuildError::InvalidInput)
        ));
    }
    for invalid in ["", "Al\0pha"] {
        assert!(matches!(
            GuildEditRanksRequest::new(
                common.clone(),
                1,
                vec![invalid.into(), "Beta".into(), "Gamma".into()]
            ),
            Err(GuildError::InvalidInput)
        ));
    }
    assert!(
        GuildEditRanksRequest::new(
            common.clone(),
            0,
            vec!["Alpha".into(), "Beta".into(), "Gamma".into()]
        )
        .is_err()
    );
    assert!(
        GuildEditRanksRequest::new(
            request(1, 1, GuildCommand::Leave),
            1,
            vec!["Alpha".into(), "Beta".into(), "Gamma".into()]
        )
        .is_err()
    );
    let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
    let first = edit_ranks_binding(&fence()?, &req).map_err(debug)?;
    for field in 0..5 {
        let mut changed = req.clone();
        match field {
            0 => changed.names.swap(0, 1),
            1 => changed.names.push("Delta".into()),
            2 => changed.names[2] = "Delta".into(),
            3 => changed.expected_revision = 2,
            _ => changed.common.guild_id = gid(2),
        }
        assert_ne!(
            edit_ranks_binding(&fence()?, &changed).map_err(debug)?,
            first
        );
    }
    let max = GuildEditRanksRequest::new(common, 1, edit_ranks_large_names()).map_err(debug)?;
    assert_eq!(max.names.len(), 20);
    // Canonical uniqueness is an actual DB refusal, not an alternate Rust name normalizer.
    assert!(
        GuildEditRanksRequest::new(
            request(2, 1, GuildCommand::EditRanks),
            1,
            vec!["Al Dric".into(), "ALDRIC".into(), "Gamma".into()]
        )
        .is_ok()
    );
    Ok(())
}
#[test]
fn guild_edit_ranks_native_rename_cycles_and_exact_contiguous_set() -> TestResult {
    text_run("guild_edit_cycle", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let untouched = edit_ranks_other_fields(h).await?;
        let foreign = edit_ranks_rows(h, 2).await?;
        let characters = resign_resource_character_revisions(h).await?;
        let req = edit_ranks_request(1, 1, &["Vice Leader", "Member", "Visitor", "Leader"]);
        assert_eq!(
            result(edit_ranks_apply(h, req.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &req.names).await?;
        assert_eq!(revision(h, 1).await?, "2");
        assert_eq!(revision(h, 2).await?, "1");
        assert_eq!(edit_ranks_rows(h, 2).await?, foreign);
        for (who, rank) in [(0, 1), (91, 2), (92, 3), (93, 4), (94, 1), (95, 4)] {
            assert_eq!(rank_level(h, who).await?, rank);
        }
        assert_eq!(edit_ranks_other_fields(h).await?, untouched);
        assert_eq!(resign_resource_character_revisions(h).await?, characters);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_twenty_to_three_remaps_only_removed_levels() -> TestResult {
    text_run("guild_edit_remap", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let grow = GuildEditRanksRequest::new(
            request(1, 1, GuildCommand::EditRanks),
            1,
            edit_ranks_large_names(),
        )
        .map_err(debug)?;
        assert_eq!(
            result(edit_ranks_apply(h, grow.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &grow.names).await?;
        assert_eq!(rank_level(h, 93).await?, 4);
        // Feed the created level20 through the existing genuine SetRank writer, not raw SQL.
        assert_eq!(
            result(rank_apply(h, rank_request(2, 93, 2, 20), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let untouched = edit_ranks_other_fields(h).await?;
        let characters = resign_resource_character_revisions(h).await?;
        let foreign = edit_ranks_rows(h, 2).await?;
        let shrink = edit_ranks_request(3, 3, &["Rank T", "Rank A", "Rank B"]);
        assert_eq!(
            result(edit_ranks_apply(h, shrink.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &shrink.names).await?;
        for (who, rank) in [(0, 1), (91, 2), (92, 3), (93, 3), (94, 1), (95, 4)] {
            assert_eq!(rank_level(h, who).await?, rank);
        }
        assert_eq!(edit_ranks_other_fields(h).await?, untouched);
        assert_eq!(resign_resource_character_revisions(h).await?, characters);
        assert_eq!(edit_ranks_rows(h, 2).await?, foreign);
        assert_eq!(revision(h, 1).await?, "4");
        // Grow the genuine minimum3 snapshot again: removed historical level20 is not restored.
        let regrow = GuildEditRanksRequest::new(
            request(4, 1, GuildCommand::EditRanks),
            4,
            edit_ranks_large_names(),
        )
        .map_err(debug)?;
        assert_eq!(
            result(edit_ranks_apply(h, regrow.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &regrow.names).await?;
        assert_eq!(rank_level(h, 93).await?, 3);
        assert_eq!(rank_level(h, 0).await?, 1);
        assert_eq!(rank_level(h, 91).await?, 2);
        assert_eq!(edit_ranks_other_fields(h).await?, untouched);
        assert_eq!(resign_resource_character_revisions(h).await?, characters);
        assert_eq!(edit_ranks_rows(h, 2).await?, foreign);
        assert_eq!(revision(h, 1).await?, "5");
        assert_eq!(h.count("game_guild_operations").await?, 4);
        assert_eq!(h.count("game_guild_event_outbox").await?, 4);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_canonical_duplicates_and_stale_cas_are_atomic() -> TestResult {
    text_run("guild_edit_refusals", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let no_payload_before = text_snapshot(h).await?;
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &authority,
                    &h.node,
                    fence()?,
                    request(99, 1, GuildCommand::EditRanks)
                )
                .await,
            Err(GuildError::InvalidInput)
        ));
        assert_eq!(text_snapshot(h).await?, no_payload_before);
        for sql in [
            "SELECT game_guild_rank_names_valid(ARRAY['Alpha','Beta']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY(SELECT 'Rank '||chr(65+g) FROM generate_series(0,20) g))",
            "SELECT game_guild_rank_names_valid(ARRAY['Alpha',NULL,'Gamma']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY['Alpha','','Gamma']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY['Al Dric','ALDRIC','Gamma']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY[['Alpha'],['Beta'],['Gamma']]::text[])",
            "SELECT game_guild_rank_names_valid('[0:2]={Alpha,Beta,Gamma}'::text[])",
        ] {
            let valid: bool = sqlx::query_scalar(sql).fetch_one(&h.pool).await?;
            assert!(!valid, "accepted malformed rank vector: {sql}");
        }
        let unchanged = rank_mutation_snapshot(h).await?;
        for (n, names) in [
            (1, ["Al Dric", "ALDRIC", "Gamma"]),
            (2, ["Alpha", "Alpha", "Gamma"]),
        ] {
            assert_eq!(
                result(edit_ranks_apply(h, edit_ranks_request(n, 1, &names)).await?).result,
                GuildResult::NameTaken
            );
            assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        }
        assert_eq!(
            result(
                edit_ranks_apply(h, edit_ranks_request(3, 2, &["Alpha", "Beta", "Gamma"])).await?
            )
            .result,
            GuildResult::StaleRevision
        );
        assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        assert_eq!(revision(h, 1).await?, "1");
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_current_guild_leader_and_actual_resign_roles() -> TestResult {
    text_run("guild_edit_roles", async |h| {
        edit_ranks_fixture(h).await?;
        let actors = [
            historical_session(h, 91).await?,
            historical_session(h, 92).await?,
            historical_session(h, 94).await?,
        ];
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let unchanged = rank_mutation_snapshot(h).await?;
        for (index, actor) in actors.iter().enumerate() {
            let mut req =
                edit_ranks_request(u32::try_from(index + 1)?, 1, &["Alpha", "Beta", "Gamma"]);
            req.common.command = CommandRef::new(
                actor.game_session_id,
                CommandId::new(u64::try_from(index + 1)?)?,
            );
            assert_eq!(
                result(
                    h.root
                        .commit_guild_edit_ranks(&authority, &h.node, *actor, req)
                        .await
                        .map_err(debug)?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        }
        assert_eq!(
            result(resign_apply(h, resign_request(4, 91, 1), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let unchanged = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(
                edit_ranks_apply(h, edit_ranks_request(5, 2, &["Alpha", "Beta", "Gamma"])).await?
            )
            .result,
            GuildResult::NotAllowed
        );
        assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        let mut new_leader = edit_ranks_request(6, 2, &["Alpha", "Beta", "Gamma"]);
        new_leader.common.command = CommandRef::new(actors[0].game_session_id, CommandId::new(6)?);
        assert_eq!(
            result(
                h.root
                    .commit_guild_edit_ranks(&authority, &h.node, actors[0], new_leader)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(rank_level(h, 91).await?, 1);
        assert_eq!(rank_level(h, 0).await?, 2);
        assert_eq!(revision(h, 1).await?, "3");
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_history_binds_name_order_count_and_revision() -> TestResult {
    text_run("guild_edit_replay", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma", "Delta"]);
        let first = result(edit_ranks_apply(h, req.clone()).await?);
        assert_eq!(first.result, GuildResult::Ok);
        assert_eq!(
            result(
                edit_ranks_apply(h, edit_ranks_request(2, 2, &["Gamma", "Beta", "Alpha"])).await?
            )
            .result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(999)?;
        let replay = other
            .commit_guild_edit_ranks(&authority, &h.node, stale, req.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), first);
        assert_eq!(text_snapshot(h).await?, before);
        for field in 0..6 {
            let mut changed = req.clone();
            match field {
                0 => changed.names.swap(0, 1),
                1 => {
                    changed.names.pop();
                }
                2 => changed.names[0] = "Epsilon".into(),
                3 => changed.expected_revision = 2,
                4 => changed.common.guild_id = gid(2),
                _ => changed.common.operation = GuildCommand::SetMessage,
            }
            assert!(matches!(
                other
                    .commit_guild_edit_ranks(&authority, &h.node, fence()?, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_operations").await?, 2);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_lifecycle_does_not_cure_or_extend_deadlines() -> TestResult {
    for state in [1_i16, 2, 3, 4] {
        text_run("guild_edit_lifecycle", async |h| {
            initial_guild(h, 1, 0).await?;
            sqlx::query("UPDATE game_guilds SET state=$1,formation_deadline=1,vice_deficit_since=$2,premium_deficit_since=$3")
                .bind(state).bind(if state == 1 { None } else { Some(1_i64) })
                .bind(if state == 1 { None } else { Some(7_i64) }).execute(&h.pool).await?;
            if state == 4 {
                h.pool
                    .execute(
                        "DELETE FROM game_account_guild_leadership;DELETE FROM game_guild_members;",
                    )
                    .await?;
            }
            guard(h).await?;
            let unchanged = edit_ranks_other_fields(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
            assert_eq!(
                result(edit_ranks_apply(h, req.clone()).await?).result,
                if state <= 2 {
                    GuildResult::Ok
                } else {
                    GuildResult::GuildDisbanding
                }
            );
            assert_eq!(edit_ranks_other_fields(h).await?, unchanged);
            if state <= 2 {
                edit_ranks_expect(h, &req.names).await?;
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
            }
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(state <= 2)
            );
            Ok(())
        })?;
    }
    Ok(())
}
#[test]
fn guild_edit_ranks_native_two_root_cas_and_occurrence_serialization() -> TestResult {
    text_run("guild_edit_races", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let left = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let right = edit_ranks_request(2, 1, &["Delta", "Epsilon", "Zeta"]);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_edit_ranks(&a, &h.node, fence()?, left.clone()),
            other.commit_guild_edit_ranks(&b, &h.node, fence()?, right.clone()),
        )
        .await;
        let (x, y) = (result(x.map_err(debug)?), result(y.map_err(debug)?));
        assert_eq!(
            usize::from(x.result == GuildResult::Ok) + usize::from(y.result == GuildResult::Ok),
            1
        );
        assert_eq!(
            usize::from(x.result == GuildResult::StaleRevision)
                + usize::from(y.result == GuildResult::StaleRevision),
            1
        );
        edit_ranks_expect(
            h,
            if x.result == GuildResult::Ok {
                &left.names
            } else {
                &right.names
            },
        )
        .await?;
        let same = edit_ranks_request(3, 2, &["Eta", "Theta", "Iota", "Kappa"]);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_edit_ranks(&a, &h.node, fence()?, same.clone()),
            other.commit_guild_edit_ranks(&b, &h.node, fence()?, same.clone()),
        )
        .await;
        let (x, y) = (x.map_err(debug)?, y.map_err(debug)?);
        assert_eq!(
            usize::from(matches!(&x, GuildOutcome::Committed(_)))
                + usize::from(matches!(&y, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(x), result(y));
        edit_ranks_expect(h, &same.names).await?;
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_closed_sql_and_after_remap_rollback() -> TestResult {
    text_run("guild_edit_atomic", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let before = text_snapshot(h).await?;
        for sql in [
            "UPDATE game_guild_ranks SET name='Forged' WHERE level=3",
            "DELETE FROM game_guild_ranks WHERE level=4",
            "INSERT INTO game_guild_ranks(guild_id,level,name) SELECT guild_id,5,'Forged' FROM game_guilds",
            "UPDATE game_guild_members SET rank_level=3 WHERE rank_level=4",
        ] {
            let error = h
                .pool
                .execute(sql)
                .await
                .expect_err("unbound raw mutation accepted");
            assert_eq!(
                error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .as_deref(),
                Some("23514")
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        for (setup, cleanup) in [
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.title='forged';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER aaa_guild_edit_test_fault ON game_guild_members;DROP FUNCTION guild_edit_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF NEW.level=2 THEN NEW.name='Unbound';END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_fault BEFORE UPDATE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER aaa_guild_edit_test_fault ON game_guild_ranks;DROP FUNCTION guild_edit_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.level=5;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_fault BEFORE UPDATE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER aaa_guild_edit_test_fault ON game_guild_ranks;DROP FUNCTION guild_edit_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'native post-remap event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_edit_test_fault AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER guild_edit_test_fault ON game_guild_event_outbox;DROP FUNCTION guild_edit_test_fault();",
            ),
        ] {
            h.pool.execute(setup).await?;
            assert!(edit_ranks_apply(h, req.clone()).await.is_err());
            assert_eq!(text_snapshot(h).await?, before);
            assert_eq!(rank_level(h, 93).await?, 4);
            h.pool.execute(cleanup).await?;
        }
        let outcome = edit_ranks_apply(h, req.clone()).await?;
        assert!(matches!(&outcome, GuildOutcome::Committed(_)));
        assert_eq!(result(outcome).result, GuildResult::Ok);
        edit_ranks_expect(h, &req.names).await?;
        assert_eq!(rank_level(h, 93).await?, 3);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })?;
    text_run("guild_edit_overflow", async |h| {
        edit_ranks_fixture(h).await?;
        sqlx::query("UPDATE game_guilds SET revision=18446744073709551615 WHERE guild_id=encode($1,'hex')::uuid").bind(gid(1).as_bytes().as_slice()).execute(&h.pool).await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        assert!(
            edit_ranks_apply(
                h,
                edit_ranks_request(1, u64::MAX, &["Alpha", "Beta", "Gamma"])
            )
            .await
            .is_err()
        );
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_real_actor_fences_and_cross_root_authority() -> TestResult {
    text_run("guild_edit_fences", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let before = text_snapshot(h).await?;
        let wrong = fixture::register(&h.root, 2).await?;
        let recovery = authority.record_for(&h.root).map_err(debug)?;
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
            let node = if field == 7 { &wrong } else { &h.node };
            let mut case_req = req.clone();
            if field == 4 {
                case_req.common.command =
                    CommandRef::new(stale.game_session_id, req.common.command.command_id());
            }
            assert!(matches!(
                h.root
                    .commit_guild_edit_ranks(&authority, node, stale, case_req.clone())
                    .await,
                Err(GuildError::NotCurrent)
            ));
            let write = EditRanksWrite {
                node: node.clone(),
                fence: stale,
                edit_ranks: case_req.clone(),
                binding: edit_ranks_binding(&stale, &case_req).map_err(debug)?,
            };
            let mut tx = h.pool.begin().await?;
            assert!(matches!(
                write.execute(&mut tx, &recovery).await,
                Err(GuildError::NotCurrent)
            ));
            tx.rollback().await?;
            assert_eq!(text_snapshot(h).await?, before);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_edit_ranks(&authority, &h.node, fence()?, req.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        // The same real capability works before actual revocation, ruling out a broken fixture.
        assert_eq!(
            result(
                h.root
                    .commit_guild_edit_ranks(&authority, &h.node, fence()?, req)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let req = edit_ranks_request(2, 2, &["Alpha", "Beta", "Gamma", "Delta"]);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        assert!(matches!(
            writer
                .submit(&AssignmentRequest {
                    operation_key: OperationKey::from_bytes([90; 32]),
                    actor: ControlActor::new(&actor).map_err(debug)?,
                    command: AssignmentCommand::Revoke {
                        scope: fence()?.runtime_scope,
                        predecessor
                    },
                })
                .await
                .map_err(debug)?,
            AssignmentOutcome::Committed(_)
        ));
        let actual = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(actual.state, AssignmentState::Revoked);
        assert_eq!(actual.ownership_generation, 2);
        assert!(matches!(
            h.root
                .commit_guild_edit_ranks(&authority, &h.node, fence()?, req.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        let write = EditRanksWrite {
            node: h.node.clone(),
            fence: fence()?,
            edit_ranks: req.clone(),
            binding: edit_ranks_binding(&fence()?, &req).map_err(debug)?,
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            write.execute(&mut tx, &recovery).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_recovery_precedes_fresh_write_and_historical_replay() -> TestResult {
    text_run("guild_edit_recovery", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut stale = authority.record_for(&h.root).map_err(debug)?;
        stale.recovery_generation += 1;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let write = EditRanksWrite {
            node: h.node.clone(),
            fence: fence()?,
            edit_ranks: req.clone(),
            binding: edit_ranks_binding(&fence()?, &req).map_err(debug)?,
        };
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &stale).await.is_err());
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(
            result(edit_ranks_apply(h, req).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &stale).await.is_err());
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}
#[test]
fn guild_edit_ranks_native_forged_rank_set_and_closed_event_proofs_rollback() -> TestResult {
    text_run("guild_edit_proofs", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let before = text_snapshot(h).await?;
        // Corrupt only newly proposed rows, never overwrite an immutable committed receipt.
        h.pool.execute("CREATE TABLE guild_edit_test_payload(payload BYTEA NOT NULL);CREATE FUNCTION guild_edit_test_payload_op() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT sha256(payload) INTO NEW.audit_sha256 FROM guild_edit_test_payload;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_payload_op BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_edit_test_payload_op();CREATE FUNCTION guild_edit_test_payload_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT payload INTO NEW.payload FROM guild_edit_test_payload;NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_payload_event BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_edit_test_payload_event();").await?;
        let valid = edit_ranks_test_audit(&req)?;
        let mut altered_values = Vec::new();
        let mut reordered = valid.clone();
        reordered["names"] = serde_json::json!(["Beta", "Alpha", "Gamma"]);
        altered_values.push(reordered);
        let mut truncated = valid.clone();
        truncated["names"] = serde_json::json!(["Alpha", "Beta"]);
        altered_values.push(truncated);
        let mut replaced = valid.clone();
        replaced["names"] = serde_json::json!(["Alpha", "Beta", "Delta"]);
        altered_values.push(replaced);
        let mut wrong_cas = valid.clone();
        wrong_cas["expected_revision"] = serde_json::json!(2);
        altered_values.push(wrong_cas);
        let mut missing = valid.clone();
        missing
            .as_object_mut()
            .ok_or("audit not object")?
            .remove("names");
        altered_values.push(missing);
        let mut extra = valid;
        extra["caller_is_leader"] = serde_json::json!(true);
        altered_values.push(extra);
        for altered in altered_values {
            sqlx::query("DELETE FROM guild_edit_test_payload")
                .execute(&h.pool)
                .await?;
            sqlx::query("INSERT INTO guild_edit_test_payload VALUES($1)")
                .bind(serde_json::to_vec(&altered)?)
                .execute(&h.pool)
                .await?;
            assert!(edit_ranks_apply(h, req.clone()).await.is_err());
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER aaa_guild_edit_test_payload_op ON game_guild_operations;DROP TRIGGER aaa_guild_edit_test_payload_event ON game_guild_event_outbox;DROP FUNCTION guild_edit_test_payload_op();DROP FUNCTION guild_edit_test_payload_event();DROP TABLE guild_edit_test_payload;").await?;
        // Same expected revision and actual final rows must still bind the exact ordered vector.
        h.pool.execute("CREATE FUNCTION guild_edit_test_names() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.edit_rank_names=ARRAY['Beta','Alpha','Gamma'];RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_names BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_edit_test_names();").await?;
        assert!(edit_ranks_apply(h, req.clone()).await.is_err());
        assert_eq!(text_snapshot(h).await?, before);
        h.pool.execute("DROP TRIGGER aaa_guild_edit_test_names ON game_guild_operations;DROP FUNCTION guild_edit_test_names();").await?;
        assert_eq!(
            result(edit_ranks_apply(h, req).await?).result,
            GuildResult::Ok
        );
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}
