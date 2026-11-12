//! Shared dialogue executor for existing QuestState transitions only.
//! Selection does not grant inventory, appearance, currency, or reward authority.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use std::collections::BTreeMap;

pub(super) type Context = (u64, u64, String);

/// The donor NPC's one topic and optional held configuration identity. A named
/// Lua talkState uses `value`; numeric NpcHandler topics use `number`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Topic {
    pub number: u16,
    pub value: Option<String>,
}

impl From<u16> for Topic {
    fn from(number: u16) -> Self {
        Self {
            number,
            value: None,
        }
    }
}

pub(super) struct Choice {
    pub transition: Option<String>,
    pub next_topic: Option<Topic>,
    pub reply: Vec<String>,
}

#[derive(Default)]
pub(super) struct State {
    context: Option<Context>,
    topic: Topic,
    // A failed offer does not advance the topic, but still owns its confirmation.
    pub(super) reward_request: Option<Context>,
    pending: Option<Pending>,
    #[cfg(test)]
    interrupt_after_commit: Option<u64>,
}

#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn fixture_interrupt_quest_dialogue_commit(
        &self,
        session: GameSessionId,
        command: u64,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.lock().map_err(|_| "sessions")?;
        let state = &mut sessions.get_mut(&session).ok_or("session")?.quest_dialogue;
        if command == 0 || state.interrupt_after_commit.is_some() {
            return Err("fault already armed or invalid command".into());
        }
        state.interrupt_after_commit = Some(command);
        Ok(())
    }
}

impl State {
    pub(super) fn restart_topic(&mut self) {
        // An unresolved writer retains its confirmation until reconciliation.
        if self.pending.is_none() {
            self.context = None;
            self.topic = Topic::default();
            self.reward_request = None;
        }
    }

    pub(super) fn topic(&self, context: &Context) -> Topic {
        if self.context.as_ref() == Some(context) {
            self.topic.clone()
        } else {
            Topic::default()
        }
    }

    fn apply_topic(&mut self, context: Context, next: Option<Topic>) {
        let previous = self.topic(&context);
        self.context = Some(context);
        self.topic = next.unwrap_or(previous);
        if self.topic == Topic::default() {
            self.reward_request = None;
        }
    }
}

pub(super) struct Step {
    context: Context,
    topic: Topic,
    next_topic: Option<Topic>,
    transition: Option<String>,
    reply: Vec<String>,
}

#[derive(Clone)]
struct Pending {
    actor: ExactActorRef,
    character: CharacterId,
    request: QuestTransitionRequest,
    intent: NpcTalkIntent,
    context: Context,
    next_topic: Option<Topic>,
    reply: Vec<String>,
}

/// Bind a pure, source-backed selector to the actual current conversation.
/// The closure sees one common topic for that NPC/context and the admitted quest copy.
pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    select: impl FnOnce(&Context, &Topic, bool, &BTreeMap<String, i64>) -> Option<Choice>,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let (topic, fresh_conversation) = {
        let sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        let state = &sessions.get(&command.game_session_id)?.quest_dialogue;
        if state.pending.is_some() {
            return None;
        }
        (
            state.topic(&context),
            state.context.as_ref() != Some(&context),
        )
    };
    let sessions = admission.states.quest_sessions.lock().ok()?;
    let copy = sessions.get(&command.game_session_id)?.copy.as_ref()?;
    let tracks = copy.read_tracks(admission.quest_catalogue.as_deref()?)?;
    let choice = select(&context, &topic, fresh_conversation, &tracks)?;
    Some(Step {
        context,
        topic,
        next_topic: choice.next_topic,
        transition: choice.transition,
        reply: choice.reply,
    })
}

