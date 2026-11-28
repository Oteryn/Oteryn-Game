use super::fixture::{Harness, debug};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};

use crate::foundation::{CommandId, CommandRef};

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const STAGE: &str = "oteryn:quest-progress/chosen/mystery-of-the-valley/stage";

pub(super) async fn exercise_intro(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<EntityRef> {
    eprintln!("POSTGRES_CASE_STARTED questpc8_mystery_lai after PostgreSQL fixture initialization");
    let [lai] = if owner
        .bundle_map()
        .is_some_and(|map| map.overlay.base().tile(33834, 32352, -4).is_some())
    {
        super::classic_quests_fixtures::install_positions(
            owner,
            admitted,
            ["oteryn:npc.lai"],
            [oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33834,
                y: 32352,
                floor: -4,
            }],
        )
        .await?
    } else {
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.lai"]).await?
    };
    talk(owner, admitted, lai, 5100, "hi").await?;
    talk(owner, admitted, lai, 5101, "statue").await?;
    assert_eq!(progress(h).await?, -1);
    // These are the Ice Islands prerequisites, through their durable transitions.
    for (command, key) in [
        (
            5110,
            "oteryn:quest-transition/quest/u8_0/the_ice_islands/mission06/npc_1",
        ),
        (
            5113,
            "oteryn:quest-transition/quest/u8_0/the_ice_islands/mission06/npc_8",
        ),
    ] {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        if command == 5113 {
            // The New Frontier's physical quest owner now guards the prison
            // route. Import its donor snapshot for this Lai-only scenario.
            let mut seed = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *seed)
                .await?;
            let seeded = sqlx::query(
                "INSERT INTO game_character_quest_tracks
                    (character_id, track_key, quest_key, value, committed_character_revision)
                 SELECT character_id, prereq.track, 'oteryn:quest.the_new_frontier_quest',
                        prereq.value, character_revision
                 FROM game_character_roots
                 CROSS JOIN (VALUES
                    ('oteryn:quest-progress/quest/u8_54/the_new_frontier/mission07/hidden_note', 1),
                    ('oteryn:quest-progress/quest/u8_54/the_new_frontier/questline', 23)
                 ) AS prereq(track, value)
                 WHERE character_id = encode($1,'hex')::uuid",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .execute(&mut *seed)
            .await?;
            assert_eq!(seeded.rows_affected(), 2);
            seed.commit().await?;
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
            talk(owner, admitted, lai, 5107, "statue").await?;
            assert_eq!(
                progress(h).await?,
                -1,
                "accepting the cure task does not unlock the sled"
            );
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                authority,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: key.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(command).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(outcome, QuestTransitionOutcome::Committed(_)),
            "{key}: {outcome:?}"
        );
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        if command == 5110 {
            talk(owner, admitted, lai, 5102, "statue").await?;
            assert_eq!(progress(h).await?, -1);
        }
    }
    let before_refusal = revision(owner, admitted, h).await?;
    let premium = owner.premium_coordinator.take();
    talk(owner, admitted, lai, 5103, "statue").await?;
    assert_eq!(progress(h).await?, -1);
    assert_eq!(revision(owner, admitted, h).await?, before_refusal);
    owner.premium_coordinator = premium;
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    talk(owner, admitted, lai, 5104, "statue").await?;
    assert_eq!(progress(h).await?, 1);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_program_receipts WHERE transition_key='oteryn:quest-npc/authored-mystery/lai/statue'")
        .fetch_one(&h.pool).await?;
    assert_eq!(before, 1);
    talk(owner, admitted, lai, 5104, "statue").await?;
    talk(owner, admitted, lai, 5105, "statue").await?;
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    talk(owner, admitted, lai, 5106, "statue").await?;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_program_receipts WHERE transition_key='oteryn:quest-npc/authored-mystery/lai/statue'")
        .fetch_one(&h.pool).await?;
    assert_eq!(before, after);
    assert_eq!(progress(h).await?, 1);
    let before_hint = revision(owner, admitted, h).await?;
    assert_lai_reply(
        owner,
        admitted,
        lai,
        5120,
        "fragment of a statue",
        "ice and snow",
    )
    .await?;
    assert_lai_reply(owner, admitted, lai, 5121, "legend", "three dragons").await?;
    assert_lai_reply(owner, admitted, lai, 5122, "yes", "snow and ice").await?;
    assert_eq!(revision(owner, admitted, h).await?, before_hint);
    Ok(lai)
}

pub(super) async fn assert_lai_reply(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    lai: EntityRef,
    command: u64,
    text: &str,
    expected: &str,
) -> TestResult {
    let result = talk(owner, admitted, lai, command, text).await?;
    let lines = result.conversation.ok_or("Lai conversation")?.lines;
    assert!(
        lines.iter().any(|line| line.contains(expected)),
        "{text}: {lines:?}"
    );
    Ok(())
}

async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(STAGE)
            .fetch_optional(&h.pool)
            .await?
            .unwrap_or(-1),
    )
}

pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    if command_id == 5104 {
        assert_eq!(
            result.disposition,
            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
        );
    }
    Ok(result)
}

async fn revision(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<String> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    Ok(sqlx::query_scalar("SELECT character_revision::text FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?)
}
