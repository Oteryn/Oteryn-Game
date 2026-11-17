//! M05: the production NPC caller commits the donor Gnomus reward atomically.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestTransitionOutcome, npc_program::NpcProgramRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{npc_rules::NpcRuleTrigger, world_callbacks::Position};

const QUEST: &str = "oteryn:quest.dangerous_depths_quest";
const PREFIX: &str = "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/";
const RULE: &str = "oteryn:quest-npc/dangerous_depths/gnomus/measurements-report";
const NPC: &str = "oteryn:npc.gnomus";

#[test]
fn m05_gnomus_dispatch_commits_item_points_deadline_and_bound_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m05-gnomus", || {
        super::quest_world_callbacks::scenario(
            "m05-gnomus",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!("M05_DISPATCH_PG_EXECUTED item_points_deadline_replay");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    // Accept Gnomus's actual measurement task before supplying its completed
    // field observations; preserve the real quest and progression receipt chain.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let setup = slot.commit_quest_transition(owner.root, authority, owner.holder, fence,
        crate::durability::quest_state::QuestTransitionRequest {
            transition_key: "oteryn:quest-transition/quest/u11_50/dangerous_depths/gnomes/measurements/npc_2".into(),
            cause: crate::durability::quest_state::QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(9499).map_err(debug)?)),
        }, owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone()).await.map_err(debug)?;
    let QuestTransitionOutcome::Committed(setup) = setup else {
        return Err("measurement task was not accepted".into());
    };
    assert_eq!(
        setup.committed_character_revision.get(),
        fence.expected_character_revision.get() + 1
    );
    assert_eq!(setup.changes.len(), 1);
    assert_eq!(setup.changes[0].track, format!("{PREFIX}measurements"));
    assert_eq!(setup.changes[0].after, 1);
    drop(slot);
    // Only the completed measurement prerequisites use fixture privileges. All
    // dialogue and reward mutations below run with production triggers enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, value) in [("location_count", 5_i64), ("points", 7)] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(format!("{PREFIX}{track}")).bind(QUEST).bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [gnomus] = super::classic_quests::install(owner, admitted, [NPC]).await?;
    super::classic_quests::talk(owner, admitted, gnomus, 9500, "hi").await?;
    let before: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    use crate::durability::item_loop_move::ItemLoopContent;
    assert!(
        weights
            .loop_facts(&crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i27654".into(),
                revision_ref: "definition-r1".into()
            })
            .is_some(),
        "M05 reward must be admitted in the native provider"
    );
    let reply = super::classic_quests::talk(owner, admitted, gnomus, 9501, "measurements").await?;
    assert!(
        reply
            .iter()
            .any(|line| line.contains("Excellent, you returned with more data!"))
    );
    let after: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    for (track, value) in [("measurements", 2), ("points", 8), ("location_count", 5)] {
        assert_eq!(
            super::classic_quests::progress(
                owner,
                admitted,
                authority,
                &format!("{PREFIX}{track}")
            )
            .await?,
            value
        );
    }
    let deadline = super::classic_quests::progress(
        owner,
        admitted,
        authority,
        &format!("{PREFIX}time_task_measurements"),
    )
    .await?;
    assert!((before + 72000..=after + 72000).contains(&deadline));
    let request = NpcProgramRequest {
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(9501).map_err(debug)?,
        ),
        npc: NPC.into(),
        trigger: NpcRuleTrigger::Say,
        message: "measurements".into(),
        topic: 0,
        transition_key: RULE.into(),
    };
    let replay = owner
        .root
        .reconcile_npc_program(authority, fence.character_id, request.clone())
        .await
        .map_err(debug)?
        .ok_or("NPC receipt")?;
    assert_eq!(replay.reply, reply);
    let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay.transition else {
        return Err("bound replay missing".into());
    };
    assert_eq!(receipt.changes.len(), 3);
    assert_eq!(
        receipt.committed_character_revision.get(),
        fence.expected_character_revision.get() + 1
    );
    let committed = snapshot(h).await?;
    assert_eq!(committed.1, 1, "exactly one native item reward");
    assert_eq!(committed.2, 1, "one source NPC receipt");
    let cooldown =
        super::classic_quests::talk(owner, admitted, gnomus, 9502, "measurements").await?;
    assert_eq!(
        cooldown,
        ["I don't need your help for now. Come back later."]
    );
    assert_eq!(
        snapshot(h).await?,
        committed,
        "cooldown cannot mint or write another revision"
    );
    let mut changed = request;
    changed.message = "different".into();
    assert!(
        owner
            .root
            .reconcile_npc_program(authority, fence.character_id, changed)
            .await
            .is_err()
    );
    assert_eq!(snapshot(h).await?, committed);
    Ok(())
}

async fn snapshot(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT character_revision::bigint FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid),(SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i27654'),(SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2)")
        .bind(id(41).as_slice()).bind(RULE).fetch_one(&h.pool).await?)
}
