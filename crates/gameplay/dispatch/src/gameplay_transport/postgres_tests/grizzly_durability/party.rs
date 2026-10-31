use super::*;
use crate::foundation::{PartyMemberSnapshot, PartyPresence, PartySnapshot};

#[test]
fn grizzly_durable_frozen_party_presence_revision_and_busy_world_retry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() { return Ok(()); }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?.join()
        .map_err(|_| "Grizzly party owner PostgreSQL thread panicked")??;
    Ok(())
}

fn source(revision: u64) -> PartySnapshot {
    PartySnapshot {
        world: id(42), character: id(41), party_id: Some(id(90)), revision: Some(revision),
        leader: Some(id(41)), shared_xp_enabled: true,
        members: vec![PartyMemberSnapshot { character: id(41), invitation_order: 1,
            presence_revision: 1, presence: PartyPresence::Channel, channel: Some(id(43)) }],
        observed_database_micros: 1,
    }
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create(admin, "grizzly_frozen_party", true).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let sequencer = CharacterRevisionSequencer::new();
        let catalogue = catalogue()?;
        let yes = PremiumProbe::new(true);
        seed(&h, Some(6), &[]).await?;
        committed(apply(&h, &authority, &sequencer, &catalogue, command(1)?, Action::Join, &yes).await?)?;
        committed(apply(&h, &authority, &sequencer, &catalogue, command(2)?, Action::StartTask(1), &yes).await?)?;
        let observed_revision = state(&h, &authority).await?.character_revision().ok_or("revision")?.get();
        let kill = Action::KillCreature { task_id: 1, creature: "crocodile".into(), observed_revision };
        // This native party predecessor has already changed presence, not membership.
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq,shared_xp_enabled) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,2,true)")
            .bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_revision,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,2,2,NULL)")
            .bind(id(41).as_slice()).bind(id(90).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let applied = sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, fence(1)?, death(80)?, kill.clone(), Arc::clone(&catalogue), source(1))
            .await.map_err(debug)?;
        let receipt = committed(applied)?;
        assert_eq!(value(&h, &authority, quest::KILL_COUNTER_BASE + 1).await?, 1);
        assert_eq!(sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, fence(1)?, death(80)?, kill.clone(), Arc::clone(&catalogue), source(1))
            .await.map_err(debug)?, QuestTransitionOutcome::AlreadyCommitted(receipt));

        sqlx::query("UPDATE game_parties SET revision=2 WHERE party_id=encode($1,'hex')::uuid")
            .bind(id(90).as_slice()).execute(&h.pool).await?;
        for _ in 0..2 {
            assert_eq!(sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
                &h.root, &authority, &h.node, fence(1)?, death(81)?, kill.clone(), Arc::clone(&catalogue), source(1))
                .await.map_err(debug)?, QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch));
        }
        assert_eq!(value(&h, &authority, quest::KILL_COUNTER_BASE + 1).await?, 1);
        let conflict = sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, fence(1)?, death(81)?, kill.clone(), Arc::clone(&catalogue), source(2)).await;
        assert!(matches!(conflict, Err(CharacterProgressionError::ConflictingOccurrence)), "{conflict:?}");

        // A busy native World owner must not wait while holding a Character root,
        // and must never turn transient lock contention into a durable refusal.
        let mut world = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))")
            .bind(id(42).iter().map(|byte| format!("{byte:02x}")).collect::<String>())
            .execute(&mut *world).await?;
        let busy = sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, fence(1)?, death(82)?, kill.clone(), Arc::clone(&catalogue), source(2)).await;
        assert!(matches!(busy, Err(CharacterProgressionError::Unavailable(_))), "{busy:?}");
        world.commit().await?;
        committed(sequencer.acquire(fence(1)?.character_id).await.commit_grizzly_kill(
            &h.root, &authority, &h.node, fence(1)?, death(82)?, kill, Arc::clone(&catalogue), source(2))
            .await.map_err(debug)?)?;
        assert_eq!(value(&h, &authority, quest::KILL_COUNTER_BASE + 1).await?, 2);
        eprintln!("GRIZZLY_PARTY_OWNER POSTGRES_CASE_COMPLETED");
        drop(authority); drop(seal); h.cleanup().await
    })
}
