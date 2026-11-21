async fn revision(pool: &sqlx::PgPool) -> TestResult<u64> {
    let text: String =
        sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots")
            .fetch_one(pool)
            .await?;
    Ok(text.parse()?)
}

fn kill(tag: u8) -> TestResult<BestiaryKillRequest> {
    Ok(BestiaryKillRequest {
        occurrence: BestiaryKillOccurrence::from_bytes(id(tag)).map_err(debug)?,
        race: crate::domain::bestiary::BestiaryRace::new(
            "oteryn:creature.rat",
            "definition-r1",
            vec![5, 10, 20],
        )
        .map_err(debug)?,
        context: xp_request(tag)?.context,
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
    })
}

fn death(tag: u8) -> TestResult<CharacterDeathRequest<2>> {
    let xp = xp_request(tag)?;
    Ok(CharacterDeathRequest {
        public_cause: None,
        pvp_contributors: Vec::new(),
        occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
        context: xp.context,
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        policy: xp.policy,
        held_blessings: Vec::new(),
        death_cell: DeathCell {
            // W02 retains this Channel death fixture and every existing loss expectation.
            runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
                crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            ),
            spatial_position: vec![1, 2, 3, 7],
            map_revision: "map-1".into(),
        },
        respawn_position: b"temple:fixture".to_vec(),
    })
}

fn monk(tag: u8) -> TestResult<MonkStateSaveRequest> {
    Ok(MonkStateSaveRequest {
        occurrence: MonkStateSaveOccurrence::from_bytes(id(tag)).map_err(debug)?,
        state: DurableMonkState::new(3, 0).map_err(debug)?,
    })
}

/// A writer that bypasses the sequencer: one XP award straight at the current revision.
async fn bypass(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    tag: u8,
) -> TestResult {
    let current = revision(&harness.pool).await?;
    let outcome = harness
        .root
        .commit_character_experience(authority, &harness.node, fence(current)?, xp_request(tag)?)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, ExperienceCommitOutcome::Committed(_)),
        "{outcome:?}"
    );
    Ok(())
}

