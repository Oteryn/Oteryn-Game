use super::*;

pub(super) async fn partial_batches(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>, initial_xp: i64,
) -> TestResult {
    for (id, name) in [(3, "crocodiles"), (6, "badgers")] {
        wire::talk(owner, admitted, npc, &[(id, "hi"), (id + 1, name), (id + 2, "yes")]).await?;
    }
    seed(h, &[(quest::KILL_COUNTER_BASE + 1, 300), (quest::KILL_COUNTER_BASE + 2, 300)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    // A different committed predecessor arrives after the NPC's read-only copy. The
    // frozen report still contains both tasks, but the second native write must refuse.
    seed(h, &[(quest::QUEST_STORAGE_BASE + 2, -1), (quest::KILLS_STORAGE_BASE + 2, -1),
        (quest::KILL_COUNTER_BASE + 2, 0)]).await?;
    wire::talk_refused(owner, admitted, npc, &[(9, "hi"), (10, "tasks")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 0);
    assert_eq!(value(owner, authority, quest::REPEAT_STORAGE_BASE + 1).await?, 1);
    {
        let copies = owner.states.quest_sessions.lock().map_err(debug)?;
        let copy = copies.get(&admitted.game_session_id).and_then(|entry| entry.copy.as_ref()).ok_or("refreshed copy")?;
        assert_eq!(quest::storage_value(copy.tracks(), quest::QUEST_STORAGE_BASE + 1), 0);
        assert_eq!(quest::storage_value(copy.tracks(), quest::QUEST_STORAGE_BASE + 2), -1);
    }
    wire::talk(owner, admitted, npc, &[(11, "hi"), (12, "tasks")]).await?;
    let complete: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-1'")
        .fetch_one(&h.pool).await?;
    assert_eq!(complete, 1, "the next utterance cannot repeat the first completion");

    // Start two genuine new runs, then stop the second write after the first already
    // committed. This is a transport-unknown outcome, not a permanent rule refusal.
    for (id, name) in [(13, "crocodiles"), (16, "badgers")] {
        wire::talk(owner, admitted, npc, &[(id, "hi"), (id + 1, name), (id + 2, "yes")]).await?;
    }
    seed(h, &[(quest::KILL_COUNTER_BASE + 1, 300), (quest::KILL_COUNTER_BASE + 2, 300)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    sqlx::query("CREATE FUNCTION grizzly_npc_fail_second_report() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.transition_key='oteryn:quest-transition/grizzly/complete-2' THEN RAISE EXCEPTION 'fixture interrupted second report'; END IF; RETURN NEW; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER grizzly_npc_interrupt_report BEFORE INSERT ON game_character_quest_receipts FOR EACH ROW EXECUTE FUNCTION grizzly_npc_fail_second_report()")
        .execute(&h.pool).await?;
    let interrupted = wire::talk_interrupted(owner, admitted, npc, &[(19, "hi"), (20, "tasks")]).await;
    sqlx::query("DROP TRIGGER grizzly_npc_interrupt_report ON game_character_quest_receipts").execute(&h.pool).await?;
    sqlx::query("DROP FUNCTION grizzly_npc_fail_second_report()").execute(&h.pool).await?;
    interrupted?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 0);
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 2).await?, 1);
    let original = crate::foundation::CommandRef::new(admitted.game_session_id,
        crate::foundation::CommandId::new(20).map_err(debug)?);
    assert!(owner.states.command_ingress.reconcilable(original));
    assert!(!owner.states.command_ingress.original_terminalized(original));
    assert!(crate::gameplay_transport::session_ingress::SessionCommandStream::open(
        Some(&owner.states.command_ingress), admitted).is_none(),
        "the unresolved original must block a fresh command stream");
    // Resending the unsequenced command resumes its original two actions, replaying the
    // first receipt and committing only the remaining second task before stream-open.
    wire::talk(owner, admitted, npc, &[(20, "tasks"), (20, "tasks")]).await?;
    assert!(owner.states.command_ingress.original_terminalized(original));
    assert!(!owner.states.command_ingress.reconcilable(original));
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 3);
    assert_eq!(value(owner, authority, quest::REPEAT_STORAGE_BASE + 1).await?, 2);
    assert_eq!(value(owner, authority, quest::REPEAT_STORAGE_BASE + 2).await?, 1);
    let (first, second, refused, experience): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-1'), \
         (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/grizzly/complete-2'), \
         (SELECT count(*) FROM game_character_grizzly_refusals WHERE transition_key='oteryn:quest-transition/grizzly/complete-2'), \
         (SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid)",
    ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((first, second, refused), (2, 1, 1));
    assert_eq!(experience, initial_xp + 2_100);

    // An unresolved operation belonging to another writer blocks the generic
    // confirmation gate before Grizzly creates Pending. This new command is a
    // definitive refusal; it must not strand another outstanding reservation.
    owner.refresh_level_up_vitals(admitted.runtime_actor.ok_or("live actor")?, admitted.game_session_id).await;
    assert!(!owner.states.level_refresh.is_marked(admitted.item_fence.ok_or("item fence")?.character_id));
    // A new stream first reconciles possible retained NPC/coin-exchange work. Park
    // this unrelated marker only after hi and the task choice are acknowledged,
    // so this regression exercises the fresh confirmation gate rather than that
    // earlier recovery gate's legitimate unknown-outcome disconnect.
    let refused = wire::talk_refused_with_parked_confirmation(owner, admitted, npc,
        &[(21, "hi"), (22, "crocodiles"), (23, "yes")]).await;
    let unresolved = owner.spell_lane.acquire().await.err().ok_or("fixture attempt stays parked")?;
    let (permit, attempt) = unresolved.into_resolution();
    assert!(attempt.is::<()>(), "only the fixture's proven uncommitted marker is removed");
    drop(permit);
    refused?;
    let original = crate::foundation::CommandRef::new(admitted.game_session_id,
        crate::foundation::CommandId::new(23).map_err(debug)?);
    assert!(owner.states.command_ingress.original_terminalized(original));
    assert!(!owner.states.command_ingress.reconcilable(original));
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 0);
    wire::talk(owner, admitted, npc, &[(24, "hi"), (25, "crocodiles"), (26, "yes")]).await?;
    assert_eq!(value(owner, authority, quest::QUEST_STORAGE_BASE + 1).await?, 1);
    Ok(())
}
