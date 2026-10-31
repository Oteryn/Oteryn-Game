//! Grizzly Adams uses command 7, its existing NPC conversation and the fenced quest writer.
//! Conversation choices are transient; confirmed actions and refusals have durable receipts.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::grizzly::NoGrizzlyPremium;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::grizzly_adams::{
    self as quest, GrizzlyAction, GrizzlyActor, GrizzlyState,
};

type Context = (u64, u64, String);

#[derive(Default)]
pub(super) struct Conversation {
    context: Option<Context>,
    topic: Topic,
    pending: Option<Pending>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
enum Topic {
    #[default]
    None,
    Start(u8),
    CancelMenu,
    Cancel(u8),
    BossMenu,
}

#[derive(Clone)]
struct Pending {
    character: CharacterId,
    cause: QuestCause,
    actions: Vec<GrizzlyAction>,
    intent: NpcTalkIntent,
    reply: String,
    context: Context,
}

struct Reply {
    actions: Vec<GrizzlyAction>,
    line: String,
}

impl Reply {
    fn text(line: impl Into<String>) -> Self {
        Self {
            actions: Vec::new(),
            line: line.into(),
        }
    }
    fn action(action: GrizzlyAction, line: impl Into<String>) -> Self {
        Self {
            actions: vec![action],
            line: line.into(),
        }
    }
}

pub(super) fn context(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
) -> Option<Context> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        session,
        intent,
    )?;
    (context.2 == quest::NPC).then_some(context)
}

/// A fresh transport snapshot deliberately clears visible NPC engagement. Only an
/// already retained, identical command can use its original conversation generation
/// here; the current book, exact NPC generation, actor and range are checked again.
fn retained_context(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
) -> Option<Context> {
    let visible = runtime.visible_entities();
    let speaker = visible
        .players
        .iter()
        .find(|player| player.actor == actor)?;
    let npc = visible.npcs.iter().find(|npc| {
        npc.actor.placement_identity() == intent.npc_actor.identity
            && npc.generation == intent.npc_actor.generation
    })?;
    if admission
        .states
        .chat
        .engaged(session)
        .is_some_and(|current| current != npc.actor)
        || runtime.npc_key(npc.actor) != Some(quest::NPC)
        || speaker.position.floor != npc.position.floor
        || speaker.position.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
        || speaker.position.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
    {
        return None;
    }
    let book = admission.states.chat.npc_talk()?;
    book.get(quest::NPC)?;
    Some((
        book.catalogue_revision(),
        admission.states.chat.npc_conversation_generation(session)?,
        quest::NPC.into(),
    ))
}

