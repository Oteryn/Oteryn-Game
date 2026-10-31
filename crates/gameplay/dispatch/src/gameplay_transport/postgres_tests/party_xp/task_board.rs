//! P04/P05 qualification through the existing real-party/real-swing fixture.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    kill_reward::{self, PendingKillSettlement},
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_quests::quest::task_board::{
    self,
    full::{BountyBoard, BountyStatus, BountyTask, TaskBoardData, WeeklyBoard, WeeklyKillTask},
};

#[path = "bounty_talisman.rs"]
mod talisman;
pub(super) use talisman::qualify_talisman;

#[path = "task_board_ordered.rs"]
mod ordered;
pub(super) use ordered::qualify_ordered_deaths;

/// Supply a previously accepted board as historical fixture input. Every kill,
/// revision and replay below goes through the production fenced dispatch writer.
pub(super) async fn install_boards(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
    pin: &ChannelContentPin,
) -> TestResult {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        task_board::extend(lowered.catalogue()).map_err(debug)?,
    ));
    sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind("party-task-board-test-map")
        .bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .execute(&h.pool).await?;
    let period: i64 = sqlx::query_scalar("SELECT game_task_board_period(encode($1,'hex')::uuid)")
        .bind(owner.world_id.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
    let data = TaskBoardData {
        weekly: Some(WeeklyBoard {
            period,
            difficulty: 0,
            any_required: 50,
            any_kills: 0,
            kills: ["rat", "cave_rat", "troll", "goblin", "orc"]
                .into_iter()
                .map(|name| WeeklyKillTask {
                    race: format!("oteryn:creature.{name}"),
                    required: 50,
                    current: 0,
                })
                .collect(),
            deliveries: Vec::new(),
            finished: false,
        }),
        bounty: BountyBoard {
            state: BountyStatus::Active,
            active: Some(BountyTask {
                race: "oteryn:creature.rat".into(),
                required: 25,
                current: 0,
                experience: 1850,
                points: 25,
                grade: 0,
                difficulty: 0,
                slot: 0,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let body = serde_json::to_value(data)?;
    for &(_, session) in players {
        owner.refresh_quest_session(session, true).await;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "quest fence read")?
            .ok_or("quest fence")?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_task_boards(character_id,world_id,body,committed_character_revision,cause_id,cause_ordinal) SELECT character_id,encode($2,'hex')::uuid,$3,character_revision,encode($4,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice())
            .bind(&body).bind(session.as_bytes().as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    Ok(())
}

pub(super) async fn capture_pending(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult<PendingKillSettlement> {
    let taken = {
        let attack = owner.states.attack.lock().await;
        let session = players
            .iter()
            .find(|(_, session)| attack.kills().has_session(*session))
            .map(|(_, session)| *session)
            .ok_or("queued task-board death")?;
        match attack.kills().take(session, 2) {
            kill_reward::TakeOutcome::Taken(taken) => taken,
            _ => return Err("queued task-board death".into()),
        }
    };
    let entry = taken.entry().ok_or("pending task-board death")?.clone();
    taken.requeue();
    Ok(entry)
}

/// A shared member with no damage and no own queue must still wait for the
/// leader's older death before its terminal-session release can be sent.
pub(super) async fn seal_shared_member_after_death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
    entry: &PendingKillSettlement,
) -> TestResult {
    let member = players[1].1;
    assert_ne!(entry.facts.principal.session, member);
    assert!(!owner.states.attack.lock().await.kills().has_session(member));
    assert!(
        !entry
            .damage_shares
            .iter()
            .any(|share| share.total > 0 && share.session == Some(member))
    );
    let release = owner.seal_kill_rewards(member).await;
    let kill_reward::KillRelease::Send(Some(mark)) = release else {
        return Err("shared member release must first settle the leader's queued death".into());
    };
    owner
        .settle_kill_release(Some(mark), kill_reward::ReleaseEnd::Retryable)
        .await;
    assert_eq!(owner.states.attack.lock().await.kills().counts().0, 0);
    let recipients = entry
        .task_board_recipients
        .get()
        .ok_or("frozen shared release recipients")?;
    assert!(
        recipients
            .iter()
            .any(|recipient| recipient.fence.game_session_id == member)
    );
    assert_eq!(
        ledger(h).await?.1.len(),
        players.len(),
        "both credits commit before a non-damaging member can release its session"
    );
    println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: shared_member_release_waits_for_leader_death");
    Ok(())
}

/// This member has generated offers but no selected bounty when the leader kills.
pub(super) async fn prepare_selection(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    session: GameSessionId,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let body: serde_json::Value = sqlx::query_scalar(
        "SELECT body FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut *tx)
    .await?;
    let mut data: TaskBoardData = serde_json::from_value(body)?;
    let task = data.bounty.active.take().ok_or("historical bounty")?;
    data.bounty.state = BountyStatus::Selection;
    data.bounty.offers = [
        "oteryn:creature.rat",
        "oteryn:creature.cave_rat",
        "oteryn:creature.troll",
    ]
    .into_iter()
    .zip(0_u8..)
    .map(|(race, slot)| BountyTask {
        race: race.into(),
        slot,
        ..task.clone()
    })
    .collect();
    sqlx::query(
        "UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(serde_json::to_value(data)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn select_after_shared_death(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
    entry: PendingKillSettlement,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::connection::{
        AdmittedSession, FirstEntryOutcome, SessionContinuity,
    };
    use crate::gameplay_transport::task_board::TaskBoardReply;
    use oteryn_protocol_oteryn::task_board as wire;

    let (actor, session) = players[1];
    assert_ne!(entry.facts.principal.session, session);
    assert!(
        !entry
            .damage_shares
            .iter()
            .any(|share| share.total > 0 && share.session == Some(session)),
        "the command sender must be a shared member with no damage to this creature"
    );
    assert!(
        !owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(session)
    );
    assert!(
        owner
            .states
            .attack
            .lock()
            .await
            .kills()
            .has_session(entry.facts.principal.session)
    );
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = fence.connection_generation.get();
    continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[1, wire::CAPABILITY_TASK_BOARD_V1],
    )
    .ok_or("task-board capability")?;
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity,
        item_fence: Some(fence.item_fence()),
    };
    let payload = wire::full::encode_request(&wire::full::Request {
        action: 8,
        selection: 0,
        race: String::new(),
    })
    .map_err(debug)?;
    let TaskBoardReply::Payload(bytes) = owner
        .task_board_wire(admitted, 2_000_000, wire::COMMAND_TYPE_TASK_BOARD, &payload)
        .await
    else {
        return Err("shared member's selection must finish after draining the leader".into());
    };
    let reply = wire::full::decode_reply(&bytes).map_err(debug)?;
    assert_eq!(
        (reply.refusal, reply.bounty.state, reply.bounty.current),
        (0, 2, 0)
    );
    assert_eq!(owner.states.attack.lock().await.kills().counts().0, 0);
    let recipients = entry
        .task_board_recipients
        .get()
        .ok_or("frozen recipients")?;
    assert_eq!(recipients.len(), 2);
    assert!(
        recipients
            .iter()
            .any(|recipient| recipient.fence.game_session_id == session)
    );
    let before = ledger(h).await?;
    assert_eq!(
        before.1.len(),
        players.len(),
        "both death receipts precede selection"
    );
    for (character, body) in &before.0 {
        let data: TaskBoardData = serde_json::from_value(body.clone())?;
        assert_eq!(data.weekly.ok_or("weekly task")?.any_kills, 1);
        assert_eq!(
            data.bounty.active.ok_or("active bounty")?.current,
            u32::from(character.as_slice() != fence.character_id.as_bytes())
        );
    }
    let (_, _, reply) = before
        .1
        .iter()
        .find(|(character, _, _)| character.as_slice() == fence.character_id.as_bytes())
        .ok_or("shared member's prior death receipt")?;
    let historical: crate::durability::task_board::TaskBoardView =
        serde_json::from_value(reply.clone())?;
    assert_eq!(historical.data.bounty.state, BountyStatus::Selection);
    assert!(historical.data.bounty.active.is_none());
    let principal = entry.facts.principal.session;
    assert_eq!(
        owner.states.attack.lock().await.kills().append(entry),
        kill_reward::AppendOutcome::Queued
    );
    owner.revision_sequencer = Default::default();
    assert_eq!(
        owner.drain_kill_rewards(principal).await,
        kill_reward::DrainOutcome::Drained
    );
    assert_eq!(
        ledger(h).await?,
        before,
        "replaying the prior death cannot credit the newly selected bounty"
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: shared_member_selection_drains_leader_before_new_bounty"
    );
    Ok(())
}

type Ledger = (
    Vec<(Vec<u8>, serde_json::Value)>,
    Vec<(Vec<u8>, Vec<u8>, serde_json::Value)>,
    Vec<(Vec<u8>, String)>,
);

async fn ledger(h: &Harness) -> TestResult<Ledger> {
    Ok((
        sqlx::query_as("SELECT uuid_send(character_id),body FROM game_character_task_boards ORDER BY character_id")
            .fetch_all(&h.pool).await?,
        sqlx::query_as("SELECT uuid_send(character_id),uuid_send(cause_id),reply FROM game_task_board_receipts WHERE cause_kind='creature_death' ORDER BY character_id,cause_id")
            .fetch_all(&h.pool).await?,
        sqlx::query_as("SELECT uuid_send(character_id),character_revision::text FROM game_character_roots ORDER BY character_id")
            .fetch_all(&h.pool).await?,
    ))
}

pub(super) async fn assert_replay(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
    entry: PendingKillSettlement,
    shared_without_damage: bool,
) -> TestResult {
    let frozen = entry
        .task_board_recipients
        .get()
        .ok_or("frozen task-board recipients")?;
    assert_eq!(frozen.len(), players.len());
    let mut characters = std::collections::BTreeSet::new();
    for &(actor, session) in players {
        let runtime = owner.runtime.lock().await;
        let character = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?
            .character_id();
        characters.insert(*character.as_bytes());
    }
    assert_eq!(
        frozen
            .iter()
            .map(|r| *r.fence.character_id.as_bytes())
            .collect::<std::collections::BTreeSet<_>>(),
        characters
    );
    let contributors = entry
        .damage_shares
        .iter()
        .filter(|share| share.total > 0)
        .map(|share| *share.character.as_bytes())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        contributors.len(),
        if shared_without_damage { 1 } else { 3 },
        "the shared-member case must really include a non-damaging member"
    );
    let before = ledger(h).await?;
    assert_eq!(
        (before.0.len(), before.1.len()),
        (players.len(), players.len())
    );
    for (_, body) in &before.0 {
        let data: TaskBoardData = serde_json::from_value(body.clone())?;
        let weekly = data.weekly.ok_or("weekly task")?;
        assert_eq!(weekly.any_kills, 1);
        assert_eq!(
            weekly
                .kills
                .iter()
                .find(|task| task.race == "oteryn:creature.rat")
                .ok_or("rat task")?
                .current,
            1
        );
        assert_eq!(data.bounty.active.ok_or("bounty task")?.current, 1);
    }
    let session = entry.facts.principal.session;
    assert_eq!(
        owner.states.attack.lock().await.kills().append(entry),
        kill_reward::AppendOutcome::Queued
    );
    owner.revision_sequencer = Default::default();
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        kill_reward::DrainOutcome::Drained
    );
    assert_eq!(
        ledger(h).await?,
        before,
        "recovered sequencer and retained death change no counters, receipts or Character revision"
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: frozen_party_and_damage_recipients_replay={}",
        players.len()
    );
    Ok(())
}
