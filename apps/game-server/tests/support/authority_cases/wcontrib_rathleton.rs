//! WCONTRIB batch 2: real point custody, ballot identity, promoted ranks and saved elections.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::DurabilityError;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_travel::{NpcTravelGate, TravelRefusal};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, GameSessionId};
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute;
use oteryn_gameplay_content::content::{ProjectV2TravelDestination, ProjectV2TravelRoute};
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue, rathleton};
use std::sync::Arc;

const FUNGI: &str = "oteryn:quest-transition/world-contribution/votes/shortcut/fungi";
const MINOTAUR: &str = "oteryn:quest-transition/world-contribution/votes/shortcut/minotaur";
const GOLEMS: &str = "oteryn:quest-transition/world-contribution/votes/dungeon/golems";
const BULLWARK: &str = "oteryn:quest-transition/world-contribution/votes/boss/bullwark";
const COMMONER: &str = "oteryn:quest-transition/rathleton/promote/commoner";
const INHABITANT: &str = "oteryn:quest-transition/rathleton/promote/inhabitant";
const CITIZEN: &str = "oteryn:quest-transition/rathleton/promote/citizen";
const SQUIRE: &str = "oteryn:quest-transition/rathleton/promote/squire";

#[test]
fn wcontrib_rathleton_pg_point_debit_ballot_ranks_and_save_period() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("wcontrib-rathleton-pg".into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(exercise(admin))
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Rathleton PG thread failed")??;
    Ok(())
}

