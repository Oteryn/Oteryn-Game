use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, fence, id, pvp_source_death_fixture, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::character_death::PublicDeathCause;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::{
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::FirstEntryOutcome,
    death_ml_loss_postgres::death as death_request,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    cyclopedia_character::kill_credits::{COMMAND_TYPE_PLAYER_KILL_CREDITS, decode_reply},
    death_protection::{
        COMMAND_TYPE_DEATH_OCCURRENCE, DeathOccurrence, DeathOccurrenceReply, FinalBlowKind,
    },
};

#[test]
fn combat_occurrences_creature_and_environment_final_blows_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            run(async |admin| {
                for (cause, kind) in [
                    (
                        PublicDeathCause::creature("oteryn:creature.dragon", "Dragon"),
                        FinalBlowKind::Creature,
                    ),
                    (
                        Some(PublicDeathCause::UnidentifiedCreature),
                        FinalBlowKind::Creature,
                    ),
                    (None, FinalBlowKind::Environment),
                ] {
                    reset();
                    let h = Harness::create_with_admission_seed(
                        admin.clone(),
                        "pve_occurrences",
                        true,
                        false,
                    )
                    .await?;
                    prepare_current_session(&h).await?;
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let deps = Dependencies::new()?;
                    deps.runtime
                        .lock()
                        .await
                        .initialize_first_entry_position(deps.actor)
                        .map_err(debug)?;
                    let owner = deps.owner(&h, &authority);
                    deps.initialize(&owner).await;
                    let mut current = fence(1)?;
                    current.character_lease_generation = 2;
                    let mut death = death_request(90)?;
                    death.public_cause = cause;
                    let death = owner
                        .settle_source_player_death(current, death, None, None)
                        .await
                        .map_err(debug)?;
                    let mut after_death = current;
                    after_death.expected_character_revision = death.committed_character_revision;
                    assert!(
                        h.root
                            .consume_pending_respawn(
                                &authority,
                                &h.node,
                                after_death,
                                Some(death.occurrence)
                            )
                            .await
                            .map_err(debug)?
                            .is_some()
                    );
                    let mut admitted = deps.admitted;
                    admitted.first_entry = FirstEntryOutcome::Positioned;
                    admitted.item_fence = Some(current.item_fence());
                    admitted.continuity.selected_capabilities =
                        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[382])
                            .ok_or("legacy selection")?;
                    assert_eq!(
                        exchange(&owner, &mut admitted, 1, COMMAND_TYPE_DEATH_OCCURRENCE, &[])
                            .await?,
                        (CommandStatus::Rejected, vec![]),
                    );
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[382, 595],
                    )
                    .ok_or("occurrence selection")?;
                    let (status, payload) =
                        exchange(&owner, &mut admitted, 2, COMMAND_TYPE_DEATH_OCCURRENCE, &[])
                            .await?;
                    assert_eq!(status, CommandStatus::Accepted);
                    assert_eq!(
                        DeathOccurrenceReply::decode(&payload),
                        Some(DeathOccurrenceReply(Some(DeathOccurrence {
                            occurrence: *death.occurrence.as_bytes(),
                            final_blow: kind
                        })))
                    );
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    h.cleanup().await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "PvE occurrence worker panicked")??;
    Ok(())
}

#[test]
fn combat_occurrences_justified_killer_and_assist_share_victim_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            run(async |admin| {
                for local_final_blow in [true, false] {
                    player_credit_scenario(admin.clone(), local_final_blow).await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "player credit worker panicked")??;
    Ok(())
}

async fn player_credit_scenario(admin: String, local_final_blow: bool) -> TestResult {
    use super::appearance_change_postgres_fixtures::observer_visibility_tests as observer;
    reset();
    let h =
        Harness::create_with_admission_seed(admin, "pvp_credit_occurrences", true, false).await?;
    prepare_current_session(&h).await?;
    h.seed_pvp_target().await?;
    observer::admit(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let owner = deps.owner(&h, &authority);
    deps.initialize(&owner).await;
    let victim = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let victim_fence = owner
        .current_quest_fence(victim)
        .await
        .map_err(debug)?
        .ok_or("victim fence")?;
    let mut request = death_request(90)?;
    request.public_cause = Some(PublicDeathCause::Player {
        character_id: if local_final_blow { id(41) } else { id(60) },
    });
    pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
    sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
        .bind(id(101).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
    for (actor, final_blow) in [(41, local_final_blow), (60, !local_final_blow)] {
        let (kind, compact) = if final_blow {
            (PvpContributorKind::Damage, 1)
        } else {
            (PvpContributorKind::Assist, 2)
        };
        pvp_source_death_fixture::seed_ledger(
            &h.pool,
            101,
            actor,
            true,
            Some(compact),
            "0 seconds",
        )
        .await?;
        request.pvp_contributors.push(PvpDeathContributor {
            character_id: CharacterId::from_bytes(id(actor)).map_err(debug)?,
            kind,
            unjustified: false,
        });
    }
    let death = owner
        .settle_source_player_death(victim_fence, request.clone(), None, None)
        .await
        .map_err(debug)?;
    assert_eq!(
        owner
            .settle_source_player_death(victim_fence, request, None, None)
            .await
            .map_err(debug)?,
        death
    );
    let penalties: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_unjustified_points")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(penalties, 0);
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[382, 595])
            .ok_or("credit selection")?;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("viewer fence")?
            .item_fence(),
    );
    let (status, payload) = exchange(
        &owner,
        &mut admitted,
        1,
        COMMAND_TYPE_PLAYER_KILL_CREDITS,
        &[0, 0, 50],
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let rows = decode_reply(&payload).ok_or("credit reply")?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].occurrence, *death.occurrence.as_bytes());
    assert_eq!(rows[0].victim, id(101));
    assert_eq!(rows[0].final_blow, local_final_blow);
    let character = CharacterId::from_bytes(id(41)).map_err(debug)?;
    let committed_at: i64 = sqlx::query_scalar(
        "SELECT committed_at FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid",
    )
    .bind(death.occurrence.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let summary = h.root.read_character_info(
        &authority,
        character,
        character,
        oteryn_protocol_oteryn::character_info::PageRequest { offset: 0, limit: 1 },
        committed_at,
    ).await.map_err(debug)?;
    assert_eq!(summary.get(67..69), Some([1, 0].as_slice()), "Character Info counts the justified kill credit");
    let late_credit = sqlx::query(
        "INSERT INTO game_character_player_kill_credits
           (character_id, death_occurrence_id, committed_at, final_blow)
         SELECT character_id, death_occurrence_id, committed_at, false
         FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid",
    )
    .bind(death.occurrence.as_bytes().as_slice())
    .execute(&h.pool)
    .await;
    assert!(matches!(late_credit, Err(error)
        if error.as_database_error().and_then(|error| error.code()).as_deref() == Some("23514")));
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
