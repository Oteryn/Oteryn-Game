//! Demon Wars prince deaths credit their own personal award and shared faction together.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes::counters::{
    CounterKind, CreditEvent, CreditOutcome, bindings,
};
use crate::foundation::{CharacterId, RuntimeScopeRefV1, ScopeOwnershipGeneration};

#[test]
fn worldchg_postgres_demon_princes_award_each_nemesis_at_one_hundred() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h =
                        fixture::Harness::create(admin, "worldchg_demon_nemesis", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Demon Wars PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = scope
    else {
        return Err("channel fixture".into());
    };
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
        .bind(world_id.as_bytes().as_slice()).bind([7_u8; 32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let day = h
        .root
        .world_changes(world_id, 1)
        .await?
        .ok_or("world changes")?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let character =
        CharacterId::decode(&fixture::id(fixture::CHARACTER)).map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let event = |creature: &str, ordinal: u32| -> fixture::TestResult<CreditEvent> {
        let mut occurrence = world_id.as_bytes().to_vec();
        occurrence.extend_from_slice(channel_id.as_bytes());
        occurrence.extend_from_slice(&generation.get().to_be_bytes());
        occurrence.extend_from_slice(&ordinal.to_be_bytes());
        occurrence.extend_from_slice(&1_u64.to_be_bytes());
        Ok(CreditEvent {
            source_kind: 1,
            occurrence,
            credits: bindings()?.credits(creature, (33260, 31930, 13)),
            counter_generations: day.counter_generations,
            occurred_at_ms: 100,
            character: Some(character),
        })
    };
    let deaths = async |creature, ordinal| {
        h.root
            .credit_world_change_event(
                &authority,
                &h.node,
                scope,
                generation,
                1,
                event(creature, ordinal)?,
            )
            .await
            .map_err(Box::<dyn std::error::Error>::from)
    };
    let awards = |key: &str| {
        let pool = &h.pool;
        let key = key.to_owned();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key=$1")
                .bind(key).fetch_one(pool).await
        }
    };
    let askarak = "oteryn:achievement/askarak_nemesis";
    let shaburak = "oteryn:achievement/shaburak_nemesis";
    for (prince, ordinary, key, start) in [
        (
            "oteryn:creature.askarak_prince",
            "oteryn:creature.askarak_lord",
            askarak,
            1,
        ),
        (
            "oteryn:creature.shaburak_prince",
            "oteryn:creature.shaburak_demon",
            shaburak,
            110,
        ),
    ] {
        for ordinal in start..start + 99 {
            assert_eq!(
                deaths(prince, ordinal).await?,
                CreditOutcome::Applied {
                    phase_changed: false
                }
            );
        }
        assert_eq!(
            deaths(ordinary, start + 99).await?,
            CreditOutcome::Applied {
                phase_changed: false
            }
        );
        assert_eq!(awards(key).await?, 0);
        assert_eq!(
            deaths(prince, start + 100).await?,
            CreditOutcome::Applied {
                phase_changed: false
            }
        );
        assert_eq!(awards(key).await?, 1);
        assert_eq!(
            deaths(prince, start + 100).await?,
            CreditOutcome::AlreadyApplied
        );
        assert_eq!(
            deaths(prince, start + 101).await?,
            CreditOutcome::Applied {
                phase_changed: false
            }
        );
        assert_eq!(awards(key).await?, 1);
    }
    let day = h
        .root
        .world_changes(world_id, 1)
        .await?
        .ok_or("world changes")?;
    assert_eq!(
        day.counter_progress[CounterKind::DemonWars.index()],
        [102, 102, 0]
    );
    Ok(())
}
