use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

async fn complete_parent(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    complete_parent_key(owner, admitted, "oteryn:quest.authored.make_believe_quest").await
}

pub(super) async fn complete_parent_key(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    parent: &str,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    let slug = parent.rsplit('.').next().ok_or("parent key")?;
    let track_value = format!("oteryn:quest-progress/fixture/prior-{slug}");
    let key_value = format!("oteryn:quest-transition/fixture/prior-{slug}-completed");
    let track = track_value.as_str();
    let key = key_value.as_str();
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: track.into(),
                quest: parent.into(),
                initial: 0,
                min: 0,
                max: 1,
            }],
            vec![QuestTransition {
                key: key.into(),
                quest: parent.into(),
                effects: vec![QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Eq(0),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: true,
                experience: None,
        world_change: None,
            }],
        )
        .map_err(debug)?,
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(1390).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_ELIGIBILITY POSTGRES_STARTED version={version}");
    let [elane, wayland] = super::classic_quests_fixtures::install(
        owner,
        admitted,
        ["oteryn:npc.elane", "oteryn:npc.wayland_smythers"],
    )
    .await?;
    super::classic_quests_fixtures::talk(owner, admitted, elane, 1300, "hi").await?;
    let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let result =
        super::qengine_rewards_fixtures::talk(owner, admitted, elane, 1301, "hunter commission").await?;
    if name.ends_with("no-premium") {
        assert_eq!(
            result,
            NpcIntentDisposition::Rejected,
            "a premium-required quest cannot start with no native premium authority"
        );
    } else {
        assert_eq!(
            result,
            NpcIntentDisposition::Ok,
            "Current native premium authorizes the Hunter start"
        );
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        if name.ends_with("high") {
            assert_eq!(level, 1000);
        } else {
            assert!(level < 1000);
            complete_parent(owner, admitted).await?;
        }
        super::classic_quests_fixtures::talk(owner, admitted, wayland, 1310, "hi").await?;
        let refused =
            super::qengine_rewards_fixtures::talk(owner, admitted, wayland, 1311, "illuminated commission")
                .await?;
        assert_eq!(
            refused,
            NpcIntentDisposition::Rejected,
            "missing level or parent prerequisite must refuse"
        );
        if name.ends_with("high") {
            // Level and Premium are already satisfied. Try the exchange before the fixture
            // records Make Believe completion and verify that dispatch grants nothing.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx).await?;
            sqlx::query(
                "WITH backpack AS (
                    SELECT * FROM game_item_container_slots
                    WHERE character_id=encode($1,'hex')::uuid
                 ), bags AS (
                    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                        definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                    SELECT uuidv7(),world_id,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,uuidv7()
                    FROM backpack CROSS JOIN generate_series(1,2)
                    RETURNING item_instance_id,world_id
                 ), placed AS (
                    INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,
                        item_instance_id,placement_ordinal,placed_transaction_id)
                    SELECT b.character_id,b.world_id,b.item_instance_id,g.item_instance_id,
                        (SELECT COALESCE(max(e.placement_ordinal),0) FROM game_item_container_entries e
                         WHERE e.parent_item_instance_id=b.item_instance_id) + row_number() OVER (),uuidv7()
                    FROM bags g CROSS JOIN backpack b RETURNING item_instance_id
                 ), inputs AS MATERIALIZED (
                    SELECT p.item_instance_id AS bag, b.world_id, b.character_id,
                           uuidv7() AS item, n
                    FROM (SELECT item_instance_id, row_number() OVER () AS index FROM placed) p
                    CROSS JOIN backpack b CROSS JOIN LATERAL generate_series(1, 12 + (p.index % 2)) n
                 ), minted AS (
                    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                        definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                    SELECT item,world_id,'Item','oteryn:item.tibia.i54640','definition-r1',100,1,uuidv7()
                    FROM inputs RETURNING item_instance_id
                 )
                 INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,
                    item_instance_id,placement_ordinal,placed_transaction_id)
                 SELECT character_id,world_id,bag,item,n,uuidv7() FROM inputs
                 JOIN minted ON minted.item_instance_id=inputs.item",
            ).bind(id(41).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            for (command, words) in [(1313, "basic"), (1314, "yes")] {
                super::qengine_rewards_fixtures::talk(owner, admitted, wayland, command, words)
                    .await?;
            }
            let started: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_quest_states
                 WHERE character_id=encode($1,'hex')::uuid
                 AND quest_key='oteryn:quest.authored.illuminated_warrior_outfits_quest')",
            ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert!(!started, "Wayland cannot start or advance the commission before Make Believe");
            let granted: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks
                 WHERE unlock_key LIKE '%illuminated%')",
            ).fetch_one(&h.pool).await?;
            assert!(!granted, "Wayland must refuse the outfit grant before Make Believe");
            let scraps: i64 = sqlx::query_scalar(
                "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
                 WHERE definition_production_key='oteryn:item.tibia.i54640' AND lifecycle=1",
            ).fetch_one(&h.pool).await?;
            assert_eq!(scraps, 2500, "refusing the exchange must preserve all scraps");
            complete_parent(owner, admitted).await?;
            let parent_before:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.make_believe_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                super::qengine_rewards_fixtures::talk(
                    owner,
                    admitted,
                    wayland,
                    1315,
                    "illuminated commission"
                )
                .await?,
                NpcIntentDisposition::Ok
            );
            let parent_after:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.make_believe_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                parent_after, parent_before,
                "eligibility never rewrites its parent completion fact"
            );
        }
    }
    let after:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    if name.ends_with("no-premium") {
        assert_eq!(
            after, before,
            "denied eligibility creates no quest receipt or progress"
        );
    }
    Ok(())
}
