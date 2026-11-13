//! Ordered source NPC storage branches on the established talk and receipt owners.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::domain::CharacterId;
use crate::durability::quest_state::{
    QuestTransitionOutcome,
    npc_program::{self, NpcProgramRequest},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleCatalog, NpcRuleEffect, NpcRuleTrigger};
use std::sync::Arc;

pub(in crate::gameplay_transport) mod teleport;

type Context = (u64, u64, String);
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Conversation {
    context: Context,
    actor: ExactActorRef,
    npc: EntityRef,
    npc_actor: ExactActorRef,
    topic: i32,
}
#[derive(Clone)]
pub(super) enum Pending {
    Topic(Conversation),
    Commit {
        character: CharacterId,
        request: NpcProgramRequest,
        intent: NpcTalkIntent,
        next: Conversation,
        fallback_lines: Vec<String>,
        teleport: bool,
        supersedes_topic: bool,
    },
}
pub(super) struct ReplyStep {
    lines: Vec<String>,
    end: bool,
    end_delay_ms: u64,
    previous: Option<Conversation>,
    next: Conversation,
    supersedes_topic: bool,
}
pub(super) enum Step {
    Reply(Box<ReplyStep>),
    Refused,
    Held,
    Commit(Box<CommitStep>),
}
pub(super) struct CommitStep {
    context: Context,
    previous: Option<Conversation>,
    next: Conversation,
    request: NpcProgramRequest,
    fallback_lines: Vec<String>,
    teleport: Option<oteryn_gameplay_quests::quest::npc_rules::NpcRulePosition>,
    magic: usize,
    supersedes_topic: bool,
}

pub(super) async fn inventory(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<npc_program::NpcProgramInventory> {
    let npc = {
        let runtime = admission.runtime.lock().await;
        let npc =
            service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)?;
        let source = NpcRuleCatalog::runtime()
            .ok()?
            .admitted_for_npc(&npc.key, admission.quest_catalogue.as_ref()?)
            .ok()??;
        if !npc_program::inventory_prefetch_required(&npc.key, &source) {
            return None;
        }
        npc.key.clone()
    };
    let expected = command.item_fence?;
    let fence = admission
        .current_quest_fence(command.game_session_id)
        .await
        .ok()??;
    if fence.item_fence() != expected {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_ref()?;
    let native = admission.active_generation?.native_gameplay()?;
    admission
        .root
        .read_current_npc_program_inventory(
            admission.character,
            admission.holder,
            fence,
            CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).ok()?,
            ),
            &npc,
            catalogue,
            &super::super::item_ref_admission::NativeItemWeights(native)
                .with_wheel(admission.premium_coordinator),
        )
        .await
        .ok()
}

