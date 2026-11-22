//! Exercise the real NPC dialogue, item custody, MINT and quest receipt owners.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::{
    npc_service::{NpcIntentDisposition, NpcTalkIntent},
    world_spatial_entities::EntityRef,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const NPC: &str = "oteryn:npc.wyda";
const QUEST: &str = "oteryn:quest.fixture.wyda_exchange";
const TRANSITION: &str = "oteryn:quest-reward/weighted-wyda";
const TRACK: &str = "oteryn:quest-progress/fixture/wyda-exchange";
const HERB: &str = "oteryn:item.tibia.i3734";
const SHOVEL: &str = "oteryn:item.tibia.i3457";
const ROPE: &str = "oteryn:item.tibia.i3003";
const SHOVEL_REPLY: &str = "The herb will do. Here is a shovel.";
const ROPE_REPLY: &str = "Take this rope in exchange for the herb.";

#[test]
fn weighted_npc_exchange_replays_committed_item_and_reply_after_lost_ack() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(false)
}

#[test]
fn weighted_npc_exchange_refuses_inactive_event_and_grants_once_per_active_epoch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(true)
}

fn authored(event: bool) -> TestResult<QuestStateCatalogue> {
    // The 70/30 split comes from Crystal 201189f8, wyda.lua:104–115.
    // Use two inventory-admitted items while the donor reward custody data is authored.
    // Grimvale supplies an existing live gate for the generic occurrence rule.
    let stages = json!([{"key":"exchange","completes":false,"trigger":{
        "kind":"npc_dialogue","callback":{
            "key":TRANSITION,"quest":QUEST,"npc":NPC,"keywords":["blood herb"],"topic":0,"next_topic":0,
            "storage":[{"track":TRACK,"minimum":1,"maximum":1,"set":if event {1} else {2},"preserve":event}],
            "take":[{"item":HERB,"count":1}],"inventory":[],"give":null,"outfit":null,
            "achievement":null,"reply":[],"failure_reply":["Sorry, not possible."],
            "sources":["crystal:201189f8:data-global/npc/wyda.lua:104-115"],
            "weighted_exchange":{
                "world_change":event.then_some("grimvale"),
                "outcomes":[
                    {"weight":70,"give":{"item":SHOVEL,"count":1},"reply":[SHOVEL_REPLY]},
                    {"weight":30,"give":{"item":ROPE,"count":1},"reply":[ROPE_REPLY]}
                ]
            }
        }
    }}]);
    let digest = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,"quests":[{"quest":QUEST,"recipe_sha256":digest,"stages":stages,
        "tracks":[{"key":TRACK,"initial":1,"minimum":1,"maximum":2}],
        "sources":["crystal:201189f8:data-global/npc/wyda.lua:104-115"]}]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("content revision")?;
    Ok(bindings.attach(&base)?)
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    id: u64,
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
    Ok(owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await)
}

async fn quantity(h: &super::fixture::Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}

async fn plans(h: &super::fixture::Harness) -> TestResult<Vec<Value>> {
    Ok(sqlx::query_scalar(
        "SELECT plan FROM game_quest_reward_sources WHERE transition_key=$1 ORDER BY cause_ordinal",
    )
    .bind(TRANSITION)
    .fetch_all(&h.pool)
    .await?)
}

fn scenario(event: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("weighted-npc-exchange", move || {
        let catalogue = authored(event)?;
        super::full_world_support::world_scenario!(
            "weighted_npc_exchange",
            |h,
             _request,
             seal,
             authority,
             world_id,
             _native,
             _world,
             _cells,
             _room,
             _pin,
             _deps,
             actor,
             session,
             owner,
             fence,
             admitted| {
                owner.quest_catalogue = Some(Arc::new(catalogue));
                owner.admit_quest_session(&admitted).await;
                exercise(&mut owner, &admitted, &h, fence, event).await?;
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            }
        )
    })
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &super::fixture::Harness,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    event: bool,
) -> TestResult {
    use super::full_world_support::ENTRY;
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, [NPC]).await?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence,
        &Offering {
            position: Position {
                x: u16::try_from(ENTRY.0)?,
                y: u16::try_from(ENTRY.1)?,
                floor: -7,
            },
            item: HERB.into(),
            count: 2,
        },
        6282,
        true,
    )
    .await?;
    assert_eq!(quantity(h, HERB).await?, 2);
    assert_eq!(
        talk(owner, admitted, npc, 1, "hi").await?.disposition,
        NpcIntentDisposition::Ok
    );
    if event {
        assert_eq!(
            talk(owner, admitted, npc, 2, "blood herb")
                .await?
                .disposition,
            NpcIntentDisposition::Rejected
        );
        assert!(plans(h).await?.is_empty());
        assert_eq!(quantity(h, HERB).await?, 2);
        activate_event(h, admitted.world_id, 1).await?;
    }
    owner
        .states
        .npc_conversations
        .fixture_interrupt_quest_dialogue_commit(admitted.game_session_id, 3)?;
    assert!(talk(owner, admitted, npc, 3, "blood herb").await?.unknown);
    let committed = plans(h).await?;
    let [plan] = committed.as_slice() else {
        return Err("expected one committed exchange".into());
    };
    let roll = plan["weighted_outcome"]["draw"]
        .as_i64()
        .ok_or("persisted draw")?;
    let (chosen, unchosen, reply) = match roll {
        1..=70 => (SHOVEL, ROPE, SHOVEL_REPLY),
        71..=100 => (ROPE, SHOVEL, ROPE_REPLY),
        _ => return Err("draw bounds".into()),
    };
    assert_eq!(plan["give"]["item"], chosen);
    assert_eq!(plan["weighted_outcome"]["reply"], json!([reply]));
    let replay = owner
        .resume_npc_bank(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 3,
                item_fence: Some(fence),
            },
        )
        .await
        .ok_or("retained NPC command")?;
    assert!(!replay.unknown);
    assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
    assert_eq!(
        json!(replay.conversation.ok_or("replayed NPC reply")?.lines),
        plan["weighted_outcome"]["reply"]
    );
    assert_eq!(plans(h).await?, committed);
    assert_eq!(quantity(h, HERB).await?, 1);
    assert_eq!(quantity(h, chosen).await?, 1);
    assert_eq!(quantity(h, unchosen).await?, 0);
    assert_eq!(
        talk(owner, admitted, npc, 4, "blood herb")
            .await?
            .disposition,
        NpcIntentDisposition::Rejected
    );
    assert_eq!(plans(h).await?, committed);
    assert_eq!(quantity(h, HERB).await?, 1);
    if event {
        activate_event(h, admitted.world_id, 2).await?;
        assert_eq!(
            talk(owner, admitted, npc, 5, "blood herb")
                .await?
                .disposition,
            NpcIntentDisposition::Ok
        );
        assert_eq!(plans(h).await?.len(), 2);
        assert_eq!(quantity(h, HERB).await?, 0);
        assert_eq!(quantity(h, SHOVEL).await? + quantity(h, ROPE).await?, 2);
    }
    Ok(())
}

async fn activate_event(
    h: &super::fixture::Harness,
    world: crate::foundation::WorldId,
    epoch: i64,
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,13,12,0,0,'UTC'),'2031-06')")
        .bind(world.as_bytes().as_slice()).bind(epoch).bind([1_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
    assert!(
        h.root
            .world_changes(world, u64::try_from(epoch)?)
            .await?
            .ok_or("active day")?
            .grimvale_full_moon
    );
    Ok(())
}
