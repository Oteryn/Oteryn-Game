use super::*;

pub(super) async fn live_level(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>, initial_xp: i64,
) -> TestResult {
    wire::talk(owner, admitted, npc, &[(3, "hi"), (4, "crocodiles"), (5, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 1);
    wire::talk(owner, admitted, npc, &[(6, "hi"), (7, "badgers"), (8, "yes")]).await?;
    seed(h, &[(quest::POINTS_STORAGE, 40), (quest::PROMOTION_STORAGE, 2),
        (quest::KILL_COUNTER_BASE + 1, 300)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(9, "hi"), (10, "tasks"), (10, "tasks")]).await?;
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 41);
    let durable: (i64, i64) = sqlx::query_as("SELECT level,total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(durable, (50, initial_xp + 800));
    seed(h, &[(quest::KILL_COUNTER_BASE + 2, 300)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
    // A failed native read must not let the old live level skip the now-due promotion.
    // Only this isolated fixture's runtime-table grant is withheld, then restored.
    sqlx::query("REVOKE SELECT ON game_character_progression_state FROM oteryn_game_runtime")
        .execute(&h.pool).await?;
    // Harness connects its retained root holder as the test administrator. Select
    // the actual runtime role, as the existing house-transfer read-failure fixture
    // does, so the revoked grant really makes this owner read unavailable.
    h.root.try_issue_semantic_pass()?.run(|holder, _| Box::pin(async move {
        sqlx::query("SET ROLE oteryn_game_runtime").execute(&mut **holder).await?;
        Ok(())
    })).await?;
    assert_eq!(crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        &h.root, authority, character,
    ).await, crate::gameplay_transport::spell_character_facts::CastFactsLoad::Unavailable,
        "the fixture must deny the real native cast-facts read");
    // Keep this failure case independent of whether the live cadence already ran.
    owner.states.level_refresh.mark([character]);
    let refused = wire::talk_refused(owner, admitted, npc, &[(11, "hi")]).await;
    sqlx::query("GRANT SELECT ON game_character_progression_state TO oteryn_game_runtime")
        .execute(&h.pool).await?;
    h.root.try_issue_semantic_pass()?.run(|holder, _| Box::pin(async move {
        sqlx::query("RESET ROLE").execute(&mut **holder).await?;
        Ok(())
    })).await?;
    refused?;
    assert!(owner.states.level_refresh.is_marked(character));
    assert!(owner.states.command_ingress.original_terminalized(crate::foundation::CommandRef::new(
        admitted.game_session_id, crate::foundation::CommandId::new(11).map_err(debug)?)));
    assert!(owner.states.command_ingress.resumable(admitted),
        "a pre-write refresh failure must not strand an unknown command");
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 2).await?, 1);
    let premature: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-2'")
        .fetch_one(&h.pool).await?;
    assert_eq!(premature, 0);
    // No vitals capability was selected. The next real NPC utterance must consume
    // the existing live level mark before evaluating the level-50 promotion branch.
    wire::talk(owner, admitted, npc, &[(12, "hi"), (13, "tasks"), (14, "promotion")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 2).await?, 1,
        "the source tasks utterance asks for promotion before completing the ready second task");
    assert_eq!(value(owner, authority, quest::PROMOTION_STORAGE).await?, 4);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    assert_eq!(states.get(&runtime, actor, admitted.game_session_id).ok_or("live state")?
        .character_facts().level, 50);
    drop(states); drop(runtime);
    let receipts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-1')")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipts, (1, 1));
    Ok(())
}
