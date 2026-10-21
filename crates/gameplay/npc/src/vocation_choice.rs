//! Dawnport vocation-choice dialogue; durable requests and secure occurrences stay in dispatch.

use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::npc_command::NpcTalkOutcome;
use oteryn_protocol_oteryn::npc_service::{
    NpcConversation, NpcConversationState, NpcIntentDisposition, TALK_RANGE_TILES,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use crate::content::npc_catalogue::BASE_VOCATIONS;

/// The dispatch chat owner supplies its actual session lock, engagement and capacity checks.
pub trait NpcDialogueOwner {
    fn converse<R>(
        &self,
        session: GameSessionId,
        f: impl FnOnce(&mut Option<ExactActorRef>) -> R,
    ) -> Option<R>;
    fn engaged(&self, session: GameSessionId) -> Option<ExactActorRef>;
    fn end_engagement(&self, session: GameSessionId, npc: ExactActorRef);
}

#[derive(Debug, Clone)]
pub struct ConfirmedNpcVocation<O, R> {
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub actor: ExactActorRef,
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub npc: EntityRef,
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub selected: &'static str,
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC retained-intent consumer reads this field; D1173 integration is pending."
    )]
    pub occurrence: O,
    pub retained: Option<R>,
}

/// A topic inside the existing per-session NPC owner, not another build/training store.
#[derive(Debug, Clone)]
pub struct NpcVocationConversation<O, R> {
    actor: ExactActorRef,
    npc: EntityRef,
    selected: Option<&'static str>,
    confirmed: Option<ConfirmedNpcVocation<O, R>>,
}

impl<O: Copy + PartialEq, R: Clone + PartialEq> NpcVocationConversation<O, R> {
    pub fn confirmed(&self) -> Option<ConfirmedNpcVocation<O, R>> {
        self.confirmed.clone()
    }
    #[allow(
        dead_code,
        reason = "D1220: A2 NPC rollback path retains this occurrence-specific operation; D1173 integration is pending."
    )]
    pub fn rollback_request(&mut self, occurrence: O) {
        if let Some(choice) = self
            .confirmed
            .as_mut()
            .filter(|choice| choice.occurrence == occurrence)
        {
            choice.retained = None;
        }
    }
    #[allow(
        dead_code,
        reason = "D1220: A2 native NPC caller retains the exact build intent through this operation; D1173 integration is pending."
    )]
    pub fn retain(&mut self, occurrence: O, request: R) -> bool {
        let Some(choice) = self
            .confirmed
            .as_mut()
            .filter(|choice| choice.occurrence == occurrence)
        else {
            return false;
        };
        if let Some(old) = &choice.retained {
            return old == &request;
        }
        choice.retained = Some(request);
        true
    }
}

pub fn answer(npc: EntityRef, disposition: NpcIntentDisposition, line: &str) -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: false,
        inventory_changed: false,
        disposition,
        conversation: Some(NpcConversation {
            open: Some((npc, NpcConversationState::Open)),
            lines: vec![line.to_owned()],
        }),
        window: None,
    }
}

