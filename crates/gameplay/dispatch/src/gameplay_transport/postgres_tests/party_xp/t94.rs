//! T94 qualifying source through the existing genuine party-kill dispatch.
use super::*;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::world_event_outcomes::SeasonBinding;
use crate::foundation::CharacterId;
use crate::native_admission_source::world_event_outcomes::{Challenge, SeasonRecord, Snapshot};
pub(super) async fn prepare(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    world: WorldId,
    first: CharacterId,
    second: CharacterId,
) -> TestResult {
    h.root
        .t94_initialize(a, "oteryn:game:primary", &mut T94Bootstrap)
        .await?;
    h.root
        .t94_start_world(a, world, &Default::default(), "oteryn:game:primary")
        .await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let (guild, season, challenge): (String, String, String) =
        sqlx::query_as("SELECT uuidv7()::text,uuidv7()::text,uuidv7()::text")
            .fetch_one(&h.pool)
            .await?;
    // Harness-admin preexisting Guild fixture, matching the existing PvP guild
    // fixture convention. The real kill and T94 capture run with normal fences.
    let mut guild_seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *guild_seed)
        .await?;
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES($1::uuid,encode($2,'hex')::uuid,'Outcome Guild',2,$3,1,$3)").bind(&guild).bind(world.as_bytes().as_slice()).bind(now).execute(&mut *guild_seed).await?;
    sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES($1::uuid,1,'Leader'),($1::uuid,2,'Member')").bind(&guild).execute(&mut *guild_seed).await?;
    for (character, rank) in [(first, 1i16), (second, 2)] {
        sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at) VALUES(encode($1,'hex')::uuid,$2::uuid,$3,$4)").bind(character.as_bytes().as_slice()).bind(&guild).bind(rank).bind(now).execute(&mut *guild_seed).await?;
    }
    guild_seed.commit().await?;
    let b = SeasonBinding {
        attribution: Default::default(),
        withdraw_display: false,
        season: SeasonRecord {
            ruleset_revision: "t94:test-native-kill-v1".into(),
            starts_at: (now + 3).to_string(),
            ends_at: (now + 3600).to_string(),
            freeze_at: (now + 3601).to_string(),
            state: "OPEN".into(),
            challenges: vec![Challenge {
                challenge_id: challenge,
                source_key: "oteryn:creature.rat".into(),
                scoring_revision: "t94:test-native-kill-v1".into(),
                max_score: "100".into(),
            }],
            standings: vec![],
            computed_at: now.to_string(),
            source_complete_through: now.to_string(),
            correction_of: vec![],
        },
        rules: std::collections::BTreeMap::from([("oteryn:creature.rat".into(), 1)]),
        eligible_guilds: vec![guild],
        tombstone_name: None,
        corrections_closed: false,
        correction_deadline: Some((now + 3601) as u64),
        complete_through: now as u64,
    };
    sqlx::query("INSERT INTO game_t94_bindings(family,world_id,subject_id,approved,approved_at,binding) VALUES('guild_season',encode($1,'hex')::uuid,$2::uuid,true,to_timestamp($3::bigint),$4)").bind(world.as_bytes().as_slice()).bind(season).bind(now-10).bind(serde_json::to_vec(&b)?).execute(&h.pool).await?;
    tokio::time::sleep(Duration::from_secs(4)).await;
    Ok(())
}
pub(super) async fn verify(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let rows:Vec<(String,String,Vec<u8>)>=sqlx::query_as("SELECT guild_id::text,score::text,membership_cut FROM game_t94_contributions ORDER BY guild_id").fetch_all(&h.pool).await?;
    assert_eq!(
        rows.len(),
        1,
        "one guild contribution per actual occurrence, not per party member"
    );
    assert_eq!(rows[0].1, "1");
    assert_eq!(rows[0].2.len(), 32);
    let mut changed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *changed)
        .await?;
    sqlx::query("DELETE FROM game_guild_members WHERE guild_id=$1::uuid")
        .bind(&rows[0].0)
        .execute(&mut *changed)
        .await?;
    changed.commit().await?;
    for _ in 0..8 {
        h.root.t94_produce(a).await?;
    }
    let bytes:Vec<u8>=sqlx::query_scalar("SELECT p.wire FROM game_t94_publications p JOIN game_t94_bindings b USING(family,world_id,subject_id) WHERE p.family='guild_season' AND convert_from(b.binding,'UTF8')::jsonb->'rules' ? 'oteryn:creature.rat' ORDER BY p.delivery_order DESC LIMIT 1").fetch_one(&h.pool).await?;
    let Snapshot::Season(s) = Snapshot::decode(std::str::from_utf8(&bytes)?)? else {
        return Err("T94 season".into());
    };
    assert_eq!(s.season[0].standings.len(), 1);
    assert_eq!(s.season[0].standings[0].total_score, "1");
    assert_eq!(s.season[0].standings[0].rank, "1");
    let retained:Vec<(String,String,Vec<u8>)>=sqlx::query_as("SELECT guild_id::text,score::text,membership_cut FROM game_t94_contributions ORDER BY guild_id").fetch_all(&h.pool).await?;
    assert_eq!(retained, rows);
    Ok(())
}

struct T94Bootstrap;
impl crate::native_admission_source::account_characters::EpochFence for T94Bootstrap {
    fn read(
        &mut self,
    ) -> Result<u64, crate::native_admission_source::account_characters::FenceUnusable> {
        Ok(0)
    }
    fn persist(
        &mut self,
        _: u64,
    ) -> Result<(), crate::native_admission_source::account_characters::FenceUnusable> {
        Ok(())
    }
}
