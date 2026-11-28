use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, maintain_ready, prepare_current_session};
use crate::durability::account_achievement::EarnedAchievement;
use crate::durability::highscore_snapshot::{
    AchievementPoints, SnapshotCatalogues, reply::HighscoresReply,
};
use crate::gameplay_transport::{
    charm_progression::canonical_races, connection::FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn::highscores::{HighscoresQuery, decode_highscores_result};
use std::sync::Arc;

struct NoEarnedAchievements;

impl AchievementPoints for NoEarnedAchievements {
    fn total_points(&self, earned: &[EarnedAchievement]) -> Option<u32> {
        earned.is_empty().then_some(0)
    }
}

#[test]
fn highscores_charm_points_publish_earned_totals_and_shared_ranks() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("highscores-charm-points", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(check_published_rankings(admin))
    })
}

async fn check_published_rankings(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "hs_charm_points", true, false).await?;
    prepare_current_session(&h).await?;
    let race = canonical_races()
        .and_then(|races| {
            races
                .iter()
                .find(|race| race.key() == "oteryn:creature.rat")
        })
        .ok_or("missing canonical Rat Bestiary entry")?;

    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let dependencies = Dependencies::new()?;
    let owner = dependencies.owner(&h, &authority);
    let admitted = dependencies.admitted;
    // Open the genuine session authority before adding synthetic ranking peers.
    seed_ranked_characters(&h, race.key(), i64::from(race.final_kill_threshold())).await?;
    let catalogues =
        SnapshotCatalogues::activated(Arc::new(NoEarnedAchievements)).map_err(debug)?;
    maintain_ready(&h.root).await?;
    let cut = h
        .root
        .refresh_highscore_schedule(admitted.world_id, catalogues.clone(), false)
        .await
        .map_err(debug)?;
    assert!(
        cut.snapshots.iter().any(|snapshot| snapshot.category == 11),
        "the live refresh must publish category 11"
    );

    maintain_ready(&h.root).await?;
    let reply = owner
        .highscores(
            admitted.world_id,
            admitted.game_session_id,
            HighscoresQuery {
                world: None,
                details: true,
                category: 11,
                vocation: 1,
                page: 1,
                snapshot_id: 0,
            },
        )
        .await;
    let HighscoresReply::Result(bytes) = reply else {
        return Err(format!("expected Charm Points page, got {reply:?}").into());
    };
    let page = decode_highscores_result(&bytes).map_err(debug)?;
    let bestiary_points = u64::from(catalogues.bestiary[race.key()].charm_points);
    assert!(bestiary_points > 0);
    let rows: Vec<_> = page
        .rows
        .iter()
        .map(|row| (row.name.as_str(), row.rank, row.value))
        .collect();
    assert_eq!(
        rows,
        [
            ("Fixture Hero", 1, bestiary_points + 2),
            ("Alpha Hero", 2, bestiary_points),
            ("Zeta Hero", 2, bestiary_points),
            ("Partial Hero", 4, 0),
        ]
    );
    assert_eq!(page.own_row.as_ref(), page.rows.first());
    drop(owner);
    drop(authority);
    drop(seal);
    eprintln!(
        "POSTGRES_CASE_COMPLETED highscores_charm_points_publish_earned_totals_and_shared_ranks"
    );
    h.cleanup().await
}

async fn seed_ranked_characters(h: &Harness, race: &str, threshold: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    // Administrative ranking fixtures, with no gameplay producer or session for the peers.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let own = id(41);
    let own = own.as_slice();
    sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'knight',0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,game_character_uuid_v7()) ON CONFLICT(character_id) DO UPDATE SET vocation='knight'")
        .bind(own).execute(&mut *tx).await?;
    for (tag, name) in [(80, "Zeta Hero"), (81, "Alpha Hero"), (82, "Partial Hero")] {
        let peer = id(tag);
        let peer = peer.as_slice();
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($2,'hex')::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(own).bind(peer).bind(name).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_progression_state SELECT (jsonb_populate_record(NULL::game_character_progression_state,to_jsonb(p)||jsonb_build_object('character_id',encode($2,'hex')::uuid))).* FROM game_character_progression_state p WHERE character_id=encode($1,'hex')::uuid")
            .bind(own).bind(peer).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_build_state SELECT (jsonb_populate_record(NULL::game_character_build_state,to_jsonb(b)||jsonb_build_object('character_id',encode($2,'hex')::uuid))).* FROM game_character_build_state b WHERE character_id=encode($1,'hex')::uuid")
            .bind(own).bind(peer).execute(&mut *tx).await?;
    }
    for tag in [41, 80, 81, 82] {
        let kills = if tag == 82 { threshold - 1 } else { threshold };
        sqlx::query("INSERT INTO game_character_bestiary_progress(character_id,race_key,kill_count,committed_character_revision,last_bestiary_occurrence_id) VALUES(encode($1,'hex')::uuid,$2,$3,2,game_character_uuid_v7())")
            .bind(id(tag).as_slice()).bind(race).bind(kills).execute(&mut *tx).await?;
    }
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_echo_warden_kill_receipts(character_id,occurrence_id,species_key,bestiary_stars,first_species,charm_points) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,true,2)")
        .bind(own).bind(id(83).as_slice()).bind(race).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_echo_warden_rewards(character_id,species_key,first_occurrence_id,bestiary_stars,charm_points) VALUES(encode($1,'hex')::uuid,$3,encode($2,'hex')::uuid,1,2)")
        .bind(own).bind(id(83).as_slice()).bind(race).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