fn names(ids: &[u8]) -> String {
    ids.iter()
        .filter_map(|id| quest::task(*id))
        .map(|task| task.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Source topic order: grizzly_adams.lua:412-450,614-722,723-904.
fn dialogue(state: &GrizzlyState, actor: GrizzlyActor, topic: &mut Topic, text: &str) -> Reply {
    let text = text.trim().to_ascii_lowercase();
    if matches!(text.as_str(), "no" | "bye" | "farewell") {
        *topic = Topic::None;
        return Reply::text("All right. Ask me about tasks when you are ready.");
    }
    if text == "yes" {
        match std::mem::take(topic) {
            Topic::Start(id) => {
                return Reply::action(
                    GrizzlyAction::StartTask(id),
                    "Happy hunting! Return here when your task is complete.",
                );
            }
            Topic::Cancel(id) => {
                return Reply::action(
                    GrizzlyAction::CancelTask(id),
                    "Your task is cancelled. Its kill counter has been reset.",
                );
            }
            Topic::CancelMenu => {
                *topic = Topic::CancelMenu;
                return Reply::text(format!(
                    "Which task do you want to cancel: {}?",
                    names(&quest::started_tasks(state))
                ));
            }
            Topic::None if !quest::joined(state) => {
                return Reply::action(
                    GrizzlyAction::Join,
                    "Welcome to the Paw and Fur - Hunting Elite! Ask me for a task.",
                );
            }
            _ => return Reply::text("Choose a task before confirming it."),
        }
    }
    if text == "join" {
        *topic = Topic::None;
        return if quest::joined(state) {
            Reply::text("You are already a member. Ask me for tasks.")
        } else {
            Reply::action(
                GrizzlyAction::Join,
                "Welcome to the Paw and Fur - Hunting Elite! Ask me for a task.",
            )
        };
    }
    if matches!(text.as_str(), "hi" | "hello") {
        *topic = Topic::None;
        return Reply::text(if quest::joined(state) {
            "Welcome back! Ask me about tasks, status, promotion, bosses or special tasks."
        } else {
            "Would you like to join the Paw and Fur - Hunting Elite?"
        });
    }
    let started = quest::started_tasks(state);
    if matches!(text.as_str(), "points" | "rank") {
        *topic = Topic::None;
        let rank = match quest::rank(state) {
            1 => "Member",
            2 => "Huntsman",
            3 => "Ranger",
            4 => "Big Game Hunter",
            5 => "Trophy Hunter",
            6 => "Elite Hunter",
            _ => "Unranked",
        };
        return Reply::text(format!(
            "You have {} Paw and Fur points. Your rank is {rank}.",
            quest::storage_value(state, quest::POINTS_STORAGE).max(0)
        ));
    }
    if matches!(text.as_str(), "status" | "started") {
        if started.is_empty() {
            return Reply::text("You have not started any task yet.");
        }
        return Reply::text(
            started
                .iter()
                .filter_map(|id| quest::task(*id))
                .map(|task| {
                    format!(
                        "{}: {}/{}",
                        task.name,
                        quest::storage_value(state, quest::KILL_COUNTER_BASE + u32::from(task.id)),
                        task.required
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    if matches!(text.as_str(), "promotion" | "promotions") {
        *topic = Topic::None;
        return if quest::promotion_due(state, actor.level).is_some() {
            Reply::action(
                GrizzlyAction::Promote,
                "Congratulations! You have advanced to the next Paw and Fur rank.",
            )
        } else {
            Reply::text("You do not have the points and level required for another promotion.")
        };
    }
    if matches!(text.as_str(), "special" | "special task" | "special tasks") {
        // NPC:915-943 resets the topic after every eligible special-task branch.
        if actor.level >= 80 && quest::storage_value(state, quest::POINTS_STORAGE) >= 70 {
            *topic = Topic::None;
        }
        return Reply::action(
            GrizzlyAction::StartSpecialTask,
            "Your special hunting assignment is ready. Check your quest log.",
        );
    }
    if text == "cancel" {
        if started.is_empty() {
            return Reply::text("You have not started any task yet.");
        }
        *topic = Topic::CancelMenu;
        return Reply::text(format!(
            "Cancelling resets the kill counter. Which task: {}?",
            names(&started)
        ));
    }
    if let Some(task) = quest::task_by_name(&text) {
        if *topic == Topic::CancelMenu {
            if !started.contains(&task.id) {
                return Reply::text("You have not started that task.");
            }
            *topic = Topic::Cancel(task.id);
            return Reply::text(format!(
                "Cancel {} and reset its kill counter? Say yes or no.",
                task.name
            ));
        }
        // NPC:887-894 clears a pending choice when the player asks about an active
        // task. The earlier repeat checks (669-674) consume the first three brackets'
        // in-range names, except the library's unlisted "underwater quar" alias.
        if matches!(topic, Topic::Start(_))
            && started.contains(&task.id)
            && (task.maximum_level >= 130
                || actor.level > task.maximum_level
                || text == "underwater quar")
        {
            *topic = Topic::None;
            return Reply::text(format!(
                "You currently killed {}/{} {}.",
                quest::storage_value(state, quest::KILL_COUNTER_BASE + u32::from(task.id)).max(0),
                task.required,
                task.name
            ));
        }
        match quest::can_start_task(state, actor, task.id) {
            Ok(()) => {
                *topic = Topic::Start(task.id);
                return Reply::text(format!(
                    "Hunt {} {}. Do you want to start this task? Say yes or no.",
                    task.required, task.name
                ));
            }
            Err(error) => return Reply::text(refusal(error)),
        }
    }
    if matches!(
        text.as_str(),
        "task" | "tasks" | "mission" | "report" | "reports" | "boss" | "bosses"
    ) {
        if !quest::joined(state) {
            return Reply::text("You must join before accepting tasks.");
        }
        let Ok(catalogue) = quest::catalogue() else {
            return Reply::text("My task records are unavailable. Please try again.");
        };
        // checkZ rearms failed visits, then reports the first completed ordinary boss.
        let mut actions = Vec::new();
        for boss in catalogue
            .bosses
            .iter()
            .filter(|boss| boss.task_id.is_some())
        {
            if quest::storage_value(state, boss.storage) == 2 {
                match quest::storage_value(state, boss.kill_storage) {
                    0 => actions.push(GrizzlyAction::ReportBoss(boss.id)),
                    1 => {
                        actions.push(GrizzlyAction::ReportBoss(boss.id));
                        return Reply {
                            actions,
                            line: format!(
                                "You have defeated {}! Ask me for more tasks.",
                                boss.name
                            ),
                        };
                    }
                    _ => {}
                }
            }
        }
        if matches!(text.as_str(), "report" | "reports") {
            return Reply { actions, line: "There is no completed boss to report. Unfinished boss assignments are ready to try again.".into() };
        }
        if matches!(text.as_str(), "boss" | "bosses") {
            *topic = Topic::BossMenu;
            let points = quest::storage_value(state, quest::BOSS_POINTS_STORAGE).max(0);
            let bosses = catalogue
                .bosses
                .iter()
                .filter(|boss| boss.task_id.is_some())
                .filter(|boss| {
                    quest::plan_action(state, actor, &GrizzlyAction::SelectBoss(boss.id)).is_ok()
                })
                .map(|boss| boss.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Reply {
                actions,
                line: format!("You have {points} boss points. Available bosses: {bosses}."),
            };
        }
        if quest::promotion_due(state, actor.level).is_some() {
            return Reply {
                actions,
                line: "Ask me for a promotion before taking another task.".into(),
            };
        }
        let ready: Vec<_> = started
            .iter()
            .copied()
            .filter(|id| {
                quest::task(*id).is_some_and(|task| {
                    quest::storage_value(state, quest::KILL_COUNTER_BASE + u32::from(*id))
                        >= task.required
                })
            })
            .collect();
        if !ready.is_empty() {
            let line = format!(
                "You have completed {}. Your rewards are recorded; ask about bosses and new tasks.",
                names(&ready)
            );
            actions.extend(ready.into_iter().map(GrizzlyAction::CompleteTask));
            return Reply { actions, line };
        }
        if started.len() >= quest::ACTIVE_TASK_LIMIT {
            *topic = Topic::CancelMenu;
            return Reply {
                actions,
                line: format!(
                    "You have three unfinished tasks. Choose one to cancel: {}.",
                    names(&started)
                ),
            };
        }
        *topic = Topic::None;
        let tasks = catalogue
            .tasks
            .iter()
            .filter(|task| quest::can_start_task(state, actor, task.id).is_ok())
            .map(|task| task.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Reply {
            actions,
            line: format!("Available tasks: {tasks}. Choose the creature name."),
        };
    }
    if *topic == Topic::BossMenu
        && let Some(boss) = quest::boss_by_name(&text)
    {
        return Reply::action(
            GrizzlyAction::SelectBoss(boss.id),
            format!(
                "Your {} assignment is ready. Enter its hideout when you are prepared.",
                boss.name
            ),
        );
    }
    Reply::text("Ask me about tasks, status, cancel, promotion, bosses or special tasks.")
}

fn refusal(error: quest::GrizzlyRefusal) -> &'static str {
    use quest::GrizzlyRefusal as R;
    match error {
        R::Premium => "This task requires Premium.",
        R::Level => "That task is outside your level range.",
        R::Rank => "Your Paw and Fur rank is too low for this task.",
        R::NotJoined => "You must join before accepting tasks.",
        R::TaskLimit => "You already have three active tasks. Complete or cancel one first.",
        R::RepeatLimit => "You have already completed that task the maximum number of times.",
        R::AlreadyStarted => "You have already accepted or completed that task.",
        R::KillsIncomplete => "You have not killed enough creatures yet.",
        R::BossPoints => "You do not have enough boss points.",
        R::BossActive => "That boss assignment is already active.",
        _ => "That action is not available in your current quest state.",
    }
}

fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.grizzly.pending = None;
    }
}

fn forget_terminal(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()
        .and_then(|sessions| {
            sessions
                .get(&session)
                .and_then(|state| state.grizzly.pending.clone())
        });
    if let Some(Pending {
        cause: QuestCause::Command(command),
        ..
    }) = pending
        && admission
            .states
            .command_ingress
            .original_terminalized(command)
    {
        clear(admission, session);
    }
}

pub(super) async fn talk(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    expected: Context,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    forget_terminal(admission, command.game_session_id);
    // Quest XP uses the existing live level owner before source level-dependent
    // dialogue runs, including clients that did not select the vitals projection.
    admission
        .refresh_level_up_vitals(actor, command.game_session_id)
        .await;
    let Some(item_fence) = command.item_fence else {
        return rejected();
    };
    // A failed live refresh retains this mark. No Grizzly action has started yet,
    // so refuse this command instead of evaluating promotion/task rules at a stale level.
    if admission
        .states
        .level_refresh
        .is_marked(item_fence.character_id)
    {
        return rejected();
    }
    let premium = admission
        .session_premium_gate(command.game_session_id)
        .await
        .premium_current();
    let level = {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if context(admission, &runtime, actor, command.game_session_id, &intent)
            != Some(expected.clone())
            || states.is_dead(actor)
        {
            return rejected();
        }
        let Some(state) = states.get(&runtime, actor, command.game_session_id) else {
            return rejected();
        };
        state.character_facts().level
    };
    let tracks = {
        let Ok(sessions) = admission.states.quest_sessions.lock() else {
            return rejected();
        };
        let Some(copy) = sessions
            .get(&command.game_session_id)
            .and_then(|state| state.copy.as_ref())
        else {
            return rejected();
        };
        copy.tracks().clone()
    };
    let reply = {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let conversation = &mut sessions.entry(command.game_session_id).or_default().grizzly;
        if conversation.pending.is_some() {
            return rejected();
        }
        if conversation.context.as_ref() != Some(&expected) {
            conversation.context = Some(expected.clone());
            conversation.topic = Topic::None;
        }
        dialogue(
            &tracks,
            GrizzlyActor {
                level,
                premium,
                level_after_reward: None,
            },
            &mut conversation.topic,
            &intent.text,
        )
    };
    if reply.actions.is_empty() {
        return reward_reply(admission, actor, command, &intent, reply.line).await;
    }
    let (Ok(id), Some(catalogue)) = (
        CommandId::new(command.command_id),
        admission.quest_catalogue.as_ref(),
    ) else {
        return rejected();
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let mut slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return rejected();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if context(admission, &runtime, actor, command.game_session_id, &intent)
            != Some(expected.clone())
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
    }
    let cause = QuestCause::Command(CommandRef::new(command.game_session_id, id));
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        sessions
            .entry(command.game_session_id)
            .or_default()
            .grizzly
            .pending = Some(Pending {
            character: fence.character_id,
            cause,
            actions: reply.actions.clone(),
            intent: intent.clone(),
            reply: reply.line.clone(),
            context: expected,
        });
    }
    let progression = admission.progression_binding(command.game_session_id);
    for action in reply.actions {
        let outcome = match admission.premium_coordinator {
            Some(gate) => {
                slot.commit_grizzly_transition_with_progression(
                    admission.root,
                    admission.character,
                    admission.holder,
                    fence,
                    cause,
                    action,
                    std::sync::Arc::clone(catalogue),
                    gate,
                    progression.as_ref().map(|binding| &binding.policy),
                )
                .await
            }
            None => {
                slot.commit_grizzly_transition_with_progression(
                    admission.root,
                    admission.character,
                    admission.holder,
                    fence,
                    cause,
                    action,
                    std::sync::Arc::clone(catalogue),
                    &NoGrizzlyPremium,
                    progression.as_ref().map(|binding| &binding.policy),
                )
                .await
            }
        };
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {}
            Err(
                crate::durability::character_progression::CharacterProgressionError::Unavailable(_),
            ) => return unknown(),
            _ => {
                drop(slot);
                drop(_permit);
                return reject_batch(admission, command.game_session_id).await;
            }
        }
    }
    drop(slot);
    drop(_permit);
    admission
        .refresh_quest_session(command.game_session_id, false)
        .await;
    reward_reply(admission, actor, command, &intent, reply.line).await
}

async fn reward_reply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    mut line: String,
) -> NpcTalkOutcome {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let delivered = admission.deliver_grizzly_reward(actor, command).await;
    if delivered == Some(UseDisposition::NoRoom) {
        line.push_str(" Your item reward is waiting; make room and speak to me again.");
    }
    let mut reply = NpcConversations::bank_reply(intent, line);
    // An attempted MINT may have committed before its acknowledgement was lost.
    // Refresh the authoritative inventory on every attempt without claiming success.
    reply.inventory_changed = delivered.is_some();
    reply
}

/// Earlier actions in a source utterance may already be committed. A definitive
/// later refusal must refresh their tracks and drain their existing XP obligations,
/// otherwise the next utterance would repeat the stale first action indefinitely.
async fn reject_batch(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> NpcTalkOutcome {
    admission.refresh_quest_session(session, false).await;
    NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .grizzly
        .pending
        .clone()?;
    let id = CommandId::new(command.command_id).ok()?;
    if pending.cause != QuestCause::Command(CommandRef::new(command.game_session_id, id)) {
        return None;
    }
    let mut incomplete = false;
    for action in &pending.actions {
        match admission
            .root
            .reconcile_character_grizzly_transition(
                admission.character,
                pending.character,
                pending.cause,
                action.clone(),
            )
            .await
        {
            Ok(Some(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            )) => {}
            Ok(None) => incomplete = true,
            Err(_) => {
                return Some(NpcTalkOutcome {
                    unknown: true,
                    ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                });
            }
            _ => {
                return Some(reject_batch(admission, command.game_session_id).await);
            }
        }
    }
    if incomplete {
        // A source utterance may report several ready tasks. Resume its original frozen
        // batch under fresh authority, reusing each transition's immutable receipt key.
        let Some(permit) = admission.spell_lane_permit().await else {
            return Some(NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            });
        };
        let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
            return Some(NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            });
        };
        let catalogue = admission.quest_catalogue.as_ref()?;
        if fence.character_id != pending.character || Some(fence.item_fence()) != command.item_fence
        {
            drop(permit);
            return Some(reject_batch(admission, command.game_session_id).await);
        }
        let mut slot = admission
            .revision_sequencer
            .acquire(fence.character_id)
            .await;
        let current = {
            let runtime = admission.runtime.lock().await;
            let states = admission.states.spell_states.lock().await;
            retained_context(
                admission,
                &runtime,
                actor,
                command.game_session_id,
                &pending.intent,
            ) == Some(pending.context.clone())
                && !states.is_dead(actor)
                && states.spell_idle(&runtime, actor, command.game_session_id)
                && admission
                    .states
                    .player_trades
                    .physical_service(command.game_session_id)
                && runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
        };
        if !current {
            drop(slot);
            drop(permit);
            return Some(reject_batch(admission, command.game_session_id).await);
        }
        let progression = admission.progression_binding(command.game_session_id);
        for action in &pending.actions {
            let result = match admission.premium_coordinator {
                Some(gate) => {
                    slot.commit_grizzly_transition_with_progression(
                        admission.root,
                        admission.character,
                        admission.holder,
                        fence,
                        pending.cause,
                        action.clone(),
                        std::sync::Arc::clone(catalogue),
                        gate,
                        progression.as_ref().map(|binding| &binding.policy),
                    )
                    .await
                }
                None => {
                    slot.commit_grizzly_transition_with_progression(
                        admission.root,
                        admission.character,
                        admission.holder,
                        fence,
                        pending.cause,
                        action.clone(),
                        std::sync::Arc::clone(catalogue),
                        &NoGrizzlyPremium,
                        progression.as_ref().map(|binding| &binding.policy),
                    )
                    .await
                }
            };
            match result {
                Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {}
                Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(_)) =>
                    return Some(NpcTalkOutcome { unknown: true, ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected) }),
                _ => {
                    drop(slot); drop(permit);
                    return Some(reject_batch(admission, command.game_session_id).await);
                }
            }
        }
        drop(slot);
        drop(permit);
    }
    admission
        .refresh_quest_session(command.game_session_id, false)
        .await;
    Some(reward_reply(admission, actor, command, &pending.intent, pending.reply).await)
}

#[path = "grizzly/recovery.rs"]
mod recovery;

#[cfg(test)]
#[path = "grizzly/tests.rs"]
mod tests;
