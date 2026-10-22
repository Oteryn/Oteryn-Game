use super::*;
#[test]
fn found_defaults_and_creation_audit_are_closed() -> TestResult {
    assert_eq!(FORMATION_LIFETIME_MS, 259_200_000);
    assert_eq!(
        DEFAULT_RANKS,
        [(1, "Leader"), (2, "Vice Leader"), (3, "Member")]
    );
    assert!(GuildName::parse(&"a".repeat(29)).is_ok());
    assert!(GuildName::parse(&"a".repeat(30)).is_err());
    let audit = GuildAuditProposal {
        version: 1,
        occurrence: sid(1, 10),
        transaction_id: sid(1, 11),
        event_id: sid(1, 12),
        guild_id: *gid(1).as_bytes(),
        world_id: id(WORLD),
        actor: id(CHARACTER),
        target: id(CHARACTER),
        action: 6,
        previous_revision: 0,
        revision: 1,
        rank_before: None,
        rank_after: Some(1),
        removed_invitations: 0,
        occurred_at_unix_ms: 1,
    };
    assert!(audit.checked_bytes().is_some());
    let mut wrong = audit.clone();
    wrong.previous_revision = 1;
    wrong.revision = 2;
    assert!(wrong.checked_bytes().is_none());
    wrong = audit.clone();
    wrong.rank_after = Some(2);
    assert!(wrong.checked_bytes().is_none());
    wrong = audit.clone();
    wrong.target = sid(99, 1);
    assert!(wrong.checked_bytes().is_none());
    wrong = audit;
    wrong.action = 3;
    assert!(wrong.checked_bytes().is_none());
    assert!(!crate::durability::bank::fee_payer_is_junior(character(0)));
    assert!(crate::durability::bank::is_junior(
        crate::durability::bank::StarterIslandFact::OnIsland
    ));
    Ok(())
}
#[test]
fn native_found_creates_exact_snapshot_then_runs_all_five_membership_commands() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_flow").await?;prepare(&h).await?;
        historical_character(&h,90,id(WORLD)).await?;historical_character(&h,91,id(WORLD)).await?;
        let target_fence=historical_session(&h,90).await?;guard(&h).await?;
        let before_revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        // Character names and Guild names intentionally occupy separate namespaces.
        let character_name:String=sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        let coordinator=pre_delivery(&h.root);let req=found_request(1,1,&character_name);
        let receipt=result(found(&h,req.clone(),fence()?,&coordinator).await?);assert_eq!(receipt.result,GuildResult::Ok);assert_eq!((receipt.previous_revision,receipt.revision),(Some(0),Some(1)));
        let initial:Vec<(i16,String)>=sqlx::query_as("SELECT level,name FROM game_guild_ranks ORDER BY level").fetch_all(&h.pool).await?;
        assert_eq!(initial,vec![(1,"Leader".into()),(2,"Vice Leader".into()),(3,"Member".into())]);
        let row=sqlx::query("SELECT state,revision::text,formation_deadline-created_at AS duration,vice_deficit_since,premium_deficit_since,motd FROM game_guilds").fetch_one(&h.pool).await?;
        assert_eq!(row.try_get::<i16,_>("state")?,1);assert_eq!(row.try_get::<String,_>("revision")?,"1");assert_eq!(row.try_get::<i64,_>("duration")?,259_200_000);assert_eq!(row.try_get::<Option<i64>,_>("vice_deficit_since")?,None);assert_eq!(row.try_get::<Option<i64>,_>("premium_deficit_since")?,None);assert_eq!(row.try_get::<String,_>("motd")?,"");
        assert_eq!(h.count("game_account_guild_leadership").await?,1);assert_eq!(h.count("game_guild_members").await?,1);
        assert_eq!(result(apply(&h,request(2,1,GuildCommand::Invite(character(90))),fence()?).await?).result,GuildResult::Ok);
        let mut accept=request(3,1,GuildCommand::Accept);accept.command=CommandRef::new(target_fence.game_session_id,CommandId::new(3)?);
        assert_eq!(result(apply(&h,accept,target_fence).await?).result,GuildResult::Ok);
        let joined:i16=sqlx::query_scalar("SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid").bind(character(90).as_bytes().as_slice()).fetch_one(&h.pool).await?;assert_eq!(joined,3);
        let mut leave=request(4,1,GuildCommand::Leave);leave.command=CommandRef::new(target_fence.game_session_id,CommandId::new(4)?);
        assert_eq!(result(apply(&h,leave,target_fence).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(5,1,GuildCommand::Invite(character(90))),fence()?).await?).result,GuildResult::Ok);
        let mut accept=request(6,1,GuildCommand::Accept);accept.command=CommandRef::new(target_fence.game_session_id,CommandId::new(6)?);
        assert_eq!(result(apply(&h,accept,target_fence).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(7,1,GuildCommand::Exclude(character(90))),fence()?).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(8,1,GuildCommand::Invite(character(91))),fence()?).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(9,1,GuildCommand::Revoke(character(91))),fence()?).await?).result,GuildResult::Ok);
        let after_revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;assert_eq!(before_revision,after_revision);
        assert_eq!(h.count("game_guild_event_outbox").await?,9);assert_eq!(h.count("game_guild_operations").await?,9);
        h.cleanup().await
    })
}
#[test]
fn native_found_real_current_and_closed_premium_accounts() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for (case, expected) in [
            (0, GuildResult::Ok),
            (1, GuildResult::NotPremium),
            (2, GuildResult::NotPremium),
            (3, GuildResult::NotPremium),
        ] {
            let h = Harness::create(admin.clone(), "guild_found_premium").await?;
            prepare(&h).await?;
            guard(&h).await?;
            let (coordinator, consumer) = current_coordinator(
                &h,
                if case == 2 { account(90) } else { account(0) },
                case != 1,
            )
            .await?;
            if case == 3 {
                let ticket = consumer.ticket();
                consumer.pull_failed(account(0), ticket);
            }
            assert_eq!(
                result(
                    found(
                        &h,
                        found_request(1, 1, "Founded Premium"),
                        fence()?,
                        &coordinator
                    )
                    .await?
                )
                .result,
                expected
            );
            assert_eq!(
                h.count("game_guilds").await?,
                i64::from(expected == GuildResult::Ok)
            );
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(expected == GuildResult::Ok)
            );
            assert_eq!(
                h.count("game_premium_activation_latch").await?,
                if case == 1 { 0 } else { 1 }
            );
            h.cleanup().await?;
        }
        Ok(())
    })
}
#[test]
fn native_found_late_pre_delivery_refusal_rolls_back_savepoint_then_exact_retry() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_late").await?;prepare(&h).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_found_test_wrong_member() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.rank_level=2;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_found_test_wrong_member BEFORE INSERT ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_found_test_wrong_member();").await?;
        let before=found_footprint(&h).await?;let req=found_request(1,1,"Founded Rollback");let coordinator=pre_delivery(&h.root);
        let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator).await,Err(GuildError::Unavailable(_))));
        assert_eq!(found_footprint(&h).await?,before);assert_eq!(h.count("game_premium_activation_latch").await?,0);
        h.pool.execute("DROP TRIGGER aaa_guild_found_test_wrong_member ON game_guild_members;DROP FUNCTION guild_found_test_wrong_member();").await?;
        // A second single-field mutation remains a valid rank name under the generic schema,
        // so collect the real immediate Found-specific rank-guard message from server logs.
        // Neither this fixture trigger nor the generic rank-name CHECK raises that message.
        h.pool.execute("CREATE FUNCTION guild_found_test_wrong_rank_name() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF NEW.level=2 THEN NEW.name='Deputy';END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_found_test_wrong_rank_name BEFORE INSERT ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION guild_found_test_wrong_rank_name();").await?;
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator).await,Err(GuildError::Unavailable(_))));
        assert_eq!(found_footprint(&h).await?,before);
        h.pool.execute("DROP TRIGGER aaa_guild_found_test_wrong_rank_name ON game_guild_ranks;DROP FUNCTION guild_found_test_wrong_rank_name();").await?;
        // No operation survived: this exact fresh command really writes, rather than fast-replays.
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator).await.map_err(debug)?,GuildOutcome::Committed(_)));
        assert_eq!(h.count("game_guilds").await?,1);assert_eq!(h.count("game_guild_ranks").await?,3);assert_eq!(h.count("game_guild_event_outbox").await?,1);
        let closed=SpellPremiumCoordinator::new(Arc::default(),None,None,None);
        let saved=found_footprint(&h).await?;let mut stale=fence()?;stale.connection_generation=ConnectionGeneration::new(999)?;
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,stale,req.clone(),&closed).await.map_err(debug)?,GuildOutcome::Replayed(_)));assert_eq!(found_footprint(&h).await?,saved);
        let mut changed=req.clone();changed.name=GuildName::parse("Changed Request").unwrap();assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,changed,&closed).await,Err(GuildError::ConflictingOccurrence)));
        let mut reused=found_request(2,2,"Reused Command");reused.command=req.command;assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,reused,&closed).await,Err(GuildError::ConflictingOccurrence)));
        drop(authority);drop(seal);h.cleanup().await
    })
}
#[test]
fn native_found_shared_gate_transaction_commits_before_activation_latch() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_latch").await?;prepare(&h).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_found_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN PERFORM pg_advisory_xact_lock(901122);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_found_test_barrier BEFORE INSERT ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_found_test_barrier();").await?;
        let mut barrier=h.pool.begin().await?;sqlx::query("SELECT pg_advisory_xact_lock(901122)").execute(&mut *barrier).await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);
        let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let coordinator=pre_delivery(&h.root);let req=found_request(1,1,"Founded Before Latch");
        let (created,latched)=fixture::join_two(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator),async {
            wait_advisory(&h.pool,901122,false).await?;
            let latch_key=crate::durability::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY;
            wait_advisory(&h.pool,latch_key,true).await?;
            let (latched,released)=fixture::join_two(other.latch_premium_activation("guild-found-racing-activation",1,2),async {
                wait_advisory(&h.pool,latch_key,false).await?;barrier.rollback().await?;Ok::<_,Box<dyn std::error::Error>>(())
            }).await;released?;latched.map_err(|e| -> Box<dyn std::error::Error> { debug(e).into() })
        }).await;
        assert_eq!(result(created.map_err(debug)?).result,GuildResult::Ok);assert_eq!(latched?.activation_id,"guild-found-racing-activation");
        assert_eq!(h.count("game_guilds").await?,1);assert_eq!(h.count("game_premium_activation_latch").await?,1);
        // A new owner after the persisted switch cannot apply NotActivated, even with no clock.
        historical_character(&h,90,id(WORLD)).await?;let f=historical_session(&h,90).await?;
        let mut second=found_request(2,2,"Founded After Latch");second.command=CommandRef::new(f.game_session_id,CommandId::new(2)?);
        drop(authority);drop(seal);
        assert_eq!(result(found(&h,second,f,&pre_delivery(&h.root)).await?).result,GuildResult::NotPremium);
        assert!(matches!(found(&h,req,fence()?,&pre_delivery(&h.root)).await?,GuildOutcome::Replayed(_)));
        h.pool.execute("DROP TRIGGER aaa_guild_found_test_barrier ON game_guilds;DROP FUNCTION guild_found_test_barrier();").await?;
        drop(other);h.cleanup().await
    })
}
#[test]
fn native_found_name_membership_and_global_account_eligibility() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for case in 0..5 {
            let h=Harness::create(admin.clone(),"guild_found_eligibility").await?;prepare(&h).await?;
            let other_world=if case==2||case==4 {sid(900,40)}else{id(WORLD)};
            historical_character_account(&h,90,other_world,if case==2 {account(0)}else{account(90)}).await?;
            let old_name="Al Dric";
            sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,9999999999999,1,1)")
                .bind(gid(90).as_bytes().as_slice()).bind(other_world.as_slice()).bind(old_name).bind(if case==3 {4_i16}else{2}).execute(&h.pool).await?;
            for (level,value) in DEFAULT_RANKS {sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)").bind(gid(90).as_bytes().as_slice()).bind(i16::from(level)).bind(value).execute(&h.pool).await?;}
            if case!=3 {
                sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1)").bind(character(90).as_bytes().as_slice()).bind(gid(90).as_bytes().as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_account_guild_leadership VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)").bind(if case==2{account(0)}else{account(90)}.as_slice()).bind(gid(90).as_bytes().as_slice()).bind(character(90).as_bytes().as_slice()).execute(&h.pool).await?;
            }
            if case==1 {initial_member(&h,90,0,3).await?;}
            guard(&h).await?;
            let req=found_request(1,1,"ALDRIC");let expected=match case {0=>GuildResult::NameTaken,1=>GuildResult::AlreadyInGuild,2=>GuildResult::AccountHasPosition,_=>GuildResult::Ok};
            assert_eq!(result(found(&h,req,fence()?,&pre_delivery(&h.root)).await?).result,expected);
            assert_eq!(h.count("game_guild_operations").await?,1);assert_eq!(h.count("game_guild_event_outbox").await?,i64::from(expected==GuildResult::Ok));
            assert_eq!(h.count("game_guilds").await?,1+i64::from(expected==GuildResult::Ok));
            h.cleanup().await?;
        }Ok(())
    })
}
#[test]
fn native_found_current_authority_negatives_do_not_create_rows() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_found_fences").await?;
        prepare(&h).await?;
        guard(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let coordinator = pre_delivery(&h.root);
        let before = found_footprint(&h).await?;
        for field in 0..3 {
            let mut f = fence()?;
            match field {
                0 => f.connection_generation = ConnectionGeneration::new(99)?,
                1 => f.character_lease_generation = 99,
                _ => f.scope_ownership_generation = ScopeOwnershipGeneration::new(99)?,
            }
            assert!(matches!(
                h.root
                    .commit_guild_found(
                        &authority,
                        &h.node,
                        f,
                        found_request(field + 1, 1, "Fenced Found"),
                        &coordinator
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(found_footprint(&h).await?, before);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_found(
                    &authority,
                    &h.node,
                    fence()?,
                    found_request(4, 1, "Wrong Root"),
                    &pre_delivery(&other)
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        let wrong = fixture::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .commit_guild_found(
                    &authority,
                    &wrong,
                    fence()?,
                    found_request(5, 1, "Wrong Node"),
                    &coordinator
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(found_footprint(&h).await?, before);
        assert_eq!(
            result(
                h.root
                    .commit_guild_found(
                        &authority,
                        &h.node,
                        fence()?,
                        found_request(6, 1, "Healthy Found"),
                        &coordinator
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        drop(other);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
#[test]
fn native_found_two_root_name_account_and_occurrence_races() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for case in 0..3 {
            let h = Harness::create(admin.clone(), "guild_found_race").await?;
            prepare(&h).await?;
            let second_fence = if case == 0 {
                historical_character(&h, 90, id(WORLD)).await?;
                historical_session(&h, 90).await?
            } else {
                fence()?
            };
            guard(&h).await?;
            let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let b = other.open_character_authority(&seal).await.map_err(debug)?;
            let one = found_request(1, 1, "Racing Found");
            let mut two = if case == 2 {
                one.clone()
            } else {
                found_request(
                    2,
                    2,
                    if case == 0 {
                        "Racing Found"
                    } else {
                        "Other Found"
                    },
                )
            };
            if case == 0 {
                two.command = CommandRef::new(second_fence.game_session_id, CommandId::new(2)?);
            }
            let one_gate = pre_delivery(&h.root);
            let two_gate = pre_delivery(&other);
            let (one, two) = fixture::join_two(
                h.root
                    .commit_guild_found(&a, &h.node, fence()?, one, &one_gate),
                other.commit_guild_found(&b, &h.node, second_fence, two, &two_gate),
            )
            .await;
            let one = one.map_err(debug)?;
            let two = two.map_err(debug)?;
            if case == 2 {
                assert_eq!(
                    usize::from(matches!(&one, GuildOutcome::Committed(_)))
                        + usize::from(matches!(&two, GuildOutcome::Committed(_))),
                    1
                );
                assert_eq!(result(one), result(two));
            } else {
                let mut results = [result(one).result.code(), result(two).result.code()];
                results.sort();
                assert_eq!(
                    results,
                    [
                        0,
                        if case == 0 {
                            GuildResult::NameTaken.code()
                        } else {
                            GuildResult::AlreadyInGuild.code()
                        }
                    ]
                );
            }
            assert_eq!(h.count("game_guilds").await?, 1);
            assert_eq!(h.count("game_account_guild_leadership").await?, 1);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            // Native EXCLUSIVE admission relations serialize complete passes. Two simultaneous
            // current Characters of one Account are not fabricated: its native presence guard
            // has one Account PK. Cross-World Account eligibility is covered separately above.
            drop(b);
            drop(a);
            drop(seal);
            drop(other);
            h.cleanup().await?;
        }
        Ok(())
    })
}
#[test]
fn native_found_deferred_event_failure_is_atomic_then_fresh_root_retries_original() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_event").await?;prepare(&h).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_found_test_event_failure() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'Found fixture event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_found_test_event_failure AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_found_test_event_failure();").await?;
        let before=found_footprint(&h).await?;let req=found_request(1,1,"Atomic Found");let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(h.root.commit_guild_found(&a,&h.node,fence()?,req.clone(),&pre_delivery(&h.root)).await,Err(GuildError::Unavailable(_))));assert_eq!(found_footprint(&h).await?,before);
        h.pool.execute("DROP TRIGGER guild_found_test_event_failure ON game_guild_event_outbox;DROP FUNCTION guild_found_test_event_failure();").await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);let b=other.open_character_authority(&seal).await.map_err(debug)?;let gate=pre_delivery(&other);
        assert!(matches!(other.commit_guild_found(&b,&h.node,fence()?,req.clone(),&gate).await.map_err(debug)?,GuildOutcome::Committed(_)));
        assert!(matches!(other.commit_guild_found(&b,&h.node,fence()?,req,&gate).await.map_err(debug)?,GuildOutcome::Replayed(_)));
        assert_eq!(h.count("game_guilds").await?,1);assert_eq!(h.count("game_guild_ranks").await?,3);assert_eq!(h.count("game_guild_operations").await?,1);assert_eq!(h.count("game_guild_event_outbox").await?,1);
        let denied=sqlx::query("UPDATE game_guild_operations SET found_name='Mutated Found'").execute(&h.pool).await.expect_err("retained binding immutable");assert_eq!(denied.as_database_error().and_then(|e|e.code()).as_deref(),Some("23514"));
        drop(b);drop(other);drop(a);drop(seal);h.cleanup().await
    })
}