/// Called only after the existing NPC owner applies rate, exact NPC-generation and range checks.
/// The normal conversation owner supplies capacity and engagement; an unquoted `yes` commits nothing.
#[allow(
    clippy::too_many_arguments,
    reason = "native NPC owner context after its rate/generation/range admission"
)]
pub fn dawnport_choice_dialogue<O: Copy + PartialEq, R: Clone + PartialEq>(
    topic: &mut Option<NpcVocationConversation<O, R>>,
    chat: &impl NpcDialogueOwner,
    actor: ExactActorRef,
    session: GameSessionId,
    npc: EntityRef,
    npc_actor: ExactActorRef,
    text: &str,
    new_occurrence: impl FnOnce() -> Option<O>,
) -> NpcTalkOutcome {
    use NpcIntentDisposition as D;
    let text = text.trim().to_ascii_lowercase();
    if topic
        .as_ref()
        .is_some_and(|value| value.actor != actor || value.npc != npc)
    {
        if topic
            .as_ref()
            .and_then(NpcVocationConversation::confirmed)
            .is_some_and(|value| value.retained.is_some())
        {
            return answer(
                npc,
                D::Rejected,
                "The previous choice is still being checked.",
            );
        }
        *topic = None;
    }
    if topic
        .as_ref()
        .and_then(NpcVocationConversation::confirmed)
        .is_some()
    {
        return answer(npc, D::Ok, "Your confirmed choice is being checked.");
    }
    if matches!(text.as_str(), "hi" | "hello" | "hail") {
        if chat
            .converse(session, |engaged| *engaged = Some(npc_actor))
            .is_none()
        {
            return answer(npc, D::Rejected, "Please try again shortly.");
        }
        *topic = Some(NpcVocationConversation {
            actor,
            npc,
            selected: None,
            confirmed: None,
        });
        return answer(
            npc,
            D::Ok,
            "Welcome. Ask about a vocation when you are ready to choose.",
        );
    }
    if chat.engaged(session) != Some(npc_actor) {
        return NpcTalkOutcome {
            unknown: false,
            inventory_changed: false,
            disposition: D::NoConversation,
            conversation: None,
            window: None,
        };
    }
    if matches!(text.as_str(), "bye" | "farewell" | "goodbye") {
        chat.end_engagement(session, npc_actor);
        *topic = None;
        return NpcTalkOutcome {
            unknown: false,
            inventory_changed: false,
            disposition: D::Ok,
            conversation: Some(NpcConversation {
                open: None,
                lines: vec!["Farewell.".into()],
            }),
            window: None,
        };
    }
    let Some(topic) = topic.as_mut() else {
        return NpcTalkOutcome {
            unknown: false,
            inventory_changed: false,
            disposition: D::NoConversation,
            conversation: None,
            window: None,
        };
    };
    // Oressa's pinned source redirects Monk elsewhere. It must never leave an old
    // four-vocation quote armed when the player asks about this different path.
    if text == "monk" {
        topic.selected = None;
        return answer(
            npc,
            D::Ok,
            "Monks follow a different path. I cannot confirm that choice here.",
        );
    }
    if let Some(selected) = BASE_VOCATIONS.into_iter().find(|key| *key == text) {
        topic.selected = Some(selected);
        return answer(
            npc,
            D::Ok,
            &format!(
                "Do you wish to become a {selected}? Say yes to confirm, or no to reconsider."
            ),
        );
    }
    if text == "no" {
        topic.selected = None;
        return answer(npc, D::Ok, "Take your time before choosing your vocation.");
    }
    if text == "yes" {
        let Some(selected) = topic.selected else {
            return answer(npc, D::Rejected, "Choose a vocation before confirming.");
        };
        let Some(occurrence) = new_occurrence() else {
            return answer(npc, D::Rejected, "Please try again shortly.");
        };
        topic.confirmed = Some(ConfirmedNpcVocation {
            actor,
            npc,
            selected,
            occurrence,
            retained: None,
        });
        return answer(npc, D::Ok, "Your choice is being checked.");
    }
    answer(
        npc,
        D::Ok,
        "You can choose knight, paladin, druid or sorcerer after reaching level 8.",
    )
}

/// Recheck current actor, NPC generation, engagement and range. The dispatch adapter also
/// compares the independently loaded durable session and scope fence before committing.
pub fn current_npc_owner<O, R>(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    choice: &ConfirmedNpcVocation<O, R>,
    chat: &impl NpcDialogueOwner,
) -> bool {
    if choice.actor != actor
        || runtime.owner_fence().is_err()
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
    {
        return false;
    }
    let visible = runtime.visible_entities();
    let Some(npc) = visible.npcs.iter().find(|npc| {
        npc.actor.placement_identity() == choice.npc.identity
            && npc.generation == choice.npc.generation
    }) else {
        return false;
    };
    let Some(player) = visible.players.iter().find(|player| player.actor == actor) else {
        return false;
    };
    runtime.npc_key(npc.actor) == Some(crate::content::npc_catalogue::DAWNPORT_CHOICE_NPC)
        && chat.engaged(session) == Some(npc.actor)
        && player.position.floor == npc.position.floor
        && player.position.x.abs_diff(npc.position.x) <= TALK_RANGE_TILES
        && player.position.y.abs_diff(npc.position.y) <= TALK_RANGE_TILES
}
