//! M38 basic Rascalio contact through the live NPC command path; event ferrying remain separate.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

static EXERCISED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[test]
fn npcfix_m38_rascalio_basic_contact_and_stale_actor_are_fenced_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    EXERCISED.store(false, std::sync::atomic::Ordering::SeqCst);
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("npcfix-m38-rascalio", || {
        super::quest_world_callbacks::scenario(
            "npcfix-m38-rascalio",
            Position {
                x: 32260,
                y: 31642,
                floor: -7,
            },
            false,
        )
    })?;
    assert!(
        EXERCISED.load(std::sync::atomic::Ordering::SeqCst),
        "Rascalio scenario hook did not execute"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_RASCALIO duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    EXERCISED.store(true, std::sync::atomic::Ordering::SeqCst);
    eprintln!("POSTGRES_STARTED NPCFIX_M38 Rascalio basic source contact");
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.rascalio"]).await?;
    {
        let runtime = owner.runtime.lock().await;
        let actor = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| {
                n.actor.placement_identity() == npc.identity && n.generation == npc.generation
            })
            .ok_or("Rascalio actor")?;
        assert_eq!(
            (actor.position.x, actor.position.y, actor.position.floor),
            (32259, 31643, 7)
        );
    }
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        super::classic_quests::talk(owner, admitted, npc, 39800, "hi").await?,
        ["Be greeted, Quest witness!"]
    );
    assert_eq!(
        super::classic_quests::talk(owner, admitted, npc, 39801, "name").await?,
        ["I am known as Rascalio."]
    );
    assert_eq!(
        super::classic_quests::talk(owner, admitted, npc, 39801, "name").await?,
        ["I am known as Rascalio."]
    );
    assert_eq!(
        super::classic_quests::talk(owner, admitted, npc, 39802, "job").await?,
        ["My role here is Ferryman."]
    );
    assert!(
        super::classic_quests::talk(owner, admitted, npc, 39803, "passage")
            .await?
            .is_empty()
    );
    let mut stale = npc;
    stale.generation = stale.generation.checked_add(1).ok_or("generation")?;
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
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 39804,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: stale,
                text: "name".into(),
            },
        )
        .await;
    assert_ne!(result.disposition, NpcIntentDisposition::Ok);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        before, after,
        "basic dialogue must not charge a travel or exchange fee"
    );
    assert_eq!(owner.runtime.lock().await.visible_entities().npcs.len(), 1);
    Ok(())
}
