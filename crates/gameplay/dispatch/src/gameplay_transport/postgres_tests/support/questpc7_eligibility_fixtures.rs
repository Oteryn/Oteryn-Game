use super::fixture::{Harness, debug, id};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

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
    if name == "questpc7-eligibility-make-believe-high" {
        return make_believe_contacts(owner, admitted, h).await;
    }
    let [elane, wayland] = super::classic_quests_fixtures::install(
        owner,
        admitted,
        ["oteryn:npc.elane", "oteryn:npc.wayland_smythers"],
    )
    .await?;
    super::classic_quests_fixtures::talk(owner, admitted, elane, 1300, "hi").await?;
    let before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let result =
        super::qengine_rewards_fixtures::talk(owner, admitted, elane, 1301, "hunter commission")
            .await?;
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
        }
        super::classic_quests_fixtures::talk(owner, admitted, wayland, 1310, "hi").await?;
        if name == "questpc7-eligibility-outfit-high" {
            let parent_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_quest_states
                 WHERE character_id=encode($1,'hex')::uuid
                 AND quest_key='oteryn:quest.authored.make_believe_quest')",
            )
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert!(!parent_exists, "fresh character has no Make Believe state");
            for command in [1311, 1312] {
                let menu = super::classic_quests_fixtures::talk(
                    owner, admitted, wayland, command, "outfit",
                )
                .await?
                .join(" ")
                .to_ascii_lowercase();
                for option in ["basic", "insignia", "halo"] {
                    assert!(
                        menu.contains(option),
                        "outfit request {command} missing {option}: {menu}"
                    );
                }
            }
            let receipts: Vec<String> = sqlx::query_scalar(
                "SELECT transition_key FROM game_character_quest_receipts
                 WHERE character_id=encode($1,'hex')::uuid
                 AND quest_key='oteryn:quest.authored.illuminated_warrior_outfits_quest'",
            )
            .bind(id(41).as_slice())
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(
                receipts,
                ["oteryn:quest-reward/questpc7-illuminated-intro"],
                "the second outfit request is read-only"
            );
            return Ok(());
        }
        let intro =
            super::qengine_rewards_fixtures::talk(owner, admitted, wayland, 1311, "outfit").await?;
        if name.ends_with("high") {
            assert_eq!(
                intro,
                NpcIntentDisposition::Ok,
                "Wayland starts without Make Believe"
            );
            // Supply the full commission's payment separately from the boss loot tests.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "WITH backpack AS (
                    SELECT * FROM game_item_container_slots
                    WHERE character_id=encode($1,'hex')::uuid
                 ), bags AS (
                    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                        definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                    SELECT uuidv7(),world_id,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,uuidv7()
                    FROM backpack CROSS JOIN generate_series(1,3)
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
                    FROM (SELECT item_instance_id, row_number() OVER (ORDER BY item_instance_id) AS index FROM placed) p
                    CROSS JOIN backpack b CROSS JOIN LATERAL generate_series(1, 16 + (p.index < 3)::integer) n
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
            for (command, stage, addon, remaining) in [
                (1312, "basic", 0, 2500),
                (1314, "insignia", 1, 1250),
                (1316, "halo", 2, 0),
            ] {
                for (id, words) in [(command, stage), (command + 1, "yes")] {
                    let result =
                        super::qengine_rewards_fixtures::talk(owner, admitted, wayland, id, words)
                            .await?;
                    assert_eq!(result, NpcIntentDisposition::Ok, "Wayland {stage}: {words}");
                }
                let scraps: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
                     WHERE definition_production_key='oteryn:item.tibia.i54640' AND lifecycle=1",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(
                    scraps, remaining,
                    "Wayland {stage} charges its exact scrap price"
                );
                let granted: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks
                     WHERE unlock_key=$1)",
                )
                .bind(format!(
                    "oteryn:outfit.illuminated_warrior/quest-grant/{addon}"
                ))
                .fetch_one(&h.pool)
                .await?;
                assert!(
                    granted,
                    "Wayland {stage} grants the corresponding appearance"
                );
            }
            let completed: bool = sqlx::query_scalar(
                "SELECT completed_character_revision IS NOT NULL
                 FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid
                 AND quest_key='oteryn:quest.authored.illuminated_warrior_outfits_quest'",
            )
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert!(completed, "Wayland completes the commission after halo");
            let parent_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_quest_states
                 WHERE character_id=encode($1,'hex')::uuid
                 AND quest_key='oteryn:quest.authored.make_believe_quest')",
            )
            .bind(id(41).as_slice())
            .fetch_one(&h.pool)
            .await?;
            assert!(!parent_exists, "Wayland leaves Make Believe untouched");
        } else {
            assert_eq!(
                intro,
                NpcIntentDisposition::Rejected,
                "Wayland retains the level gate"
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

async fn make_believe_contacts(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::quest_log::QuestLogObservation;
    use oteryn_gameplay_npc::npc_command::RATE_WINDOW_MICROS;
    use oteryn_protocol_oteryn::npc_service::{
        CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, MAX_TALK_INTENTS_PER_SECOND,
        NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
    };

    const QUEST: &str = "oteryn:quest.authored.make_believe_quest";
    const START: &str = "oteryn:quest-reward/make-believe/accept";
    const READY: &str = "oteryn:quest-reward/make-believe/storehouse-ready";
    const DEFENCE: &str = "oteryn:quest-reward/make-believe/salgadara-defence";
    let [wayland, pocaro] = super::classic_quests_fixtures::install(
        owner,
        admitted,
        ["oteryn:npc.wayland_smythers", "oteryn:npc.mayor_pocaro"],
    )
    .await?;
    let mut supported = admitted.continuity.selected_capabilities.as_slice().to_vec();
    supported.push(CAPABILITY_NPC_SERVICE_V1);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &supported)
            .ok_or("NPC capability selection")?;
    let actor = admitted.runtime_actor.ok_or("Make Believe player")?;
    let session = admitted.game_session_id;
    let words = [
        (pocaro, "hi"),
        (pocaro, "help"),
        (pocaro, "bye"),
        (wayland, "hi"),
        (wayland, "further"),
        (wayland, "yes"),
        (wayland, "infiltration"),
        (wayland, "yes"),
        (wayland, "further"),
        (wayland, "yes"),
        (wayland, "bye"),
        (pocaro, "hi"),
        (pocaro, "help"),
        (pocaro, "help"),
    ];
    let commands = words
        .into_iter()
        .zip(admitted.continuity.next_command_id..)
        .map(|((npc_actor, text), id)| {
            let payload = encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor,
                text: text.into(),
            })
            .map_err(debug)?;
            Ok((id, COMMAND_TYPE_NPC_TALK_INTENT, payload))
        })
        .collect::<TestResult<_>>()?;
    let replies = super::questl2_fixture::wire_sequence_observed(
        owner,
        admitted,
        commands,
        async |index, reply| {
            if matches!(index, 3..=7 | 11..=12) {
                assert_eq!(decode_npc_intent_result(reply).map_err(debug)?, NpcIntentDisposition::Ok);
            }
            let receipts = sqlx::query_scalar::<_, String>(
                "SELECT transition_key FROM game_character_quest_receipts
                 WHERE quest_key=$1 AND transition_key IN ($2, $3, $4)
                 ORDER BY committed_character_revision LIMIT 4",
            )
            .bind(QUEST)
            .bind(START)
            .bind(READY)
            .bind(DEFENCE)
            .fetch_all(&h.pool)
            .await?;
            let expected: &[&str] = match index {
                0..=4 => &[],
                5..=6 => &[START],
                7..=11 => &[START, READY],
                _ => &[START, READY, DEFENCE],
            };
            assert_eq!(receipts, expected, "progress after NPC command {index}");
            let QuestLogObservation::Changed(journal) =
                owner.quest_log_observation(actor, session, None).await
            else {
                return Err("Make Believe journal unavailable".into());
            };
            assert_eq!(
                journal.titles().iter().any(|(_, name)| name == "Make Believe"),
                !expected.is_empty(),
                "journal visibility follows mission acceptance",
            );
            assert!(journal.copy.quest_state(QUEST).is_none_or(|state| !state.completed));
            if (index + 1).is_multiple_of(MAX_TALK_INTENTS_PER_SECOND as usize) {
                // Wait for the production admission window to expire.
                tokio::time::sleep(std::time::Duration::from_micros(RATE_WINDOW_MICROS)).await;
            }
            Ok(())
        },
    )
    .await?;
    assert_eq!(replies.len(), words.len());
    Ok(())
}
