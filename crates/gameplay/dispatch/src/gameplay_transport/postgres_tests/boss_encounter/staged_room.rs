//! Unqualified-content refusal remains covered alongside the accepted W02 writer privileges.
use super::*;

#[test]
fn boss_lever_staged_commands_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h =
                    Harness::create_with_admission_seed(admin, "boss_lever_staged", true, false)
                        .await?;
                prepare_current_session(&h).await?;
                eprintln!("Canonical PostgreSQL 18.6: staged boss lever dispatch refusal");
                for table in [
                    "game_boss_room_catalogue",
                    "game_boss_room_activities",
                    "game_boss_room_participants",
                    "game_boss_room_reward_receipts",
                ] {
                    let readable: bool = sqlx::query_scalar(
                        "SELECT has_table_privilege('oteryn_game_runtime',$1,'SELECT')",
                    )
                    .bind(table)
                    .fetch_one(&h.pool)
                    .await?;
                    assert!(readable, "missing staged read access: {table}");
                    for privilege in ["INSERT", "UPDATE", "DELETE", "TRUNCATE", "TRIGGER"] {
                        let writable: bool = sqlx::query_scalar(
                            "SELECT has_table_privilege('oteryn_game_runtime',$1,$2)",
                        )
                        .bind(table)
                        .bind(privilege)
                        .fetch_one(&h.pool)
                        .await?;
                        // W02 coordinator ANSWER A installs the complete fenced
                        // writer in 0628. Only its activity/member INSERT is now
                        // admitted; content, rewards and destructive rights are unchanged.
                        let expected = privilege == "INSERT"
                            && matches!(
                                table,
                                "game_boss_room_activities" | "game_boss_room_participants"
                            );
                        assert_eq!(
                            writable, expected,
                            "unexpected table privilege {privilege}: {table}"
                        );
                    }
                }
                let deps = Dependencies::new()?;
                let mut admitted = deps.admitted;
                admitted.first_entry = FirstEntryOutcome::Positioned;
                let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                    h.root.clone(),
                )
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
                admitted.item_fence = item_fence_of(current);
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let owner = deps.owner(&h, &authority);
                deps.initialize(&owner).await;
                owner
                    .runtime
                    .lock()
                    .await
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                let selected = SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[CAPABILITY_BOSS_ENCOUNTER_V1],
                )
                .ok_or("boss capability not offered")?;
                let unselected = admitted.continuity.selected_capabilities;
                assert!(selected.contains(CAPABILITY_BOSS_ENCOUNTER_V1));
                assert!(!unselected.contains(CAPABILITY_BOSS_ENCOUNTER_V1));
                // ProductionKey rejects a segment named "test". Use and validate
                // a genuine key shape so only its missing definition is negative.
                let missing_boss =
                    crate::content::ProductionKey::new("oteryn:boss.unavailable").map_err(debug)?;
                let defined: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_boss_room_catalogue WHERE boss_key=$1)",
                )
                .bind(missing_boss.as_str())
                .fetch_one(&h.pool)
                .await?;
                assert!(!defined);
                let before = participant_binding(&h, admitted).await?;
                let mut id = 1;
                for capabilities in [selected, unselected] {
                    admitted.continuity.selected_capabilities = capabilities;
                    for kind in [
                        COMMAND_TYPE_BOSS_LEVER,
                        COMMAND_TYPE_BOSS_DEFEATED,
                        COMMAND_TYPE_BOSS_TIMEOUT,
                    ] {
                        // A valid key and forged extra policy must both refuse without mutation.
                        for (payload, valid_key) in [
                            (cursor(missing_boss.as_str()), true),
                            (vec![10, 0, 16, 1], false),
                        ] {
                            let (status, bytes) =
                                exchange_kind(&owner, &mut admitted, id, kind, &payload).await?;
                            assert_eq!(status, CommandStatus::Rejected, "command {kind}");
                            if valid_key && capabilities.contains(CAPABILITY_BOSS_ENCOUNTER_V1) {
                                // W02 now routes valid requests to the fenced activity
                                // owner. This fixture has no authored boss definition,
                                // so its refusal carries the exact content reason.
                                assert_eq!(
                                    decode_boss_activity_reply(&bytes).map_err(debug)?,
                                    BossActivityReply::Refused(
                                        BossActivityRefusal::ContentUnavailable
                                    ),
                                    "command {kind} must report unavailable boss content"
                                );
                            } else {
                                // Missing capability and forged policy remain wire
                                // refusals; neither may reach the activity owner.
                                assert!(bytes.is_empty());
                            }
                            id += 1;
                        }
                    }
                }
                admitted.continuity.selected_capabilities = selected;
                // The refused activity commands must leave the current Channel read usable.
                let (status, bytes) = exchange(&owner, &mut admitted, id, &[]).await?;
                assert_eq!(status, CommandStatus::Accepted);
                assert!(page(&bytes)?.1.is_empty());
                assert_eq!(participant_binding(&h, admitted).await?, before);
                let rows: i64 = sqlx::query_scalar(
                    "SELECT (SELECT count(*) FROM game_boss_room_activities) + \
                            (SELECT count(*) FROM game_boss_room_participants) + \
                            (SELECT count(*) FROM game_boss_room_reward_receipts) + \
                            (SELECT count(*) FROM game_character_boss_cooldowns)",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(rows, 0);
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "staged boss lever dispatch panicked")??;
    Ok(())
}
