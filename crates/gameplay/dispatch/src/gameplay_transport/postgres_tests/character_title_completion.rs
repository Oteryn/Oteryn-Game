//! Q2=B: a title requires every canonical member to reach its native final threshold.
use super::character_title_sources::{seed_completed, source_limits};
use super::cyclopedia::support::exchange;
use super::fixture::{Harness, id, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::domain::bosstiary::{BosstiaryBoss, kill_thresholds, progress};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::character_info::CharacterTitle;
use std::collections::BTreeSet;

#[test]
fn character_title_class_and_boss_completion_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "title completion scenario panicked")??;
    eprintln!("character_title_class_and_boss_completion_real_dispatch POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn seed_bosses(
    h: &Harness,
    bosses: &[BosstiaryBoss],
    incomplete: &BTreeSet<String>,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    for boss in bosses {
        let final_count = kill_thresholds(&boss.category).ok_or("category")?[2];
        let count = final_count - u64::from(incomplete.contains(&boss.key));
        let points = progress(&boss.category, count)
            .ok_or("boss progress")?
            .points;
        // Bosstiary kills do not advance CharacterRevision. Retain one immutable receipt
        // per occurrence, exactly as its native owner does; no title state is seeded.
        sqlx::query("INSERT INTO game_character_bosstiary_kill_receipts(character_id,occurrence_id,boss_key,category) SELECT encode($1,'hex')::uuid,game_character_uuid_v7(),$2,$3 FROM generate_series(1::bigint,$4::bigint)")
            .bind(id(41).as_slice()).bind(&boss.key).bind(&boss.category).bind(i64::try_from(count)?).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_bosstiary_progress(character_id,boss_key,kill_count,boss_points) VALUES(encode($1,'hex')::uuid,$2,$3,$4)")
            .bind(id(41).as_slice()).bind(&boss.key).bind(i64::try_from(count)?).bind(i64::from(points)).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn command(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    // This fixture drives serve_admitted without the node's root-maintenance loop.
    // Restore a retired holder before reconnect reconciliation reads durable NPC state.
    assert!(
        owner.root.maintain_ready_once().await.map_err(debug)?,
        "node maintenance must ready the holder before the next client command"
    );
    let next = admitted.continuity.next_command_id;
    exchange(owner, admitted, next, 113, payload).await
}

fn scenario() -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::connection::FirstEntryOutcome;
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "character_title_completion", true, false)
                .await?;
        prepare_current_session(&h).await?;
        let catalogue =
            crate::content::canonical_title_catalogue(source_limits()).map_err(debug)?;
        // The smallest admitted class keeps this real receipt-chain fixture bounded.
        let amphibic: Vec<_> = catalogue
            .bestiary
            .iter()
            .filter(|(_, class)| class == "Amphibic")
            .map(|(race, _)| (race.clone(), 0))
            .collect();
        assert_eq!(amphibic.len(), 11);
        seed_completed(&h, &amphibic[..amphibic.len() - 1]).await?;
        let incomplete: BTreeSet<_> = ["bane", "archfoe", "nemesis"]
            .into_iter()
            .map(|category| {
                catalogue
                    .bosstiary
                    .iter()
                    .find(|boss| boss.category == category)
                    .map(|boss| boss.key.clone())
                    .ok_or("canonical category")
            })
            .collect::<Result<_, _>>()?;
        seed_bosses(&h, &catalogue.bosstiary, &incomplete).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(current.item_fence());
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &crate::gameplay_transport::character_info::offered(&[], true),
            &[97],
        )
        .ok_or("capability")?;
        let titles = [
            CharacterTitle::Coldblooded,
            CharacterTitle::BossAnnihilator,
            CharacterTitle::BossEraser,
            CharacterTitle::BossExecutioner,
            CharacterTitle::BossSlayer,
        ];
        for (ordinal, title) in titles.into_iter().enumerate() {
            assert_eq!(
                command(&owner, &mut admitted, &[2, title.id()]).await?.0,
                CommandStatus::Rejected,
                "one missing entry or one kill below mastery must refuse {title:?}"
            );
            if ordinal == 0 {
                // A pass deadline can retire the sole holder after a rejected command.
                // Exercise that state deterministically, without slowing a title read.
                // Under CI load a background pass may still hold the sole holder, or its
                // deadline may already have retired it. Wait for a busy holder to return,
                // then close it; a holder that never returns is already retired.
                let settle = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    match h.root.try_acquire_ready() {
                        Ok(holder) => {
                            holder.close().await?;
                            break;
                        }
                        Err(crate::durability::DurabilityError::RootUnavailable)
                            if tokio::time::Instant::now() < settle =>
                        {
                            let now = tokio::time::Instant::now();
                            h.root
                                .wait_holder_retry(now, now + std::time::Duration::from_millis(100))
                                .await;
                        }
                        Err(crate::durability::DurabilityError::RootUnavailable) => break,
                        Err(error) => return Err(debug(error).into()),
                    }
                }
                h.root.request_ready();
                assert!(!h.root.is_ready(), "the fixture retired its holder");
            }
        }
        let before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_titles WHERE title BETWEEN 59 AND 84",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(before, 0);
        seed_completed(&h, &amphibic[amphibic.len() - 1..]).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("session")?;
        for (ordinal, boss) in catalogue
            .bosstiary
            .iter()
            .filter(|boss| incomplete.contains(&boss.key))
            .enumerate()
        {
            let occurrence =
                crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(
                    110 + u8::try_from(ordinal)?
                ))
                .map_err(debug)?;
            assert!(
                h.root
                    .credit_bosstiary_kill(&authority, &h.node, fence, occurrence, boss)
                    .await
                    .map_err(debug)?
            );
            assert!(
                !h.root
                    .credit_bosstiary_kill(&authority, &h.node, fence, occurrence, boss)
                    .await
                    .map_err(debug)?,
                "captured boss occurrence replay"
            );
        }
        assert_eq!(
            command(&owner, &mut admitted, &[0, 0, 0, 1]).await?.0,
            CommandStatus::Accepted
        );
        let stored: Vec<i16> = sqlx::query_scalar(
            "SELECT title FROM game_character_titles WHERE title BETWEEN 59 AND 84 ORDER BY title",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(stored, [61, 81, 82, 83, 84]);
        assert_eq!(
            command(
                &owner,
                &mut admitted,
                &[2, CharacterTitle::Executioner.id()]
            )
            .await?
            .0,
            CommandStatus::Rejected,
            "one full class is not all Bestiary entries"
        );
        for title in titles {
            assert_eq!(
                command(&owner, &mut admitted, &[2, title.id()]).await?,
                (CommandStatus::Accepted, vec![2, title.id()])
            );
        }
        assert_eq!(
            command(
                &owner,
                &mut admitted,
                &[3, CharacterTitle::BossExecutioner.id()]
            )
            .await?
            .0,
            CommandStatus::Accepted
        );
        let (status, payload) = command(&owner, &mut admitted, &[0, 0, 0, 1]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            payload.last().copied(),
            Some(CharacterTitle::BossExecutioner.id())
        );
        let after: Vec<i16> = sqlx::query_scalar(
            "SELECT title FROM game_character_titles WHERE title BETWEEN 59 AND 84 ORDER BY title",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            after, stored,
            "title replay/reload grants each catalogue row once"
        );

        // A requalification read must wait for the same Character root lock as a native
        // completion writer. Reading facts before this transaction could revoke a newer
        // conditional grant. Observe the actual PostgreSQL wait, then release the writer.
        let mut blocker = h.pool.begin().await?;
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(id(41).as_slice()).fetch_one(&mut *blocker).await?;
        let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await?;
        let monitor_pool = h.pool.clone();
        let release_when_blocked = tokio::spawn(async move {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                        .bind(blocker_pid).fetch_one(&monitor_pool).await?;
                    if waiting {
                        return Ok::<(), sqlx::Error>(());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            }).await??;
            blocker.commit().await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        let requalified = h
            .root
            .read_and_requalify_character_titles(
                &authority,
                &h.node,
                admitted.item_fence.ok_or("item fence")?,
                current.character_id,
                crate::gameplay_transport::character_info::unix_now_ms(),
                None,
            )
            .await
            .map_err(debug)?;
        release_when_blocked.await?.map_err(debug)?;
        assert_eq!(requalified.selected, Some(CharacterTitle::BossExecutioner));
        for title in titles {
            assert!(
                requalified
                    .earned
                    .iter()
                    .any(|(earned, _)| *earned == title)
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
