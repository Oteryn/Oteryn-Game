//! The three remaining NPC action families through real command7 and retained replies.
use super::*;
use quest::GrizzlyAction as Action;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let many = quest::boss(19).ok_or("Many source boss")?;
    let other = quest::boss(20).ok_or("Noxious Spawn source boss")?;
    // This existing level130/Elite-Hunter fixture has completed Demons. Only source
    // predecessor quest facts are seeded; every action under test uses command7.
    // NPC:776-779 opens the menu with a positive balance; 814-819 spends one point.
    seed(h, &[(quest::BOSS_POINTS_STORAGE, 1)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk_refused(owner, admitted, npc,
        &[(13, "hi"), (14, "bosses"), (15, "many"), (15, "many"),
          (16, "noxious spawn")]).await?;
    assert_eq!(value(owner, authority, many.storage).await?, 1);
    assert_eq!(value(owner, authority, many.kill_storage).await?, 0);
    assert_eq!(value(owner, authority, other.storage).await?, -1);
    assert_eq!(value(owner, authority, quest::BOSS_POINTS_STORAGE).await?, 0);
    // The ingress retains one terminal result. The successful choice replays above
    // before command16 retires it; the latest refusal also survives a new connection.
    wire::talk_refused(owner, admitted, npc, &[(16, "noxious spawn")]).await?;
    assert_eq!(receipts(h, admitted, 15, Action::SelectBoss(many.id)).await?, 1);
    assert_eq!(value(owner, authority, quest::BOSS_POINTS_STORAGE).await?, 0);
    assert_eq!(value(owner, authority, other.storage).await?, -1);
    let refused: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_grizzly_refusals WHERE cause_kind='command' AND cause_id=encode($1,'hex')::uuid AND cause_ordinal=16 AND transition_key=$2")
        .bind(admitted.game_session_id.as_bytes().as_slice())
        .bind(Action::SelectBoss(other.id).transition_key()).fetch_one(&h.pool).await?;
    assert_eq!(refused, 1);

    // NPC:389-395 reports the source entered-and-killed predecessor (2,1), resets
    // its kill counter and grants the canonical native achievement exactly once.
    seed(h, &[(many.storage, 2), (many.kill_storage, 1)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(17, "hi"), (18, "report"), (18, "report"), (19, "report")]).await?;
    assert_eq!(value(owner, authority, many.storage).await?, 3);
    assert_eq!(value(owner, authority, many.kill_storage).await?, 0);
    assert_eq!(receipts(h, admitted, 18, Action::ReportBoss(many.id)).await?, 1);
    let achievements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/one_less'")
        .fetch_one(&h.pool).await?;
    assert_eq!(achievements, 1);

    // NPC:915-943 selects the first matching Tiquanda/Demodras branch. Exercise
    // both starts and both reports through the same native special-task command.
    let tiquanda = quest::boss(29).ok_or("Tiquanda source boss")?;
    let demodras = quest::boss(30).ok_or("Demodras source boss")?;
    wire::talk(owner, admitted, npc, &[(20, "hi"), (21, "special"), (21, "special")]).await?;
    assert_eq!(value(owner, authority, tiquanda.storage).await?, 1);
    assert_eq!(value(owner, authority, tiquanda.kill_storage).await?, 0);
    assert_eq!(value(owner, authority, demodras.storage).await?, -1);
    seed(h, &[(tiquanda.storage, 2), (tiquanda.kill_storage, 1)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(22, "hi"), (23, "special"), (23, "special")]).await?;
    assert_eq!(value(owner, authority, tiquanda.storage).await?, 3);
    assert_eq!(value(owner, authority, tiquanda.kill_storage).await?, 1);
    wire::talk(owner, admitted, npc, &[(24, "hi"), (25, "special"), (25, "special")]).await?;
    assert_eq!(value(owner, authority, demodras.storage).await?, 1);
    assert_eq!(value(owner, authority, demodras.kill_storage).await?, 0);
    seed(h, &[(demodras.storage, 2), (demodras.kill_storage, 1)]).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    wire::talk(owner, admitted, npc, &[(26, "hi"), (27, "special"), (27, "special")]).await?;
    assert_eq!(value(owner, authority, demodras.storage).await?, 3);
    assert_eq!(value(owner, authority, demodras.kill_storage).await?, 1);
    wire::talk_refused(owner, admitted, npc, &[(28, "hi"), (29, "special")]).await?;
    for command in [21, 23, 25, 27] {
        assert_eq!(receipts(h, admitted, command, Action::StartSpecialTask).await?, 1);
    }
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 100);
    assert_eq!(value(owner, authority, quest::BOSS_POINTS_STORAGE).await?, 0);
    assert_eq!(value(owner, authority, tiquanda.storage).await?, 3);
    assert_eq!(value(owner, authority, demodras.storage).await?, 3);
    eprintln!("GRIZZLY_NPC_BOSS_DIALOGUE native selection, report, special branches and historical reply replay verified");
    Ok(())
}

async fn receipts(h: &Harness, admitted: &AdmittedSession, command: i64, action: Action) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE cause_kind='command' AND cause_id=encode($1,'hex')::uuid AND cause_ordinal=$2::text::numeric AND transition_key=$3")
        .bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string())
        .bind(action.transition_key()).fetch_one(&h.pool).await?)
}