#[test]
fn concurrent_xp_and_charm_on_one_character_commit_in_sequence_without_mismatch() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "concurrent", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let facts = Facts::new(&[300], &[]);
        // Both requests carry the same stale revision: the slot, not the caller, picks it.
        let xp = async {
            let mut slot = sequencer.acquire(fence(1)?.character_id).await;
            slot.commit_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                xp_request(70)?,
                None,
            )
            .await
            .map_err(debug)
        };
        let charm = async {
            let mut slot = sequencer.acquire(fence(1)?.character_id).await;
            slot.commit_charm(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                unlock(71, "wound"),
                facts.clone(),
                None,
            )
            .await
            .map_err(debug)
        };
        let (xp, charm) = join_two(xp, charm).await;
        let (xp, charm) = (xp?, charm?);
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err(format!("XP did not commit: {xp:?}").into());
        };
        let CharmCommandOutcome::Committed(charm) = charm else {
            return Err(format!("charm did not commit: {charm:?}").into());
        };
        let mut chain = [
            (
                xp.original_character_revision.get(),
                xp.committed_character_revision.get(),
            ),
            (
                charm.original_character_revision.get(),
                charm.committed_character_revision.get(),
            ),
        ];
        chain.sort_unstable();
        assert_eq!(chain, [(1, 2), (2, 3)]);
        assert_eq!(revision(&harness.pool).await?, 3);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_death_chain_holds_the_slot_and_bestiary_takes_the_revision_xp_committed() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "chain", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let character = fence(1)?.character_id;
        let chain = async {
            let mut slot = sequencer.acquire(character).await;
            assert_eq!(slot.character_id(), character);
            let xp = slot
                .commit_experience(
                    &harness.root,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    xp_request(72)?,
                    None,
                )
                .await
                .map_err(debug)?;
            // Let the waiting writer run if the slot did not hold it back.
            tokio::task::yield_now().await;
            let bestiary = slot
                .commit_bestiary(
                    &harness.root,
                    &authority,
                    &harness.node,
                    fence(1)?,
                    kill(72)?,
                )
                .await
                .map_err(debug)?;
            TestResult::Ok((xp, bestiary))
        };
        let waiting = async {
            tokio::task::yield_now().await;
            let mut slot = sequencer.acquire(character).await;
            slot.commit_monk_state_save(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                monk(73)?,
            )
            .await
            .map_err(debug)
        };
        let (chain, waiting) = join_two(chain, waiting).await;
        let (xp, bestiary) = chain?;
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err(format!("XP did not commit: {xp:?}").into());
        };
        let BestiaryKillOutcome::Committed(bestiary) = bestiary else {
            return Err(format!("Bestiary did not commit: {bestiary:?}").into());
        };
        assert_eq!(xp.committed_character_revision.get(), 2);
        assert_eq!(
            bestiary.original_character_revision,
            xp.committed_character_revision
        );
        assert_eq!(bestiary.committed_character_revision.get(), 3);
        let MonkStateSaveOutcome::Committed(save) = waiting? else {
            return Err("the waiting monk save did not commit".into());
        };
        assert_eq!(
            save.original_character_revision.get(),
            3,
            "it ran after the chain"
        );
        assert_eq!(revision(&harness.pool).await?, 4);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_bestiary_mismatch_reloads_the_cursor_and_retries_once() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "bestiary_retry", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(fence(1)?.character_id).await;
        assert_eq!(
            slot.cursor(&harness.root, &authority)
                .await
                .map_err(debug)?
                .get(),
            1
        );
        bypass(&harness, &authority, 74).await?;
        let outcome = slot
            .commit_bestiary(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                kill(75)?,
            )
            .await
            .map_err(debug)?;
        let BestiaryKillOutcome::Committed(credited) = outcome else {
            return Err(format!("Bestiary did not commit: {outcome:?}").into());
        };
        assert_eq!(
            credited.original_character_revision.get(),
            2,
            "the reloaded cursor"
        );
        assert_eq!(revision(&harness.pool).await?, 3);

        // The retry is per request: a later request under the same slot gets its own one retry.
        bypass(&harness, &authority, 76).await?;
        bypass(&harness, &authority, 77).await?;
        let result = slot
            .commit_bestiary(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                kill(78)?,
            )
            .await;
        assert!(result.is_ok(), "{result:?}");
        drop(slot);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn revision_bound_writers_fail_closed_without_retry() -> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "fail_closed", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let facts = Facts::new(&[300], &[]);
        for (case, tag) in [("xp", 80), ("death", 82), ("charm", 84), ("monk", 86)] {
            let mut slot = sequencer.acquire(fence(1)?.character_id).await;
            slot.cursor(&harness.root, &authority)
                .await
                .map_err(debug)?;
            bypass(&harness, &authority, tag).await?;
            let before = revision(&harness.pool).await?;
            let base = fence(1)?;
            let mismatch = match case {
                "xp" => matches!(
                    slot.commit_experience(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        xp_request(tag + 1)?,
                        None
                    )
                    .await,
                    Err(CharacterProgressionError::CharacterRevisionMismatch)
                ),
                "death" => matches!(
                    slot.commit_death(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        death(tag + 1)?,
                        None
                    )
                    .await,
                    Err(CharacterProgressionError::CharacterRevisionMismatch)
                ),
                "charm" => matches!(
                    slot.commit_charm(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        unlock(tag + 1, "wound"),
                        facts.clone(),
                        None,
                    )
                    .await,
                    Err(CharmStateError::CharacterRevisionMismatch)
                ),
                _ => matches!(
                    slot.commit_monk_state_save(
                        &harness.root,
                        &authority,
                        &harness.node,
                        base,
                        monk(tag + 1)?
                    )
                    .await,
                    Err(CharacterProgressionError::CharacterRevisionMismatch)
                ),
            };
            assert!(mismatch, "{case}: a stale cursor must fail closed");
            assert_eq!(
                revision(&harness.pool).await?,
                before,
                "{case}: no retry committed"
            );
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn loaded_monk_values_do_not_pin_later_saves_and_exact_replay_does_not_revert_successor()
-> TestResult {
    run(async |admin| {
        let harness = Harness::create(admin, "monk_load_cursor_replay", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = fence(1)?.character_id;
        let observed_revision = revision(&harness.pool).await?;
        let observed_values = harness
            .root
            .read_character_monk_state(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(observed_revision, 1);
        assert_eq!(observed_values, DurableMonkState::default());
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character).await;
        let xp = slot
            .commit_experience(
                &harness.root,
                &authority,
                &harness.node,
                fence(observed_revision)?,
                xp_request(91)?,
                None,
            )
            .await
            .map_err(debug)?;
        let ExperienceCommitOutcome::Committed(xp) = xp else {
            return Err(format!("XP did not commit: {xp:?}").into());
        };
        assert_eq!(xp.committed_character_revision.get(), 2);
        let first_request = MonkStateSaveRequest {
            occurrence: MonkStateSaveOccurrence::from_bytes(id(92)).map_err(debug)?,
            state: DurableMonkState::new(3, 4_000_000).map_err(debug)?,
        };
        let first = slot
            .commit_monk_state_save(
                &harness.root,
                &authority,
                &harness.node,
                fence(observed_revision)?,
                first_request,
            )
            .await
            .map_err(debug)?;
        let MonkStateSaveOutcome::Committed(first) = first else {
            return Err(format!("first monk save did not commit: {first:?}").into());
        };
        assert_eq!(first.original_character_revision.get(), 2);
        assert_eq!(first.committed_character_revision.get(), 3);
        let successor_request = MonkStateSaveRequest {
            occurrence: MonkStateSaveOccurrence::from_bytes(id(93)).map_err(debug)?,
            state: DurableMonkState::new(4, 2_000_000).map_err(debug)?,
        };
        let successor = slot
            .commit_monk_state_save(
                &harness.root,
                &authority,
                &harness.node,
                fence(observed_revision)?,
                successor_request,
            )
            .await
            .map_err(debug)?;
        let MonkStateSaveOutcome::Committed(successor) = successor else {
            return Err(format!("successor monk save did not commit: {successor:?}").into());
        };
        assert_eq!(successor.original_character_revision.get(), 3);
        assert_eq!(successor.committed_character_revision.get(), 4);
        let unchanged = slot
            .commit_monk_state_save(
                &harness.root,
                &authority,
                &harness.node,
                fence(observed_revision)?,
                MonkStateSaveRequest {
                    occurrence: MonkStateSaveOccurrence::from_bytes(id(94)).map_err(debug)?,
                    state: successor_request.state,
                },
            )
            .await
            .map_err(debug)?;
        assert_eq!(unchanged, MonkStateSaveOutcome::Unchanged);
        assert_eq!(revision(&harness.pool).await?, 4);
        // Retained receipt is historical outcome evidence, never current actor authority.
        let replay = harness
            .root
            .commit_character_monk_state_save(
                &authority,
                &harness.node,
                fence(first.original_character_revision.get())?,
                first_request,
            )
            .await
            .map_err(debug)?;
        assert_eq!(replay, MonkStateSaveOutcome::AlreadyCommitted(first));
        let reconciled = harness
            .root
            .reconcile_character_monk_state_save(&authority, first_request.occurrence)
            .await
            .map_err(debug)?;
        assert_eq!(reconciled, Some(first));
        let reloaded = harness
            .root
            .read_character_monk_state(&authority, character)
            .await
            .map_err(debug)?;
        assert_eq!(reloaded, successor_request.state);
        assert_eq!(revision(&harness.pool).await?, 4);
        drop(slot);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn durable_death_clears_harmony_and_serene_and_receipt_readback_cannot_restore_them() -> TestResult
{
    run(async |admin| {
        let harness = Harness::create(admin, "monk_death_zero", true).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let character = fence(1)?.character_id;
        let sequencer = CharacterRevisionSequencer::new();
        let mut slot = sequencer.acquire(character).await;
        let request = MonkStateSaveRequest {
            occurrence: MonkStateSaveOccurrence::from_bytes(id(94)).map_err(debug)?,
            state: DurableMonkState::new(3, 4_000_000).map_err(debug)?,
        };
        let saved = slot
            .commit_monk_state_save(&harness.root, &authority, &harness.node, fence(1)?, request)
            .await
            .map_err(debug)?;
        assert!(matches!(saved, MonkStateSaveOutcome::Committed(_)));
        assert_eq!(revision(&harness.pool).await?, 2);
        assert_eq!(
            harness
                .root
                .read_character_monk_state(&authority, character)
                .await
                .map_err(debug)?,
            request.state
        );
        // Positive death uses D58/D68 ratio1/1 and a closed threshold oracle
        // covering the fixture before and after its capped loss. The older death
        // helper is used by a pre-calculation mismatch test and carries1/10.
        let mut death_request = death(95)?;
        death_request.policy.death_loss_denominator = 1;
        death_request.policy.thresholds = [
            LevelThreshold {
                level: 49,
                minimum_experience: ExactI64::new(0),
            },
            LevelThreshold {
                level: 50,
                minimum_experience: ExactI64::new(1000),
            },
        ];
        let dead = slot
            .commit_death(
                &harness.root,
                &authority,
                &harness.node,
                fence(1)?,
                death_request,
                None,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            dead,
            crate::durability::character_death::CharacterDeathOutcome::Committed(_)
        ));
        assert_eq!(revision(&harness.pool).await?, 3);
        assert_eq!(
            harness
                .root
                .read_character_monk_state(&authority, character)
                .await
                .map_err(debug)?,
            DurableMonkState::default()
        );
        let later = MonkStateSaveRequest {
            occurrence: MonkStateSaveOccurrence::from_bytes(id(96)).map_err(debug)?,
            state: DurableMonkState::new(4, 2_000_000).map_err(debug)?,
        };
        let refused = slot
            .commit_monk_state_save(&harness.root, &authority, &harness.node, fence(1)?, later)
            .await;
        assert!(
            matches!(refused, Err(CharacterProgressionError::RespawnPending)),
            "{refused:?}"
        );
        assert!(
            harness
                .root
                .reconcile_character_monk_state_save(&authority, request.occurrence)
                .await
                .map_err(debug)?
                .is_some()
        );
        assert_eq!(revision(&harness.pool).await?, 3);
        assert_eq!(
            harness
                .root
                .read_character_monk_state(&authority, character)
                .await
                .map_err(debug)?,
            DurableMonkState::default()
        );
        drop(slot);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
