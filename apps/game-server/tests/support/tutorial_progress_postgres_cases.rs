//! TUTORIAL-1 durable tutorial progress (CP D1426, migration 0130) on PostgreSQL 17.6.
//! The parent harness provides the migrations and an independently seeded current session.
use crate::bestiary_postgres_harness::{
    Harness, SESSION, TestResult, configured_admin, debug, fence, id, runtime,
};
use crate::domain::tutorial::{
    ISLET_TEMPLE, TutorialActor, TutorialEvent, TutorialGate, TutorialProgress, TutorialStep, apply,
};
use crate::durability::DurabilityRoot;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::character_tutorial::{StoredTutorialProgress, TutorialProgressWrite};
use crate::foundation::{ConnectionGeneration, GameSessionId};

const NEW_PLAYER: TutorialActor = TutorialActor {
    vocationless: true,
    has_access: false,
};

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, true).await?;
        let outcome = body(&harness).await;
        harness.cleanup().await?;
        outcome
    })
}

const ARMED: TutorialProgress = TutorialProgress {
    step: TutorialStep::Island,
    hunting_hint_pending: true,
    corruptor_loot_pending: true,
};

async fn rows(harness: &Harness) -> TestResult<i64> {
    harness.count("game_character_tutorial_progress").await
}

/// A StartThais Character writes nothing; a Newhaven Character's steps are stored under the
/// session fence; a replay is idempotent, a stale writer gets a typed conflict; after a reconnect
/// (a new connection generation) and after a node restart the tutorial resumes from the stored
/// step, and the old connection reads nothing.
#[test]
fn tutorial_progress_resumes_after_reconnect_and_restart() -> TestResult {
    run("tutorial_resume", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let read = async |fence: CurrentCharacterGameplayFence| {
            h.root
                .read_character_tutorial_progress(&authority, &h.node, fence)
                .await
        };
        let write = async |fence, expected: u64, progress| {
            h.root
                .write_character_tutorial_progress(&authority, &h.node, fence, expected, progress)
                .await
        };
        assert_eq!(read(fence(1)?).await.map_err(debug)?, None);

        // StartThais: the gate is inert, so the login changes nothing and nothing is written.
        let thais = apply(TutorialGate::Inert, None, TutorialEvent::Login(NEW_PLAYER));
        assert!(!thais.changes(None));
        assert_eq!(rows(h).await?, 0);

        // Newhaven: the login places the Character on the islet; the forcefield arms the island.
        let login = apply(
            TutorialGate::Newhaven,
            None,
            TutorialEvent::Login(NEW_PLAYER),
        );
        let islet = login.progress.ok_or("login stores the islet")?;
        assert_eq!(
            login.effect,
            Some(crate::domain::tutorial::TutorialEffect::Place(ISLET_TEMPLE))
        );
        assert_eq!(
            write(fence(1)?, 0, islet).await.map_err(debug)?,
            TutorialProgressWrite::Written { version: 1 }
        );
        assert_eq!(
            write(fence(1)?, 1, ARMED).await.map_err(debug)?,
            TutorialProgressWrite::Written { version: 2 }
        );
        let stored = StoredTutorialProgress {
            progress: ARMED,
            version: 2,
        };
        // A replay of the committed write writes nothing; a writer of the old version loses.
        assert_eq!(
            write(fence(1)?, 1, ARMED).await.map_err(debug)?,
            TutorialProgressWrite::Replayed { version: 2 }
        );
        assert_eq!(
            write(fence(1)?, 1, TutorialProgress::new(TutorialStep::Departed))
                .await
                .map_err(debug)?,
            TutorialProgressWrite::Conflict {
                stored: Some(stored)
            }
        );
        assert_eq!(
            write(fence(1)?, 0, islet).await.map_err(debug)?,
            TutorialProgressWrite::Conflict {
                stored: Some(stored)
            }
        );
        assert_eq!(read(fence(1)?).await.map_err(debug)?, Some(stored));
        // The writes advance no CharacterRevision.
        assert_eq!(h.root_revision().await?, "1");

        // Reconnect in the same GameSession: the old generation reads and writes nothing; the
        // current one resumes from the stored step, and its login places nothing.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation = 2")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            read(fence(1)?).await,
            Err(CharacterProgressionError::AuthorityRejected)
        ));
        assert!(matches!(
            write(fence(1)?, 2, islet).await,
            Err(CharacterProgressionError::AuthorityRejected)
        ));
        let current = CurrentCharacterGameplayFence {
            connection_generation: ConnectionGeneration::new(2).map_err(debug)?,
            ..fence(1)?
        };
        let resumed = read(current).await.map_err(debug)?;
        assert_eq!(resumed, Some(stored));
        let relogin = apply(
            TutorialGate::Newhaven,
            resumed.map(|stored| stored.progress),
            TutorialEvent::Login(NEW_PLAYER),
        );
        assert_eq!(relogin.progress, Some(ARMED));
        assert_eq!(relogin.effect, None);

        // Restart: a new node process reads the same step under the current fence.
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let restart_seal = h.recovery.seal_current().map_err(debug)?;
        let restart_authority = restarted
            .open_character_authority(&restart_seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .read_character_tutorial_progress(&restart_authority, &h.node, current)
                .await
                .map_err(debug)?,
            Some(stored)
        );
        drop(restart_authority);
        drop(restart_seal);
        drop(restarted);
        Ok(())
    })
}

