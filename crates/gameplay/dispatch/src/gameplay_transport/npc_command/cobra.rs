//! The poison prerequisite from Crystal 201189f8, data-global/npc/cobra.lua:48-69.
use super::*;
use crate::domain::condition::{ConditionType, DotElement};

pub(super) fn refuse_unpoisoned_greeting(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
) -> Option<NpcTalkOutcome> {
    if !oteryn_gameplay_npc::greeting::words(&intent.text)
        .iter()
        .any(|word| matches!(word.as_str(), "hi" | "hello" | "hail"))
    {
        return None;
    }
    let npc = service_context::live_npc(runtime, actor, session, intent.npc_actor)?;
    if npc.key != "oteryn:npc.cobra" {
        return None;
    }
    let speaker = service_context::speaker_position(runtime, actor, session)?;
    if speaker.floor != npc.position.floor
        || speaker.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
        || speaker.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
    {
        return Some(NpcTalkOutcome::only(NpcIntentDisposition::NotInRange));
    }
    let now = admission.owner_now().get();
    let states = admission.states.spell_states.try_lock().ok()?;
    let spell = states.get(runtime, actor, session)?.owned_conditions();
    if !spell.accepts_time(now) {
        return None;
    }
    let runtime_conditions = runtime
        .actor_conditions_at(actor, Some(session), now)
        .ok()?;
    let poisoned = spell
        .visible_statuses(now)
        .chain(runtime_conditions.visible_statuses(now))
        .any(|(kind, _)| kind == ConditionType::DamageOverTime(DotElement::Poison));
    if poisoned {
        return None;
    }
    Some(NpcTalkOutcome {
        conversation: Some(NpcConversation {
            open: None,
            lines: vec!["The cobra only admits those marked by poison.".into()],
        }),
        ..NpcTalkOutcome::only(NpcIntentDisposition::Ok)
    })
}
