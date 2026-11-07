//! Ajax149-156: source request, condition and wait flag share one NPC transaction.
use super::*;
const STAGE: &str = "oteryn:quest-progress/quest/u7_8/barbarian_outfits/barbarian_addon";
const TIMER: &str = "oteryn:quest-progress/quest/u7_8/barbarian_outfits/barbarian_addon_wait_timer";
#[path = "ajax/periodic.rs"]
mod periodic;
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    exercise_internal(h, owner, admitted, npc, false).await
}
pub(super) async fn exercise_lifecycle(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    exercise_internal(h, owner, admitted, npc, true).await
}
async fn exercise_internal(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    lifecycle: bool,
) -> TestResult {
    let health_before = {
        let runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(
                &runtime,
                admitted.runtime_actor.ok_or("current NPC customer")?,
                admitted.game_session_id,
            )
            .ok_or("current native vitals")?
            .health
    };
    assert!(
        health_before > 100,
        "the real source actor survives all ten Fire ticks"
    );
    establish_predecessor_revision(owner, admitted.game_session_id).await?;
    // Controlled source predecessor only; all tested writes below use real NPC
    // command7, actual current Actor/Character/Node and source-program owners.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,'oteryn:quest.outfit_and_addon_quests',1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=excluded.value")
        .bind(id(41).as_slice()).bind(STAGE).execute(&mut *tx).await?;
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    // Preserve the genuine source conversation across all seven utterances.
    super::super::grizzly_npc::wire::talk_paced(
        owner,
        admitted,
        npc,
        &[
            (1, "hi"),
            (2, "mine"),
            (3, "no"),
            (4, "no"),
            (5, "no"),
            (6, "say please"),
            (7, "yes"),
        ],
    )
    .await?;
    let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(STAGE).fetch_one(&h.pool).await?;
    assert_eq!(
        stage, 2,
        "actual Ajax source yes must atomically install FIRE, stage2, one-hour wait and close"
    );
    let waiting:bool=sqlx::query_scalar("SELECT t.value=(c.before_state->>'now')::bigint+3600 FROM game_character_quest_tracks t JOIN game_npc_cosmetic_receipts r ON r.character_id=t.character_id AND r.source_key='oteryn:npc-cosmetic/ajax/barbarian-fire' JOIN game_npc_source_context c ON c.npc_transaction_id=r.transaction_id WHERE t.character_id=encode($1,'hex')::uuid AND t.track_key=$2").bind(id(41).as_slice()).bind(TIMER).fetch_one(&h.pool).await?;
    assert!(
        waiting,
        "source wait is one hour from the same authoritative NPC transaction"
    );
    let snapshot:serde_json::Value=sqlx::query_scalar("SELECT snapshot FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let conditions =
        crate::foundation::condition::ConditionStore::from_persistent_snapshot(&snapshot, 0)
            .map_err(debug)?;
    assert!(
        conditions
            .instances()
            .iter()
            .any(|c| c.definition().key() == "npc.ajax.source_fire"
                && matches!(
                    c.definition().values(),
                    oteryn_gameplay_domain::domain::condition::ConditionValues::DamageOverTime {
                        element: oteryn_gameplay_domain::domain::condition::DotElement::Fire,
                        total_min: 100,
                        total_max: 100,
                        per_tick: 10,
                        interval_ms: 2000,
                        delayed: true
                    }
                )),
        "same-XID real NPC receipt must own exact delayed100HP fire snapshot"
    );
    if lifecycle {
        return periodic::exercise(h, owner, *admitted, health_before).await;
    }
    // Replay the actual source command before any later action: it must neither
    // reset its 20-second condition nor add a second parent/child receipt.
    super::super::grizzly_npc::wire::talk(owner, admitted, npc, &[(7, "yes")]).await?;
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_npc_source_actor_effects WHERE character_id=encode($1,'hex')::uuid AND source_key='oteryn:npc-cosmetic/ajax/barbarian-fire'").bind(id(41).as_slice()).fetch_one(&h.pool).await?,1);
    Ok(())
}

// Registered Ajax CALLBACK_GREET53–62 is independent of the Fire writer.
pub(super) async fn exercise_greet(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    establish_predecessor_revision(owner, admitted.game_session_id).await?;
    let program =
        oteryn_gameplay_quests::quest::npc_cosmetics::source_program::embedded().map_err(debug)?;
    for (symbol, key) in [
        ("Storage.Quest.U7_8.BarbarianOutfits.BarbarianAddon", STAGE),
        (
            "Storage.Quest.U7_8.BarbarianOutfits.BarbarianAddonWaitTimer",
            TIMER,
        ),
    ] {
        assert_eq!(
            program
                .binding(symbol)
                .ok_or("source-owned Ajax binding")?
                .key,
            key
        );
    }
    // Controlled, source-valid historical predecessor: the earlier Fire episode
    // has expired and its one-hour WaitTimer is past. Never fabricate an actor
    // effect or native source receipt; only the callback writes are tested.
    let expired: i64 =
        sqlx::query_scalar("SELECT extract(epoch FROM clock_timestamp())::bigint-60")
            .fetch_one(&h.pool)
            .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (key, value) in [(STAGE, 2_i64), (TIMER, expired)] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,'oteryn:quest.outfit_and_addon_quests',$3,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=excluded.value")
            .bind(id(41).as_slice()).bind(key).bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    super::super::grizzly_npc::wire::talk(owner, admitted, npc, &[(1, "hello"), (2, "hi")]).await?;
    let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(STAGE).fetch_one(&h.pool).await?;
    assert_eq!(
        stage, 3,
        "actual fresh Ajax greeting after source hour must advance callback stage2 to3"
    );
    // Both greetings share one genuine connection: repeated hi is not a new
    // source callback and cannot produce a second value-bearing parent.
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_npc_cosmetic_receipts WHERE character_id=encode($1,'hex')::uuid AND source_key='oteryn:npc-cosmetic/ajax/barbarian-greet'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?,1);
    let timer:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(TIMER).fetch_one(&h.pool).await?;
    assert_eq!(
        timer, expired,
        "greeting does not restart the source one-hour wait"
    );
    Ok(())
}

// Stamp a real historical predecessor through the mature Root XP owner.
async fn establish_predecessor_revision(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "actual predecessor fence")?
        .ok_or("source session")?;
    if fence.expected_character_revision.get() >= 2 {
        return Ok(());
    }
    let progression = owner
        .progression_binding(session)
        .ok_or("real progression binding")?;
    let award = crate::durability::character_progression::ExperienceAwardRequest {
        occurrence:
            crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(id(91))
                .map_err(debug)?,
        amount: oteryn_simulation_determinism::ExactI64::new(1),
        context: progression.context.clone(),
        policy_revision: progression.policy_revision.clone(),
        reward_revision: progression.reward_revision.clone(),
        policy: progression.policy.clone(),
    };
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            award,
            None,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        crate::durability::character_progression::ExperienceCommitOutcome::Committed(_)
    ));
    assert_eq!(
        owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "advanced fence")?
            .ok_or("advanced session")?
            .expected_character_revision
            .get(),
        2
    );
    Ok(())
}