/// A stale lease or session writes nothing, an invalid progress is refused before any read, and
/// the table refuses a version that does not follow the stored one.
#[test]
fn tutorial_progress_refuses_stale_fences_and_invalid_rows() -> TestResult {
    run("tutorial_stale", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let write = async |fence, expected: u64, progress| {
            h.root
                .write_character_tutorial_progress(&authority, &h.node, fence, expected, progress)
                .await
        };
        let islet = TutorialProgress::new(TutorialStep::Islet);
        let mut lease = fence(1)?;
        lease.character_lease_generation = 2;
        assert!(matches!(
            write(lease, 0, islet).await,
            Err(CharacterProgressionError::AuthorityRejected)
        ));
        let mut session = fence(1)?;
        session.game_session_id = GameSessionId::decode(&id(51)).map_err(debug)?;
        assert!(write(session, 0, islet).await.is_err());
        let departed_armed = TutorialProgress {
            step: TutorialStep::Departed,
            ..ARMED
        };
        assert!(matches!(
            write(fence(1)?, 0, departed_armed).await,
            Err(CharacterProgressionError::InvalidInput)
        ));
        assert_eq!(rows(h).await?, 0);

        assert_eq!(
            write(fence(1)?, 0, islet).await.map_err(debug)?,
            TutorialProgressWrite::Written { version: 1 }
        );
        // The table keeps the compare-and-set order and the departed invariant itself.
        for statement in [
            "UPDATE game_character_tutorial_progress SET progress_version = 3",
            "UPDATE game_character_tutorial_progress SET progress_version = 1",
            "UPDATE game_character_tutorial_progress SET progress_version = 2, tutorial_step = 3, \
             hunting_hint_pending = true",
            "UPDATE game_character_tutorial_progress SET progress_version = 2, tutorial_step = 4",
        ] {
            assert!(
                sqlx::query(statement).execute(&h.pool).await.is_err(),
                "{statement}"
            );
        }

        // A terminal session reads and writes nothing.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET session_state = 3 \
              WHERE game_session_id = encode($1,'hex')::uuid",
        )
        .bind(id(SESSION).as_slice())
        .execute(&h.pool)
        .await?;
        assert!(matches!(
            write(fence(1)?, 1, ARMED).await,
            Err(CharacterProgressionError::AuthorityRejected)
        ));
        assert!(
            h.root
                .read_character_tutorial_progress(&authority, &h.node, fence(1)?)
                .await
                .is_err()
        );
        assert_eq!(rows(h).await?, 1);
        Ok(())
    })
}
