//! WCONTRIB: real Quest writer, custody guards and shared save transitions.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::DurabilityError;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_travel::{NpcTravelGate, TravelRefusal};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::world_event::changes;
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, GameSessionId};
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute;
use oteryn_gameplay_content::content::{ProjectV2TravelDestination, ProjectV2TravelRoute};
use oteryn_gameplay_quests::quest::QuestRefusal;
use oteryn_gameplay_world_change::contribution::LakeState;
use oteryn_gameplay_world_change::owner::ContributionKind;
use std::sync::Arc;

const COAL: &str = "oteryn:quest-transition/world-contribution/coal/100";
const MEDICINE: &str = "oteryn:quest-transition/world-contribution/medicine/1";
const CORPSE: &str = "oteryn:quest-transition/world-contribution/corpses/1";

#[test]
fn wcontrib_postgres_custody_replay_refusal_and_save() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wcontrib-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(exercise(admin))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "wcontrib thread failed")??;
    Ok(())
}

fn request(key: &str, ordinal: u64) -> fixture::TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: key.into(),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
            CommandId::new(ordinal).map_err(fixture::debug)?,
        )),
    })
}

async fn exercise(admin: String) -> fixture::TestResult {
    let started = std::time::Instant::now();
    let h = fixture::Harness::create(admin, "wcontrib_custody", true).await?;
    let result = run(&h).await;
    h.cleanup().await?;
    result?;
    eprintln!(
        "WCONTRIB BATCH1 PG EXECUTED: real custody debit, Quest receipt replay, renamed occurrence refusal, rollback, stale generation, expired timed corpse, saved-state projection; duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

async fn seed(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::id(fixture::WORLD);
    // Historical starting custody DATA only. All production guards are restored
    // before every tested call, which runs through the normal runtime-role writer.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(fixture::id(90).as_slice()).bind(world.as_slice()).bind(fixture::id(91).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(90).as_slice()).bind(world.as_slice()).bind(fixture::id(92).as_slice()).execute(&mut *tx).await?;
    for (seed, key, count, ordinal) in [
        (100_u8, "oteryn:item.tibia.i12600", 100_i64, 1_i64),
        (103, "oteryn:item.tibia.i12600", 100, 2),
        (106, "oteryn:item.tibia.i12600", 50, 3),
        (110, "oteryn:item.tibia.i12517", 1, 4),
        (120, "oteryn:item.tibia.i4178", 1, 5),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)")
            .bind(fixture::id(seed).as_slice()).bind(world.as_slice()).bind(key).bind(count).bind(fixture::id(seed+1).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,placement_ordinal,item_instance_id,world_id,placed_transaction_id,parent_item_instance_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid)")
            .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(ordinal).bind(fixture::id(seed).as_slice()).bind(world.as_slice()).bind(fixture::id(seed+2).as_slice()).bind(fixture::id(90).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,remaining_ms,deadline_at,state_revision) VALUES(encode($1,'hex')::uuid,NULL,60000,NULL,1)")
        .bind(fixture::id(120).as_slice()).execute(&mut *tx).await?;
    for epoch in 0..=2_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_slice()).bind(epoch).bind([1_u8;32].as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn quantity(h: &fixture::Harness, seed: u8) -> fixture::TestResult<(i64, i16)> {
    Ok(sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(fixture::id(seed).as_slice()).fetch_one(&h.pool).await?)
}

async fn tally(h: &fixture::Harness, resource: &str) -> fixture::TestResult<u64> {
    let value: Option<String> = sqlx::query_scalar("SELECT tally::text FROM game_world_change_tallies WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0 AND resource=$2")
        .bind(fixture::id(fixture::WORLD).as_slice()).bind(resource).fetch_optional(&h.pool).await?;
    Ok(value.map(|v| v.parse()).transpose()?.unwrap_or(0))
}

async fn run(h: &fixture::Harness) -> fixture::TestResult {
    seed(h).await?;
    let world = fixture::scope()?.world_id();
    h.root
        .world_changes(world, 0)
        .await?
        .ok_or("missing initial world decision")?;
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    );
    for key in [COAL, MEDICINE, CORPSE] {
        assert!(
            catalogue.transition(key).is_some(),
            "missing contribution route {key}"
        );
    }
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let commit =
        async |revision, key: &str, ordinal| -> fixture::TestResult<QuestTransitionOutcome> {
            Ok(h.root
                .commit_character_quest_transition(
                    &authority,
                    &h.node,
                    fixture::fence(revision)?,
                    request(key, ordinal)?,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(fixture::debug)?)
        };
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(1)?,
                junkar(),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    let initial = h
        .root
        .shared_world_changes(world, 0)
        .await?
        .ok_or("initial shared state")?;
    assert!(!initial.steamship_open && initial.swamp_epidemic);
    assert_eq!(initial.lake_state, LakeState::Clean);
    // This is historical retiring-epoch custody credit, not a production bypass.
    // The final credited corpse below must still burn the real held item.
    sqlx::query("INSERT INTO game_world_change_tallies(world_id,reset_epoch,resource,tally) VALUES(encode($1,'hex')::uuid,0,'corpses',999),(encode($1,'hex')::uuid,0,'medicine',99)")
        .bind(fixture::id(fixture::WORLD).as_slice()).execute(&h.pool).await?;
    let first = commit(1, COAL, 1).await?;
    assert!(matches!(first, QuestTransitionOutcome::Committed(_)));
    assert_eq!(remaining_coal(h).await?, 150);
    assert_eq!(tally(h, "coal").await?, 100);
    let replay = commit(2, COAL, 1).await?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(remaining_coal(h).await?, 150);
    assert_eq!(tally(h, "coal").await?, 100);
    // One command occurrence cannot be renamed to debit another resource.
    assert_eq!(
        commit(2, MEDICINE, 1).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(quantity(h, 110).await?, (1, 1));
    assert_eq!(tally(h, "medicine").await?, 99);
    assert_eq!(remaining_coal(h).await?, 150);
    assert_eq!(tally(h, "coal").await?, 100);
    let revision_after_conflict: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(revision_after_conflict, "2");
    assert!(matches!(
        commit(2, COAL, 2).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(remaining_coal(h).await?, 50);
    assert_eq!(tally(h, "coal").await?, 200);
    assert_eq!(
        commit(3, COAL, 3).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(remaining_coal(h).await?, 50);
    assert_eq!(tally(h, "coal").await?, 200);
    let mut stale = fixture::fence(3)?;
    stale.connection_generation = ConnectionGeneration::new(2).map_err(fixture::debug)?;
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                stale,
                request(MEDICINE, 4)?,
                Arc::clone(&catalogue)
            )
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    assert_eq!(quantity(h, 110).await?, (1, 1));
    assert_eq!(tally(h, "medicine").await?, 99);
    assert!(matches!(
        commit(3, MEDICINE, 5).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(quantity(h, 110).await?, (0, 2));
    assert_eq!(tally(h, "medicine").await?, 100);
    // Change exactly the temporal invariant; custody, identity and generations
    // remain valid. An expired corpse cannot be exchanged for shared credit.
    timed_deadline(h, Some(1)).await?;
    assert_eq!(
        commit(4, CORPSE, 7).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(quantity(h, 120).await?, (1, 1));
    assert_eq!(tally(h, "corpses").await?, 999);
    let revision: String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(revision, "4");
    timed_deadline(h, None).await?;
    assert!(matches!(
        commit(4, CORPSE, 6).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(quantity(h, 120).await?, (0, 2));
    let retired_timed: (Option<i64>,Option<i64>,String) = sqlx::query_as("SELECT remaining_ms,deadline_at,state_revision::text FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(fixture::id(120).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retired_timed,
        (Some(60000), None, "1".into()),
        "burn retains the exact timed snapshot on its retired item"
    );
    assert_eq!(tally(h, "corpses").await?, 1000);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    assert_eq!(
        changes::tally(&mut tx, world, 0, ContributionKind::Coal).await?,
        200
    );
    tx.rollback().await?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_change_debits WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
        .bind(fixture::id(fixture::WORLD).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, 4,
        "only successful fresh operations own custody debit receipts"
    );
    let mut stale_replay = fixture::fence(5)?;
    stale_replay.connection_generation = ConnectionGeneration::new(2).map_err(fixture::debug)?;
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                stale_replay,
                request(COAL, 1)?,
                Arc::clone(&catalogue)
            )
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    // Deposits cannot change the current day's immutable shared state.
    assert_eq!(h.root.shared_world_changes(world, 0).await?, Some(initial));
    activate(h, 1).await?;
    h.root
        .world_changes(world, 1)
        .await?
        .ok_or("saved world decision")?;
    let saved = h
        .root
        .shared_world_changes(world, 1)
        .await?
        .ok_or("saved shared state")?;
    assert!(saved.steamship_open && !saved.swamp_epidemic);
    assert_eq!(saved.lake_state, LakeState::Dirty);
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(5)?,
                junkar(),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Ok(10)
    );
    assert!(h.root.shared_world_changes(world, 0).await?.is_none());
    activate(h, 2).await?;
    h.root
        .world_changes(world, 2)
        .await?
        .ok_or("second world decision")?;
    let next = h
        .root
        .shared_world_changes(world, 2)
        .await?
        .ok_or("next shared state")?;
    assert!(!next.steamship_open && next.swamp_epidemic);
    assert_eq!(next.lake_state, LakeState::Clean);
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(5)?,
                junkar(),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    drop(authority);
    drop(seal);
    Ok(())
}

async fn activate(h: &fixture::Harness, epoch: i64) -> fixture::TestResult {
    // The actual server-save owner is tested separately; this fixture advances
    // only its active pointer, then uses the production decision projection.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$1")
        .bind(epoch)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

struct AllowTravel;
impl NpcTravelGate for AllowTravel {
    async fn travel_permitted(
        &self,
        _tx: &mut Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        Ok(true)
    }
}
fn junkar() -> TravelRoute {
    TravelRoute {
        service: "oteryn:service.travel.junkar_mines".into(),
        route: ProjectV2TravelRoute {
            key: "kazordoon".into(),
            destination: ProjectV2TravelDestination {
                coordinate_frame: "global-target-2026-09-27".into(),
                x: 32660,
                y: 31957,
                floor: -15,
            },
            price: 10,
            premium: false,
            min_level: None,
        },
        keyword: "kazordoon".into(),
        postman: true,
        carpet: false,
        held: false,
        yalahar_gate: false,
        yalahar_town_gate: None,
        quest_gate: None,
    }
}

async fn remaining_coal(h: &fixture::Harness) -> fixture::TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE world_id=encode($1,'hex')::uuid AND definition_production_key='oteryn:item.tibia.i12600'")
        .bind(fixture::id(fixture::WORLD).as_slice()).fetch_one(&h.pool).await?)
}

async fn timed_deadline(h: &fixture::Harness, deadline: Option<i64>) -> fixture::TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_timed_states SET deadline_at=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(fixture::id(120).as_slice()).bind(deadline).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