fn reply(
    chat: &ChatRuntime,
    session: GameSessionId,
    context: &Context,
    intent: &NpcTalkIntent,
    lines: Vec<String>,
) -> NpcTalkOutcome {
    use oteryn_protocol_oteryn::npc_service::{MAX_REPLY_LINE_BYTES, MAX_REPLY_LINES};
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let Some(book) = chat.npc_talk() else {
        return rejected();
    };
    if book.catalogue_revision() != context.0 || lines.len() > MAX_REPLY_LINES {
        return rejected();
    }
    let Some(entry) = book.get(&context.2) else {
        return rejected();
    };
    let Some(lines) = lines
        .into_iter()
        .map(|line| {
            let rendered = entry.render_source_text(&line, &chat.speaker_name(session)?);
            (!rendered.is_empty()
                && rendered.len() <= MAX_REPLY_LINE_BYTES
                && !rendered.chars().any(char::is_control))
            .then_some(line)
        })
        .collect::<Option<Vec<_>>>()
    else {
        return rejected();
    };
    // Validate the expanded wire bounds here, but retain source paragraphs.
    // Main's FreshAdmissionAuthority publication boundary renders exactly once,
    // including replay, so a token inside the customer name remains literal.
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: false,
        disposition: NpcIntentDisposition::Ok,
        conversation: Some(NpcConversation {
            open: Some((intent.npc_actor, NpcConversationState::Open)),
            lines,
        }),
        window: None,
    }
}

/// A receipt remains successful when its original conversation has ended. Only
/// the current conversation may receive the retained reply; reconciliation must
/// not reopen an older generation or publish into another NPC's conversation.
fn publish_committed_reply(
    current: Option<Context>,
    expected: &Context,
    rendered: NpcTalkOutcome,
) -> NpcTalkOutcome {
    if current.as_ref() == Some(expected) && rendered.disposition == NpcIntentDisposition::Ok {
        rendered
    } else {
        NpcTalkOutcome::only(NpcIntentDisposition::Ok)
    }
}

