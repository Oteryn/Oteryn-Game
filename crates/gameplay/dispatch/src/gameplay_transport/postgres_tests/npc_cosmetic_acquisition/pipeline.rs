//! Historical eligible level25 is isolated fixture setup, never an earned-XP claim.
//! Every task assignment/death/report/Fame point and later XP award is earned through real owners.
use super::*;
#[path = "pipeline_combat.rs"]
mod combat;
#[path = "pipeline_scene.rs"]
mod scene;
#[path = "pipeline_walk.rs"]
mod walk;
const MAIN: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_kill_main";
const FAME: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_fame_points";
async fn track(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(key).fetch_optional(&h.pool).await?.unwrap_or(-1))
}
async fn progression(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT total_experience,level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn source_clock(h: &Harness, seconds: i64) -> TestResult {
    // Existing trusted isolated-DB clock seam; no source/client supplies this value.
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS TIMESTAMPTZ LANGUAGE sql VOLATILE SET search_path=pg_catalog AS $$ SELECT to_timestamp({seconds}) $$"))).execute(&h.pool).await?;
    Ok(())
}
async fn talk(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    texts: &[&str],
) -> TestResult {
    let first = admitted.continuity.next_command_id;
    let mut commands = Vec::with_capacity(texts.len());
    for (i, text) in texts.iter().enumerate() {
        commands.push((
            first
                .checked_add(u64::try_from(i)?)
                .ok_or("command overflow")?,
            *text,
        ));
    }
    super::super::super::grizzly_npc::wire::talk(owner, admitted, npc, &commands).await
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    gnomilly: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let began = std::time::Instant::now();
    let actor = admitted.runtime_actor.ok_or("real source player")?;
    let session = admitted.game_session_id;
    owner.admit_quest_session(admitted).await;
    combat::historical_source_eligibility(h, owner, admitted).await?;
    let scene = scene::select(owner)?;
    combat::initialize(owner, &scene)
        .await
        .map_err(|e| format!("Spike original spawn initialization: {e}"))?;
    let (_, gnomilly_cell) = scene::npc(owner, "oteryn:npc.gnomilly")
        .await
        .map_err(|e| format!("Spike placed Gnomilly lookup after spawn: {e}"))?;
    let binding = owner
        .progression_binding(session)
        .ok_or("genuine native progression binding")?;
    let next_level = binding
        .policy
        .thresholds
        .iter()
        .find(|t| t.level == 26)
        .ok_or("native level26 threshold")?
        .minimum_experience
        .get();
    // Observe actual mature XP settlement, including the real rate policy.
    // Stop with seven genuine task kills plus the1000XP report crossing26.
    let mut last_kill_xp = 240_i64;
    let mut prep = 0;

    while next_level - progression(h).await?.0 > 7 * last_kill_xp + 1000 {
        walk::walk(owner, admitted, gnomilly_cell)
            .await
            .map_err(|e| format!("Spike actual Gnomilly approach: {e}"))?;
        let respawn_ms = scene
            .table
            .sources
            .iter()
            .flat_map(|s| s.points.iter())
            .map(|p| p.respawn_ms)
            .max()
            .ok_or("actual respawn delay")?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(
                u64::from(respawn_ms) + 1000,
            ))
            .ok_or("source respawn clock")?;
        combat::respawn(owner).await?;
        for (home, _) in &scene.homes {
            if next_level - progression(h).await?.0 <= 7 * last_kill_xp + 1000 {
                break;
            }
            let before_xp = progression(h).await?.0;
            combat::kill(h, owner, admitted, *home).await?;
            last_kill_xp = progression(h).await?.0 - before_xp;
            assert!(
                (1..=1000).contains(&last_kill_xp),
                "bounded actual low-level native XP settlement"
            );
            prep += 1;
            assert!(prep <= 2000, "bounded genuine prep");
        }
    }
    assert_eq!(
        progression(h).await?.1,
        25,
        "historical eligible25 plus only actual native XP thereafter"
    );
    assert!(
        next_level - progression(h).await?.0 > 7 * last_kill_xp,
        "seven real task kills leave the level crossing to reportXP"
    );
    assert_eq!(track(h, FAME).await?, -1, "prep must not invent any Fame");
    assert_eq!(track(h, MAIN).await?, -1, "prep has no assignment");
    let mut now = 1_800_000_000_i64;
    let mut report_crossed = false;
    for cycle in 0..100 {
        source_clock(h, now).await?;
        walk::walk(owner, admitted, gnomilly_cell)
            .await
            .map_err(|e| format!("Spike actual Gnomilly approach: {e}"))?;
        talk(owner, admitted, gnomilly, &["hi", "killing", "yes"]).await?;
        assert_eq!(
            track(h, MAIN).await?,
            0,
            "source assignment via real framedNPC, cycle{cycle}"
        );
        let respawn_ms = scene
            .table
            .sources
            .iter()
            .flat_map(|s| s.points.iter())
            .map(|p| p.respawn_ms)
            .max()
            .ok_or("source delay")?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(
                u64::from(respawn_ms) + 1000,
            ))
            .ok_or("real source respawn clock")?;
        combat::respawn(owner).await?;
        for (i, (home, _)) in scene.homes.iter().enumerate() {
            combat::kill(h, owner, admitted, *home).await?;
            assert_eq!(
                track(h, MAIN).await?,
                i64::try_from(i + 1)?,
                "native source counter, cycle{cycle}"
            );
        }
        walk::walk(owner, admitted, gnomilly_cell)
            .await
            .map_err(|e| format!("Spike actual Gnomilly approach: {e}"))?;
        let before = progression(h).await?;
        talk(owner, admitted, gnomilly, &["hi", "report", "killing"]).await?;
        let after = progression(h).await?;
        assert_eq!(
            after.0,
            before.0 + 1000,
            "real report→obligation→matureXP fulfillment, cycle{cycle}"
        );
        report_crossed |= after.1 > before.1;
        if cycle == 0 {
            assert_eq!((before.1, after.1), (25, 26));
        }
        assert_eq!(track(h, MAIN).await?, -1);
        assert_eq!(track(h, FAME).await?, cycle + 1);
        let outstanding:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_xp_obligations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            outstanding, 0,
            "existingXP owner consumes actual obligation"
        );
        if cycle == 0 {
            let root_before:i64=sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            // Exact72000 boundary still refuses; it is strict Daily<now, not<=.
            source_clock(h, now + 72000).await?;
            talk(owner, admitted, gnomilly, &["hi", "killing", "yes"]).await?;
            assert_eq!(track(h, MAIN).await?, -1);
            assert_eq!(track(h, FAME).await?, 1);
            let root_after:i64=sqlx::query_scalar("SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                root_after, root_before,
                "cooldown refusal creates no Root history"
            );
        }
        now += 72001;
    }
    assert!(report_crossed);
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_spike_upper_kill_runs WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_spike_upper_kill_deaths WHERE character_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_character_xp_receipts xp JOIN game_spike_npc_children source ON source.xp_occurrence=xp.reward_occurrence_id WHERE source.character_id=encode($1,'hex')::uuid AND source.effect_kind='report')").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(counts, (100, 700, 100));
    let (commander, commander_cell) = scene::npc(owner, "oteryn:npc.gnommander").await?;
    walk::walk(owner, admitted, commander_cell).await?;
    let prior_xp = progression(h).await?;
    talk(
        owner,
        admitted,
        commander,
        &["hi", "worthy", "basic", "yes"],
    )
    .await?;
    assert_eq!(
        track(h, FAME).await?,
        0,
        "actual Gnommander source consumes100 earnedFame"
    );
    assert_eq!(
        progression(h).await?,
        prior_xp,
        "Fame spend does not mint XP"
    );
    let granted:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE granting_character_id=encode($1,'hex')::uuid AND unlock_key LIKE 'oteryn:appearance-grant/npc/gnommander/cave_explorer/%/base'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(granted, 1, "earned native source appearance");
    let observed = owner
        .appearance_observation(actor, session, Vec::new())
        .await
        .ok_or("real native wardrobe observation")?;
    let visible = observed;
    assert!(
        visible
            .catalogue
            .outfits
            .iter()
            .any(|o| o.key == "oteryn:content.outfit.cave_explorer_outfits"),
        "server publishes actual earned outfit through existing native appearance codec"
    );
    eprintln!(
        "SRV7_GNOMILLY_FULL_NATIVE_SOURCE_POSTGRES_EXECUTED historical_initial_level=25 preparation_real_deaths={prep} task_real_deaths=700 real_reports=100 real_xp_obligations_paid=100 earned_fame_spent=100 cross_level={report_crossed} elapsed={:.3}s",
        began.elapsed().as_secs_f64()
    );
    Ok(())
}