pub(super) async fn appearance(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<(std::collections::BTreeMap<u16, u8>, u8)> {
    use crate::domain::appearance::{AppearanceContent, Sex};
    let needs = {
        let runtime = admission.runtime.lock().await;
        // A greeting establishes engagement later in talk_in. This prefetch is
        // read-only; prepare rechecks the engaged NPC before using the snapshot.
        let npc =
            service_context::live_npc(&runtime, actor, command.game_session_id, intent.npc_actor)?;
        NpcRuleCatalog::runtime()
            .ok()?
            .programs()
            .iter()
            .flat_map(|program| &program.document.rules)
            .filter(|rule| rule.npc == npc.key)
            .any(|rule| {
                rule.guards.iter().any(|guard| {
                    matches!(
                        guard,
                        oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Outfit { .. }
                            | oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Sex { .. }
                    )
                }) || rule.effects.iter().any(|effect| {
                    matches!(
                        effect,
                        NpcRuleEffect::Outfit { .. } | NpcRuleEffect::OutfitAddon { .. }
                    )
                })
            })
    };
    if !needs {
        return Some((Default::default(), 0));
    }
    let (_, content) = admission.appearance_context()?;
    let fence =
        super::super::character_gameplay_fence::current(admission, command.game_session_id, false)
            .await?;
    let state = admission
        .root
        .read_current_character_appearance_state(
            admission.character,
            admission.holder,
            fence,
            content.earned_grant_keys(),
        )
        .await
        .ok()?;
    Some((
        crate::durability::character_appearance::npc_outfits::snapshot(&content, &state.earned),
        match state.sex {
            Sex::Female => 0,
            Sex::Male => 1,
        },
    ))
}

#[allow(
    clippy::too_many_arguments,
    reason = "NPC source preview includes a separately fenced appearance snapshot"
)]
pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    ordinary: &NpcTalkOutcome,
    stag_beaks: Option<&crate::durability::stag_beaks::StagBeakPreview>,
    appearance: Option<&(std::collections::BTreeMap<u16, u8>, u8)>,
    inventory: Option<&npc_program::NpcProgramInventory>,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let catalogue = admission.quest_catalogue.as_ref()?;
    let source = NpcRuleCatalog::runtime()
        .ok()?
        .admitted_for_npc(&context.2, catalogue)
        .ok()??;
    if !source
        .programs()
        .iter()
        .any(|p| p.document.rules.iter().any(|r| r.npc == context.2))
    {
        return None;
    }
    let mut tracks = {
        let sessions = admission.states.quest_sessions.lock().ok()?;
        let copy = sessions.get(&command.game_session_id)?.copy.as_ref()?;
        let mut tracks = copy.read_tracks(catalogue)?;
        for (quest, state) in copy.states() {
            if let Some(defaults) = oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
                catalogue,
                quest,
                &state.definition_hash,
            ) {
                for (key, value) in defaults {
                    tracks.entry(key.clone()).or_insert(*value);
                }
            }
        }
        tracks
    };
    let player_name = admission.states.chat.speaker_name(command.game_session_id);
    if context.2 == crate::durability::stag_beaks::HARTFORD {
        tracks.insert(crate::durability::stag_beaks::BEAKS.into(), 0);
        if let Some(proof) = stag_beaks {
            proof
                .overlay(command.item_fence?.character_id, &mut tracks)
                .ok()?;
        }
    }
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if state.quest_dialogue.has_pending_value()
        || matches!(state.catalogue, Some(Pending::Commit { .. }))
    {
        return Some(Step::Held);
    }
    let previous = match &state.catalogue {
        Some(Pending::Topic(p))
            if p.context == context && p.actor == actor && p.npc == intent.npc_actor =>
        {
            Some(p.clone())
        }
        _ => None,
    };
    if previous.is_none() {
        state.catalogue = None;
    }
    let greeting = intent
        .text
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| {
            ["hi", "hello", "hail"]
                .iter()
                .any(|g| word.eq_ignore_ascii_case(g))
        })
        && !source
            .matches_player_name_message(
                &context.2,
                &intent.text,
                previous.as_ref().map_or(0, |p| p.topic),
                player_name.as_deref(),
            )
            .ok()?;
    let mut request = NpcProgramRequest {
        command: CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        ),
        npc: context.2.clone(),
        trigger: if greeting {
            NpcRuleTrigger::Greet
        } else {
            NpcRuleTrigger::Say
        },
        message: intent.text.clone(),
        topic: if greeting {
            0
        } else {
            previous.as_ref().map_or(0, |p| p.topic)
        },
        transition_key: String::new(),
    };
    // StorageTime is checked again at database transaction time before committing.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())?;
    let eligibility = if source
        .programs()
        .iter()
        .flat_map(|program| &program.document.rules)
        .filter(|rule| rule.npc == request.npc)
        .flat_map(|rule| &rule.guards)
        .any(|guard| {
            matches!(
                guard,
                oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Level { .. }
                    | oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Vocation { .. }
                    | oteryn_gameplay_quests::quest::npc_rules::NpcRuleGuard::Premium { .. }
            )
        }) {
        let facts = admission
            .states
            .spell_states
            .try_lock()
            .ok()?
            .get(runtime, actor, command.game_session_id)?
            .character_facts();
        let vocation = match facts.vocation {
            crate::spell::Vocation::None => "none",
            crate::spell::Vocation::Knight | crate::spell::Vocation::EliteKnight => "knight",
            crate::spell::Vocation::Paladin | crate::spell::Vocation::RoyalPaladin => "paladin",
            crate::spell::Vocation::Sorcerer | crate::spell::Vocation::MasterSorcerer => "sorcerer",
            crate::spell::Vocation::Druid | crate::spell::Vocation::ElderDruid => "druid",
            crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk => "monk",
        };
        npc_program::NpcEligibility {
            level: facts.level,
            vocation: format!("oteryn:vocation.{vocation}"),
            premium_current: admission
                .session_account(command.game_session_id)
                .zip(admission.premium_coordinator)
                .is_some_and(|(account, owner)| {
                    owner.view(account).premium_gate().premium_current()
                }),
        }
    } else {
        npc_program::NpcEligibility::default()
    };
    let Some(appearance) = appearance else {
        return Some(Step::Refused);
    };
    let needs_inventory = npc_program::requires_current_inventory(&request, &source);
    let inventory = if needs_inventory {
        let Some(snapshot) =
            inventory.and_then(|snapshot| snapshot.for_fence(&request.npc, command.item_fence?))
        else {
            // A failed or stale read cannot be replaced by optimistic counts,
            // nor may it fall through to a speculative reply/topic.
            return Some(Step::Refused);
        };
        Some(snapshot)
    } else {
        None
    };
    let teleport =
        npc_program::teleport::preview_teleport(&request, &tracks, catalogue, now, &eligibility)
            .ok()
            .flatten();
    if teleport.as_ref().is_some_and(|p| {
        !matches!(
            p.source.quest.as_str(),
            "oteryn:quest.the_paradox_tower_quest" | "oteryn:quest.in_service_of_yalahar_quest"
        )
    }) {
        return Some(Step::Refused);
    }
    let plan = match teleport
        .as_ref()
        .map(|p| Ok(Some(p.source.clone())))
        .unwrap_or_else(|| {
            if let Some(inventory) = inventory {
                npc_program::preview_with_inventory(
                    &request,
                    &tracks,
                    catalogue,
                    now,
                    &eligibility,
                    player_name.as_deref(),
                    (&appearance.0, appearance.1),
                    inventory,
                )
            } else {
                npc_program::preview_with_appearance(
                    &request,
                    &tracks,
                    catalogue,
                    now,
                    &eligibility,
                    player_name.as_deref(),
                    (&appearance.0, appearance.1),
                )
            }
        }) {
        Ok(Some(plan)) => plan,
        Ok(None) => {
            if greeting {
                state.catalogue = None;
            }
            return None;
        }
        Err(_) => return Some(Step::Refused),
    };
    let next = Conversation {
        context: context.clone(),
        actor,
        npc: intent.npc_actor,
        npc_actor: admission.states.chat.engaged(command.game_session_id)?,
        topic: plan.next_topic,
    };
    let has_track_mutation = super::catalogue_track_mutation::has_mutation(&plan);
    if !has_track_mutation
        && !plan.completes
        && plan.effects.iter().any(|effect| {
            matches!(
                effect,
                NpcRuleEffect::Experience { .. }
                    | NpcRuleEffect::Outfit { .. }
                    | NpcRuleEffect::OutfitAddon { .. }
            )
        })
    {
        return Some(Step::Refused);
    }
    let supersedes_topic = plan.effects.iter().any(|effect| {
        matches!(
            effect,
            NpcRuleEffect::SetTopic { .. } | NpcRuleEffect::EndConversation { .. }
        )
    });
    if teleport.is_none()
        && !has_track_mutation
        && !plan.completes
        && request.npc != "oteryn:npc.pemaret"
        && !needs_inventory
    {
        let end = plan
            .effects
            .iter()
            .any(|effect| matches!(effect, NpcRuleEffect::EndConversation { .. }));
        let end_delay_ms = plan
            .effects
            .iter()
            .filter_map(|effect| match effect {
                NpcRuleEffect::EndConversation { delay_ms } => Some(*delay_ms),
                _ => None,
            })
            .min()
            .unwrap_or(0);
        if plan.reply.is_empty() && !end && !supersedes_topic {
            // An unhandled silent plan keeps the existing fallback; selected topic
            // changes still publish through the owner-transfer critical section.
            state.catalogue = Some(Pending::Topic(next));
            return None;
        }
        return Some(Step::Reply(Box::new(ReplyStep {
            lines: plan.reply,
            end,
            end_delay_ms,
            previous,
            next,
            supersedes_topic,
        })));
    }
    request.transition_key = plan.selected_rule_keys.first()?.clone();
    let fallback_lines = if greeting && plan.reply.is_empty() {
        ordinary
            .conversation
            .as_ref()
            .map(|conversation| conversation.lines.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    Some(Step::Commit(Box::new(CommitStep {
        context,
        previous,
        next,
        request,
        fallback_lines,
        supersedes_topic,
        magic: teleport.as_ref().map_or(0, |p| p.magic.len()),
        teleport: teleport.map(|p| p.destination),
    })))
}

fn response(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    mut lines: Vec<String>,
) -> NpcTalkOutcome {
    if let Some(name) = admission.states.chat.speaker_name(session) {
        for line in &mut lines {
            *line = oteryn_gameplay_npc::quest_greetings::render_player_name(line, &name);
        }
    }
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
fn end_response(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    npc_actor: ExactActorRef,
    delay_ms: u64,
    response: &mut NpcTalkOutcome,
) {
    if delay_ms != 0 {
        let Some(generation) = admission.states.chat.npc_conversation_generation(session) else {
            return;
        };
        if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
            && let Some(state) = sessions.get_mut(&session)
        {
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(delay_ms);
            if state.delayed_end.as_ref().is_none_or(|end| {
                end.generation != generation
                    || end.npc_actor != npc_actor
                    || deadline < end.deadline
            }) {
                state.delayed_end = Some(DelayedEnd {
                    npc_actor,
                    generation,
                    deadline,
                });
            }
        }
        return;
    }
    admission.states.chat.end_engagement(session, npc_actor);
    if let Some(conversation) = &mut response.conversation {
        conversation.open = None;
    }
}

fn install(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    next: Option<Conversation>,
) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.catalogue = next.map(Pending::Topic);
    }
}

