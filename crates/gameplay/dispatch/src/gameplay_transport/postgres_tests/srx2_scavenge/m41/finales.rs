//! Both real source finales retain reward, achievement and progress in one transaction.
use super::quest_fixture as q;
use super::*;
use crate::durability::quest_state::QuestTransitionOutcome;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use std::sync::Arc;

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    catalogue: Arc<QuestStateCatalogue>,
) -> TestResult {
    // Earlier journey phases are fixture prerequisites, not a full-route qualification.
    q::ready(owner, catalogue.clone(), 4, 9030, q::STAG_READY).await?;
    let key = "oteryn:quest-npc/the_order_of_the_stag/eldoran_the_ambitious/news-reward";
    sqlx::raw_sql("CREATE FUNCTION m41_fail_finale() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'M41 forced achievement failure'; END$$; CREATE TRIGGER m41_fail_finale BEFORE INSERT ON game_account_achievement_grant_requests FOR EACH ROW EXECUTE FUNCTION m41_fail_finale()")
        .execute(&h.pool).await?;
    let before = footprint(h).await?;
    assert!(
        q::npc(
            owner,
            native,
            catalogue.clone(),
            5,
            9040,
            "oteryn:npc.eldoran_the_ambitious",
            "news",
            0,
            key
        )
        .await
        .is_err()
    );
    assert_eq!(
        footprint(h).await?,
        before,
        "achievement failure rolls back the actual sword mint and audit"
    );
    assert_eq!(skinning::quantity(h, "oteryn:item.tibia.i52824").await?, 0);
    let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(crate::durability::stag_beaks::STAGE).fetch_one(&h.pool).await?;
    assert_eq!(stage, 12);
    sqlx::raw_sql("DROP TRIGGER m41_fail_finale ON game_account_achievement_grant_requests; DROP FUNCTION m41_fail_finale()")
        .execute(&h.pool).await?;
    let stag = q::npc(
        owner,
        native,
        catalogue.clone(),
        5,
        9040,
        "oteryn:npc.eldoran_the_ambitious",
        "news",
        0,
        key,
    )
    .await?;
    assert!(
        matches!(stag.transition, QuestTransitionOutcome::Committed(_)),
        "{stag:?}"
    );
    assert_eq!(skinning::quantity(h, "oteryn:item.tibia.i52824").await?, 1);
    same_transaction(h, "oteryn:achievement/bastard", 9040).await?;
    q::ready(owner, catalogue.clone(), 6, 9045, q::KILMARESH_READY).await?;
    let key = "oteryn:quest-npc/m41-kilmaresh/alyxo/sculptor-reward";
    let kilmaresh = q::npc(
        owner,
        native,
        catalogue.clone(),
        7,
        9050,
        "oteryn:npc.alyxo",
        "yes",
        13,
        key,
    )
    .await?;
    assert!(
        matches!(kilmaresh.transition, QuestTransitionOutcome::Committed(_)),
        "{kilmaresh:?}"
    );
    assert_eq!(kilmaresh.next_topic, 14);
    assert_eq!(skinning::quantity(h, "oteryn:item.tibia.i31574").await?, 1);
    same_transaction(h, "oteryn:achievement/sculptor_apprentice", 9050).await?;
    let remains:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(q::REMAINS).fetch_one(&h.pool).await?;
    assert_eq!(remains, 1);
    let replay = q::npc(
        owner,
        native,
        catalogue.clone(),
        7,
        9050,
        "oteryn:npc.alyxo",
        "yes",
        13,
        key,
    )
    .await?;
    assert!(matches!(
        replay.transition,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let duplicate = q::npc(
        owner,
        native,
        catalogue,
        8,
        9051,
        "oteryn:npc.alyxo",
        "yes",
        13,
        key,
    )
    .await?;
    assert!(
        !matches!(
            duplicate.transition,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ),
        "an unchanged source storage write cannot mint again"
    );
    assert_eq!(skinning::quantity(h, "oteryn:item.tibia.i31574").await?, 1);
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT achievement_key FROM game_account_achievements ORDER BY achievement_key",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        keys,
        vec![
            "oteryn:achievement/bastard",
            "oteryn:achievement/sculptor_apprentice"
        ]
    );
    Ok(())
}

async fn same_transaction(h: &Harness, key: &str, command: i64) -> TestResult {
    let same:bool=sqlx::query_scalar("SELECT a.xmin=n.xmin AND a.xmin=q.xmin AND a.xmin=s.xmin FROM game_account_achievement_grant_requests a JOIN game_quest_npc_program_receipts n USING(character_id) JOIN game_character_quest_receipts q USING(character_id) JOIN game_quest_npc_item_sources s ON s.character_id=q.character_id AND s.cause_id=q.cause_id AND s.cause_ordinal=q.cause_ordinal AND s.transition_key=q.transition_key WHERE a.achievement_key=$1 AND n.command_id=$2 AND q.cause_ordinal=$2")
        .bind(key).bind(command).fetch_one(&h.pool).await?;
    assert!(
        same,
        "actual item source, quest, NPC reply and achievement share one transaction"
    );
    Ok(())
}