async fn exercise(admin: String) -> fixture::TestResult {
    let started = std::time::Instant::now();
    let h = fixture::Harness::create(admin, "wcontrib_rathleton", true).await?;
    let result = run(&h).await;
    h.cleanup().await?;
    result?;
    eprintln!(
        "WCONTRIB_RATHLETON_PG_EXECUTED: point debit, exact and renamed replay, stale generation, explicit 50/150/300/500 promotions, save-bound period/winners; elapsed={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

pub(super) fn request(key: &str, command: u64) -> fixture::TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: key.into(),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&fixture::id(fixture::SESSION)).map_err(fixture::debug)?,
            CommandId::new(command).map_err(fixture::debug)?,
        )),
    })
}
pub(super) async fn revision(h: &fixture::Harness) -> fixture::TestResult<u64> {
    let value:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    Ok(value.parse()?)
}
pub(super) async fn commit(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    catalogue: &Arc<QuestStateCatalogue>,
    key: &str,
    command: u64,
) -> fixture::TestResult<QuestTransitionOutcome> {
    Ok(h.root
        .commit_character_quest_transition(
            authority,
            &h.node,
            fixture::fence(revision(h).await?)?,
            request(key, command)?,
            Arc::clone(catalogue),
        )
        .await
        .map_err(|e| format!("Rath {key} command {command}: {e:?}"))?)
}
pub(super) async fn tracks(h: &fixture::Harness) -> fixture::TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1),coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$3),0),coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$4),0)")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(rathleton::POINTS_TRACK).bind(rathleton::SPENT_TRACK).bind(rathleton::RANK_TRACK).fetch_one(&h.pool).await?)
}
pub(super) async fn historical_points(
    h: &fixture::Harness,
    points: i64,
    spent: i64,
    rank: i64,
) -> fixture::TestResult {
    let current = revision(h).await?.max(2);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (key, value) in [
        (rathleton::POINTS_TRACK, points),
        (rathleton::SPENT_TRACK, spent),
        (rathleton::RANK_TRACK, rank),
    ] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(key).bind(rathleton::QUEST).bind(value).bind(current.to_string()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
pub(super) async fn activate(h: &fixture::Harness, epoch: i64) -> fixture::TestResult {
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(fixture::WORLD).as_slice())
    .bind(epoch)
    .execute(&h.pool)
    .await?;
    h.root
        .world_changes(fixture::scope()?.world_id(), u64::try_from(epoch)?)
        .await?
        .ok_or("saved world decision")?;
    Ok(())
}
pub(super) async fn seed(
    h: &fixture::Harness,
    _catalogue: &QuestStateCatalogue,
) -> fixture::TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2")
        .execute(&mut *tx)
        .await?;
    for (epoch, days) in [(0_i64, 0_i64), (1, 13), (2, 14)] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4::bigint::double precision/1000),'2000-01')")
            .bind(fixture::id(fixture::WORLD).as_slice()).bind(epoch).bind([1_u8;32].as_slice()).bind(rathleton::ELECTION_ANCHOR_MS+days*86_400_000).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(fixture::id(fixture::WORLD).as_slice())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    h.root
        .world_changes(fixture::scope()?.world_id(), 0)
        .await?
        .ok_or("initial world")?;
    Ok(())
}
pub(super) async fn anchor(
    h: &fixture::Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    catalogue: &Arc<QuestStateCatalogue>,
) -> fixture::TestResult {
    assert!(matches!(
        commit(
            h,
            authority,
            catalogue,
            "oteryn:quest-transition/quest/u10_50/oramond_quest/quest_line/npc_1",
            9000
        )
        .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    Ok(())
}
async fn run(h: &fixture::Harness) -> fixture::TestResult {
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(fixture::debug)?
            .catalogue()
            .clone(),
    );
    seed(h, &catalogue)
        .await
        .map_err(|e| format!("Rath seed: {e}"))?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(|e| format!("Rath authority: {e:?}"))?;
    anchor(h, &authority, &catalogue).await?;
    historical_points(h, 3, 49, 0).await?;
    assert_eq!(
        commit(h, &authority, &catalogue, COMMONER, 20).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert!(matches!(
        commit(h, &authority, &catalogue, FUNGI, 1).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(tracks(h).await?, (2, 50, 0));
    assert!(matches!(
        commit(h, &authority, &catalogue, FUNGI, 1).await?,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(tracks(h).await?, (2, 50, 0));
    let before = revision(h).await?;
    assert_eq!(
        commit(h, &authority, &catalogue, MINOTAUR, 1).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(tracks(h).await?, (2, 50, 0));
    assert_eq!(revision(h).await?, before);
    let mut stale = fixture::fence(before)?;
    stale.connection_generation = ConnectionGeneration::new(2).map_err(fixture::debug)?;
    assert!(matches!(
        h.root
            .commit_character_quest_transition(
                &authority,
                &h.node,
                stale,
                request(FUNGI, 2)?,
                Arc::clone(&catalogue)
            )
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    assert_eq!(tracks(h).await?, (2, 50, 0));
    assert_eq!(
        commit(h, &authority, &catalogue, GOLEMS, 3).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert!(matches!(
        commit(h, &authority, &catalogue, COMMONER, 4).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(tracks(h).await?, (2, 50, 1));
    assert!(matches!(
        commit(h, &authority, &catalogue, GOLEMS, 5).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(tracks(h).await?, (1, 51, 1));
    assert_eq!(
        commit(h, &authority, &catalogue, BULLWARK, 6).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    // Historical earned/spent DATA compresses hundreds of already-qualified daily
    // tasks; each threshold crossing and promotion below still uses the real writer.
    historical_points(h, 2, 149, 1).await?;
    assert!(matches!(
        commit(h, &authority, &catalogue, FUNGI, 7).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(tracks(h).await?, (1, 150, 1));
    assert_eq!(
        commit(h, &authority, &catalogue, BULLWARK, 8).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert!(matches!(
        commit(h, &authority, &catalogue, INHABITANT, 9).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert!(matches!(
        commit(h, &authority, &catalogue, BULLWARK, 10).await?,
        QuestTransitionOutcome::Committed(_)
    ));
    assert_eq!(tracks(h).await?, (0, 151, 2));
    assert_eq!(
        commit(h, &authority, &catalogue, FUNGI, 11).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    for (spent, rank, promotion, ordinal) in
        [(299_i64, 2_i64, CITIZEN, 12_u64), (499, 3, SQUIRE, 14)]
    {
        historical_points(h, 1, spent, rank).await?;
        assert_eq!(
            commit(h, &authority, &catalogue, promotion, ordinal).await?,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        );
        assert!(matches!(
            commit(h, &authority, &catalogue, FUNGI, ordinal + 1).await?,
            QuestTransitionOutcome::Committed(_)
        ));
        assert!(matches!(
            commit(h, &authority, &catalogue, promotion, ordinal + 20).await?,
            QuestTransitionOutcome::Committed(_)
        ));
        assert_eq!(tracks(h).await?, (0, spent + 1, rank + 1));
    }
    assert_eq!(
        commit(h, &authority, &catalogue, COMMONER, 40).await?,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    );
    assert_eq!(tracks(h).await?, (0, 500, 4));
    let ballots:(i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(amount),0)::bigint FROM game_world_change_ballots WHERE world_id=encode($1,'hex')::uuid AND election_period=0")
        .bind(fixture::id(fixture::WORLD).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(ballots, (6, 6));
    let world = fixture::scope()?.world_id();
    assert!(
        h.root
            .shared_world_changes(world, 0)
            .await?
            .ok_or("initial shared")?
            .election_winners
            .is_empty()
    );
    activate(h, 1).await?;
    assert!(
        h.root
            .shared_world_changes(world, 1)
            .await?
            .ok_or("pre-boundary shared")?
            .election_winners
            .is_empty()
    );
    activate(h, 2).await?;
    let winners = h
        .root
        .shared_world_changes(world, 2)
        .await?
        .ok_or("election shared")?
        .election_winners;
    assert_eq!(
        winners.get("shortcut").map(String::as_str),
        Some("shortcut/fungi")
    );
    assert_eq!(
        winners.get("dungeon").map(String::as_str),
        Some("dungeon/golems")
    );
    assert_eq!(
        winners.get("boss").map(String::as_str),
        Some("boss/bullwark")
    );
    assert!(!winners.contains_key("raid"));
    // The actual paid-travel quote joins both promoted rank and election identity.
    historical_points(h, 0, 500, 0).await?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                route("dungeon_golems"),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    historical_points(h, 0, 500, 1).await?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                route("dungeon_golems"),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Ok(0)
    );
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                route("dungeon_minotaurs"),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    historical_points(h, 0, 500, 3).await?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                route("hero_of_rathleton"),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );
    historical_points(h, 0, 500, 4).await?;
    assert_eq!(
        h.root
            .quote_npc_travel(
                &authority,
                &h.node,
                fixture::fence(revision(h).await?)?,
                route("hero_of_rathleton"),
                &AllowTravel
            )
            .await
            .map_err(fixture::debug)?,
        Ok(0)
    );
    assert_eq!(
        h.root
            .shared_world_changes(world, 2)
            .await?
            .ok_or("replayed save")?
            .election_winners,
        winners
    );
    drop(authority);
    drop(seal);
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
fn route(key: &str) -> TravelRoute {
    let (x, y, floor) = match key {
        "dungeon_minotaurs" => (31254, 32604, 9),
        "dungeon_golems" => (31061, 32605, 9),
        "dungeon_wrathofevil" => (33459, 31715, 9),
        "hero_of_rathleton" => (33533, 31951, 14),
        _ => unreachable!("fixed native passage fixture"),
    };
    TravelRoute {
        service: "oteryn:service.travel.marvin".into(),
        route: ProjectV2TravelRoute {
            key: key.into(),
            destination: ProjectV2TravelDestination {
                coordinate_frame: "global-target-2026-09-27".into(),
                x,
                y,
                floor,
            },
            price: 0,
            premium: true,
            min_level: None,
        },
        keyword: key.replace('_', " "),
        postman: false,
        carpet: false,
        held: false,
        yalahar_gate: false,
        yalahar_town_gate: None,
        quest_gate: None,
    }
}