fn install_success(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    context: &Context,
    next: Option<Conversation>,
    supersedes_topic: bool,
) -> bool {
    let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
        return false;
    };
    let Some(state) = sessions.get_mut(&command.game_session_id) else {
        return false;
    };
    let matches = matches!(&state.catalogue, Some(Pending::Commit { request, next, .. })
        if request.command.game_session_id() == command.game_session_id
            && request.command.command_id().get() == command.command_id
            && &next.context == context);
    if !matches || state.quest_dialogue.has_pending_value() {
        return false;
    }
    if supersedes_topic && !state.quest_dialogue.retire_for_source(context) {
        return false;
    }
    state.catalogue = next.map(Pending::Topic);
    true
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
    let CommitStep {
        context,
        previous,
        next,
        request,
        fallback_lines,
        teleport,
        magic,
        supersedes_topic,
    } = match step {
        Step::Reply(step) => {
            let ReplyStep {
                lines,
                end,
                end_delay_ms,
                previous,
                next,
                supersedes_topic,
            } = *step;
            {
                let runtime = admission.runtime.lock().await;
                if next.actor != actor
                    || next.npc != intent.npc_actor
                    || admission.states.npc_conversations.current_npc(
                        &admission.states.chat,
                        &runtime,
                        actor,
                        command.game_session_id,
                        &intent,
                    ) != Some(next.context.clone())
                {
                    return rejected();
                }
                let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
                    return rejected();
                };
                let Some(state) = sessions.get_mut(&command.game_session_id) else {
                    return rejected();
                };
                if state.quest_dialogue.has_pending_value()
                    || matches!(state.catalogue, Some(Pending::Commit { .. }))
                {
                    return unknown();
                }
                let current = match &state.catalogue {
                    Some(Pending::Topic(p)) => Some(p),
                    _ => None,
                };
                if current != previous.as_ref() {
                    return rejected();
                }
                if supersedes_topic && !state.quest_dialogue.retire_for_source(&next.context) {
                    return unknown();
                }
                // Retire and publish under the same current-NPC/session critical section.
                state.catalogue = (!end).then_some(Pending::Topic(next.clone()));
            }
            let mut response = response(admission, command.game_session_id, &intent, lines);
            if end {
                end_response(
                    admission,
                    command.game_session_id,
                    next.npc_actor,
                    end_delay_ms,
                    &mut response,
                );
            }
            return response;
        }
        Step::Refused => return rejected(),
        Step::Held => return unknown(),
        Step::Commit(c) => *c,
    };
    let (Some(item_fence), Some(catalogue)) =
        (command.item_fence, admission.quest_catalogue.as_ref())
    else {
        return rejected();
    };
    // Acquire the physical item lane before the revision cursor and SQL transaction.
    let Some(item_permit) = admission.spell_lane_permit().await else {
        return unknown();
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
    {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(context)
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
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let Some(state) = sessions.get_mut(&command.game_session_id) else {
            return rejected();
        };
        if state.quest_dialogue.has_pending_value()
            || matches!(state.catalogue, Some(Pending::Commit { .. }))
        {
            return unknown();
        }
        let current = match &state.catalogue {
            Some(Pending::Topic(p)) => Some(p),
            _ => None,
        };
        if current != previous.as_ref() {
            return rejected();
        }
        state.catalogue = Some(Pending::Commit {
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            next: next.clone(),
            fallback_lines: fallback_lines.clone(),
            teleport: teleport.is_some(),
            supersedes_topic,
        });
    }
    if let Some(destination) = teleport {
        let started = {
            let mut runtime = admission.runtime.lock().await;
            teleport::start(
                admission,
                actor,
                command,
                teleport::DepartureProof {
                    request,
                    catalogue: Arc::clone(catalogue),
                    intent,
                    context: next.context.clone(),
                    magic,
                },
                destination,
                &mut runtime,
            )
        };
        drop(slot);
        drop(item_permit);
        if started.is_none() {
            install(admission, command.game_session_id, None);
            return rejected();
        }
        return resume(admission, command).await.unwrap_or_else(unknown);
    }
    let moves_items = (request.npc == "oteryn:npc.pemaret"
        && request.transition_key.ends_with("/deliver"))
        || request.npc == "oteryn:npc.tereban"
            && (request.transition_key.ends_with("-deliver")
                || request.transition_key.ends_with("/mission-complete"));
    let outcome = slot
        .commit_npc_program(
            admission.root,
            admission.character,
            admission.holder,
            &item_permit,
            fence,
            request,
            Arc::clone(catalogue),
            &super::super::item_ref_admission::NativeItemWeights(
                match admission
                    .active_generation
                    .and_then(|generation| generation.native_gameplay())
                {
                    Some(native) => native,
                    None => {
                        install(admission, command.game_session_id, None);
                        return rejected();
                    }
                },
            )
            .with_wheel(admission.premium_coordinator),
            &admission.premium_coordinator,
        )
        .await;
    drop(slot);
    drop(item_permit);
    match outcome {
        Ok(commit)
            if commit.read_only
                || matches!(
                    commit.transition,
                    QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_)
                ) =>
        {
            if !commit.magic_effects.is_empty() {
                let runtime = admission.runtime.lock().await;
                if runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok()
                    && let Ok(at) = runtime.read_actor_position(actor)
                    && let Some(map) = admission.bundle_map()
                {
                    let at = at.position();
                    if let (Ok(x), Ok(y), Some(floor)) = (
                        u16::try_from(at.x),
                        u16::try_from(at.y),
                        i8::try_from(at.floor).ok().and_then(i8::checked_neg),
                    ) {
                        for effect in &commit.magic_effects {
                            map.facts.emit_movement_effect(
                                crate::map::overlay::TilePos { x, y, floor },
                                *effect,
                            );
                        }
                    }
                }
            }
            if !install_success(
                admission,
                command,
                &next.context,
                (!commit.end_conversation).then_some(Conversation {
                    topic: commit.next_topic,
                    ..next.clone()
                }),
                supersedes_topic,
            ) {
                return unknown();
            }
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            let mut response = response(
                admission,
                command.game_session_id,
                &intent,
                if commit.reply.is_empty() {
                    fallback_lines
                } else {
                    commit.reply
                },
            );
            response.inventory_changed =
                !commit.read_only && (commit.inventory_changed || moves_items);
            if commit.end_conversation {
                end_response(
                    admission,
                    command.game_session_id,
                    next.npc_actor,
                    commit.end_delay_ms,
                    &mut response,
                );
            }
            response
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            install(admission, command.game_session_id, None);
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
        .catalogue
        .clone()?;
    let Pending::Commit {
        character,
        request,
        intent,
        next,
        fallback_lines,
        teleport,
        supersedes_topic,
    } = pending
    else {
        return None;
    };
    if request.command
        != CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        )
    {
        return None;
    }
    let quest_hold = admission
        .states
        .newhaven_travel
        .holds
        .lock()
        .ok()?
        .get(&command.game_session_id)
        .is_some_and(|h| matches!(h.route, super::super::newhaven_travel::Departure::Quest(_)));
    if quest_hold {
        match admission
            .reconcile_newhaven_travel(next.actor, command.game_session_id)
            .await
        {
            None => {
                return Some(NpcTalkOutcome {
                    unknown: true,
                    ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                });
            }
            Some(false) => admission
                .states
                .newhaven_travel
                .forget(command.game_session_id),
            Some(true) => {}
        }
    }
    let moves_items = (request.npc == "oteryn:npc.pemaret"
        && request.transition_key.ends_with("/deliver"))
        || request.npc == "oteryn:npc.tereban"
            && (request.transition_key.ends_with("-deliver")
                || request.transition_key.ends_with("/mission-complete"));
    Some(
        match admission
            .root
            .reconcile_npc_program(admission.character, character, request)
            .await
        {
            Ok(Some(commit)) => {
                let engaged = !teleport
                    || admission.states.chat.engaged(command.game_session_id)
                        == Some(next.npc_actor);
                if !install_success(
                    admission,
                    command,
                    &next.context,
                    (!commit.end_conversation && engaged).then_some(Conversation {
                        topic: commit.next_topic,
                        ..next.clone()
                    }),
                    supersedes_topic,
                ) {
                    return Some(NpcTalkOutcome {
                        unknown: true,
                        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
                    });
                }
                admission
                    .refresh_quest_session(command.game_session_id, false)
                    .await;
                let mut response = response(
                    admission,
                    command.game_session_id,
                    &intent,
                    if commit.reply.is_empty() {
                        fallback_lines
                    } else {
                        commit.reply
                    },
                );
                response.inventory_changed = commit.inventory_changed || moves_items;
                if commit.end_conversation || !engaged {
                    end_response(
                        admission,
                        command.game_session_id,
                        next.npc_actor,
                        commit.end_delay_ms,
                        &mut response,
                    );
                }
                response
            }
            Ok(None) => {
                install(admission, command.game_session_id, None);
                NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            }
            Err(_) => NpcTalkOutcome {
                unknown: true,
                ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            },
        },
    )
}