fn finish(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId, committed: bool) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
        && let Some(pending) = state.quest_dialogue.pending.take()
        && committed
    {
        state
            .quest_dialogue
            .apply_topic(pending.context, pending.next_topic);
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    // Resolve the admitted customer and existing wire bounds before a topic or
    // durable transition can change. A missing fact never publishes a token.
    let mut rendered = reply(
        &admission.states.chat,
        command.game_session_id,
        &step.context,
        &intent,
        step.reply.clone(),
    );
    if step
        .transition
        .as_ref()
        .is_some_and(|k| k.starts_with("oteryn:quest-reward/"))
    {
        rendered.inventory_changed = true;
    }
    if rendered.disposition != NpcIntentDisposition::Ok {
        return rendered;
    }
    let Some(transition) = step.transition else {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.context.clone())
        {
            return rejected();
        }
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = &mut sessions
            .entry(command.game_session_id)
            .or_default()
            .quest_dialogue;
        if state.pending.is_some() || state.topic(&step.context) != step.topic {
            return rejected();
        }
        state.apply_topic(step.context, step.next_topic);
        return rendered;
    };
    let (Some(item_fence), Ok(id), Some(catalogue)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission.quest_catalogue.as_ref(),
    ) else {
        return rejected();
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    let actor_conditions = {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || (transition.starts_with("oteryn:quest-reward/")
                && admission.active_generation.is_none_or(|generation| {
                    runtime.content_pin().server_artifact_digest()
                        != generation.identity().server_artifact_digest()
                }))
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.context.clone())
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
        admission
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|native| {
                let state = states.get(&runtime, actor, command.game_session_id)?;
                crate::durability::reward_claim_mint::QuestActorConditionProof::read_current(
                    &runtime,
                    actor,
                    fence,
                    CommandRef::new(command.game_session_id, id),
                    &transition,
                    native,
                    state.owned_conditions(),
                    admission.owner_now().get(),
                )
            })
    };
    let request = QuestTransitionRequest {
        transition_key: transition,
        cause: QuestCause::Command(CommandRef::new(command.game_session_id, id)),
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = &mut sessions
            .entry(command.game_session_id)
            .or_default()
            .quest_dialogue;
        if state.pending.is_some() {
            return unknown();
        }
        if state.topic(&step.context) != step.topic {
            return rejected();
        }
        state.pending = Some(Pending {
            actor,
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            context: step.context.clone(),
            next_topic: step.next_topic,
            reply: step.reply.clone(),
        });
    }
    let value_callback = oteryn_gameplay_quests::quest::reward_callbacks::load_for(catalogue)
        .ok()
        .is_some_and(|callbacks| callbacks.iter().any(|c| c.key == request.transition_key));
    let fee_payment = if oteryn_gameplay_quests::quest::reward_callbacks::load_for(catalogue)
        .ok()
        .and_then(|callbacks| {
            callbacks
                .into_iter()
                .find(|c| c.key == request.transition_key)
        })
        .is_some_and(|c| c.fee.is_some())
    {
        match super::endgame::prepare_reward_fee(admission, command, fence).await {
            Ok(payment) => Some(payment),
            Err(outcome) => {
                finish(admission, command.game_session_id, false);
                return *outcome;
            }
        }
    } else {
        None
    };
    let outcome = if value_callback {
        let Some(native) = admission
            .active_generation
            .and_then(|g| g.native_gameplay())
        else {
            finish(admission, command.game_session_id, false);
            return rejected();
        };
        let weights = super::super::item_ref_admission::NativeItemWeights(native)
            .with_wheel(admission.premium_coordinator);
        slot.commit_quest_reward(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            std::sync::Arc::clone(catalogue),
            crate::durability::reward_claim_mint::QuestRewardAdmission {
                native,
                weights: &weights,
                fee_payment: fee_payment.as_ref(),
                actor_conditions: actor_conditions.as_ref(),
                carried_use: None,
            },
            &admission.premium_coordinator,
        )
        .await
    } else {
        slot.commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            std::sync::Arc::clone(catalogue),
        )
        .await
    };
    drop(slot);
    #[cfg(test)]
    if value_callback && let Err(error) = &outcome {
        eprintln!("quest reward commit: {error:?}");
    }
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            #[cfg(test)]
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
                && state.quest_dialogue.interrupt_after_commit == Some(command.command_id)
            {
                state.quest_dialogue.interrupt_after_commit = None;
                return unknown();
            }
            finish(admission, command.game_session_id, true);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            // QUEST-XP-1 refresh submits any committed XP obligation and marks
            // LEVELUP-LIVE-1. Install only the live owner's durable-fact projection.
            admission
                .refresh_level_up_vitals(actor, command.game_session_id)
                .await;
            let runtime = admission.runtime.lock().await;
            publish_committed_reply(
                admission.states.npc_conversations.current_npc(
                    &admission.states.chat,
                    &runtime,
                    actor,
                    command.game_session_id,
                    &intent,
                ),
                &step.context,
                rendered,
            )
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            finish(admission, command.game_session_id, false);
            rejected()
        }
    }
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .quest_dialogue
        .pending
        .clone()?;
    let Ok(id) = CommandId::new(command.command_id) else {
        return None;
    };
    if pending.request.cause != QuestCause::Command(CommandRef::new(command.game_session_id, id)) {
        return None;
    }
    let value_callback = pending
        .request
        .transition_key
        .starts_with("oteryn:quest-reward/");
    match admission
        .root
        .reconcile_character_quest_transition(
            admission.character,
            pending.character,
            pending.request,
        )
        .await
    {
        Ok(receipt) => {
            finish(admission, command.game_session_id, receipt.is_some());
            if receipt.is_some() {
                admission
                    .refresh_quest_session(command.game_session_id, false)
                    .await;
                admission
                    .refresh_level_up_vitals(pending.actor, command.game_session_id)
                    .await;
                let mut rendered = reply(
                    &admission.states.chat,
                    command.game_session_id,
                    &pending.context,
                    &pending.intent,
                    pending.reply,
                );
                rendered.inventory_changed = value_callback;
                let runtime = admission.runtime.lock().await;
                Some(publish_committed_reply(
                    admission.states.npc_conversations.current_npc(
                        &admission.states.chat,
                        &runtime,
                        pending.actor,
                        command.game_session_id,
                        &pending.intent,
                    ),
                    &pending.context,
                    rendered,
                ))
            } else {
                Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
            }
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}

#[cfg(test)]
mod publication_tests;

#[cfg(test)]
mod tests {
    use super::{State, Topic};

    #[test]
    fn topic_is_bound_to_npc_and_conversation_and_catalogue_generation() {
        let context = (7, 8, "oteryn:npc.ezebeth".into());
        let mut state = State::default();
        state.apply_topic(context.clone(), Some(2.into()));
        assert_eq!(state.topic(&context), Topic::from(2));
        assert_eq!(
            state.topic(&(8, 8, "oteryn:npc.ezebeth".into())),
            Topic::default()
        );
        assert_eq!(
            state.topic(&(7, 9, "oteryn:npc.ezebeth".into())),
            Topic::default()
        );
        assert_eq!(
            state.topic(&(7, 8, "oteryn:npc.a_beggar".into())),
            Topic::default()
        );
    }

    #[test]
    fn no_set_topic_keeps_only_the_same_conversation_topic() {
        let old = (7, 8, "oteryn:npc.a_beggar".into());
        let next = (7, 9, "oteryn:npc.a_beggar".into());
        let mut state = State::default();
        state.apply_topic(old.clone(), Some(1.into()));
        state.apply_topic(old.clone(), None);
        assert_eq!(state.topic(&old), Topic::from(1));
        state.apply_topic(next.clone(), None);
        assert_eq!(state.topic(&next), Topic::default());
    }

    #[test]
    fn numeric_topic_and_held_donor_table_are_one_context_bound_state() {
        let old = (7, 8, "oteryn:npc.bozo".into());
        let mut state = State::default();
        let held = Topic {
            number: 3,
            value: Some("stage:31".into()),
        };
        state.apply_topic(old.clone(), Some(held.clone()));
        state.apply_topic(old.clone(), None);
        assert_eq!(state.topic(&old), held);
        state.apply_topic(old.clone(), Some(0.into()));
        assert_eq!(state.topic(&old), Topic::default());
        state.apply_topic(
            old.clone(),
            Some(Topic {
                number: 0,
                value: Some("report".into()),
            }),
        );
        assert_eq!(
            state.topic(&(7, 9, "oteryn:npc.bozo".into())),
            Topic::default()
        );
    }

    fn source_reply_fixture() -> Result<
        (
            super::ChatRuntime,
            super::GameSessionId,
            super::Context,
            super::NpcTalkIntent,
        ),
        String,
    > {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?;
        let book = crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model);
        let context: super::Context = (book.catalogue_revision(), 1, "oteryn:npc.ezebeth".into());
        assert!(book.get(&context.2).is_some());
        let chat = super::ChatRuntime::default();
        chat.install_npc_talk(std::sync::Arc::new(book));
        let session = super::GameSessionId::decode(&[
            0x01, 0x90, 0x00, 0x00, 0x00, 1, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, 1,
        ])
        .map_err(|error| format!("{error:?}"))?;
        chat.admit(
            session,
            "[|PLAYERNAME|] ${name} %s",
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1),
        );
        let intent = super::NpcTalkIntent {
            npc_actor: oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: [42; 16],
                generation: 1,
            },
            text: "mission".into(),
        };
        Ok((chat, session, context, intent))
    }

    #[test]
    fn native_source_reply_validates_the_customer_and_retains_source_paragraphs()
    -> Result<(), String> {
        let (chat, session, context, intent) = source_reply_fixture()?;
        let source = vec![
            "Hello, |PLAYERNAME|.".into(),
            "The next source paragraph remains whole.".into(),
        ];
        let outcome = super::reply(&chat, session, &context, &intent, source.clone());
        let conversation = outcome.conversation.ok_or("conversation")?;
        assert_eq!(
            conversation.lines,
            [
                "Hello, |PLAYERNAME|.",
                "The next source paragraph remains whole.",
            ]
        );
        let bytes = oteryn_protocol_oteryn::npc_service::encode_npc_conversation(&conversation)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_conversation(&bytes)
                .map_err(|error| format!("{error:?}"))?,
            conversation
        );
        chat.forget(session);
        assert_eq!(
            super::reply(&chat, session, &context, &intent, source).disposition,
            super::NpcIntentDisposition::Rejected
        );
        Ok(())
    }

    #[test]
    fn native_source_reply_refuses_a_foreign_catalogue_or_over_bound_render() -> Result<(), String>
    {
        let (chat, session, mut context, intent) = source_reply_fixture()?;
        let source =
            vec!["|PLAYERNAME|".repeat(oteryn_protocol_oteryn::npc_service::MAX_REPLY_LINE_BYTES)];
        assert_eq!(
            super::reply(&chat, session, &context, &intent, source).disposition,
            super::NpcIntentDisposition::Rejected
        );
        context.0 = context.0.wrapping_add(1);
        assert_eq!(
            super::reply(&chat, session, &context, &intent, vec!["Hello.".into()]).disposition,
            super::NpcIntentDisposition::Rejected
        );
        Ok(())
    }
}