pub(super) struct DelayedEnd {
    npc_actor: ExactActorRef,
    generation: u64,
    deadline: std::time::Instant,
}

pub(super) fn follow_end(
    conversations: &NpcConversations,
    chat: &ChatRuntime,
    session: GameSessionId,
) {
    let generation = chat.npc_conversation_generation(session);
    let engaged = chat.engaged(session);
    let due = {
        let Ok(mut sessions) = conversations.sessions.lock() else {
            return;
        };
        let Some(state) = sessions.get_mut(&session) else {
            return;
        };
        let Some(end) = &state.delayed_end else {
            return;
        };
        if generation != Some(end.generation) || engaged != Some(end.npc_actor) {
            state.delayed_end = None;
            return;
        }
        if std::time::Instant::now() < end.deadline {
            return;
        }
        state.delayed_end.take()
    };
    if let Some(end) = due
        && chat.npc_conversation_generation(session) == Some(end.generation)
    {
        chat.end_engagement(session, end.npc_actor);
    }
}

// Test-only observation of the topic produced by the normal command path.
#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn catalogue_topic_for_test(
        &self,
        session: GameSessionId,
    ) -> Option<i32> {
        match &self.sessions.lock().ok()?.get(&session)?.catalogue {
            Some(Pending::Topic(conversation)) => Some(conversation.topic),
            _ => None,
        }
    }
}
