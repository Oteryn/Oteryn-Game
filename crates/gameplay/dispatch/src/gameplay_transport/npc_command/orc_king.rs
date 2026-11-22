//! Crystal 201189f8, data-global/npc/the_orc_king.lua:48-63,156.
use super::quest_dialogue::{self, Choice, Step};
use super::*;

const NPC: &str = "oteryn:npc.the_orc_king";
const TRACK: &str = "oteryn:quest-progress/orc-fortress/king-greeted";
pub(super) const CLAIM: &str = "oteryn:quest-transition/orc-fortress/king-greeted";
const AMBUSH: &str = "Arrrrgh! A dirty paleskin! To me my children! Kill them my guards!";
const GREETED: &str =
    "Harrrrk! You think you are strong now? You shall never escape my wrath! I am immortal!";
const GUARDS: [&str; 8] = [
    "oteryn:creature.slime",
    "oteryn:creature.slime",
    "oteryn:creature.slime",
    "oteryn:creature.orc_warlord",
    "oteryn:creature.orc_warlord",
    "oteryn:creature.orc_leader",
    "oteryn:creature.orc_leader",
    "oteryn:creature.orc_leader",
];

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
) -> Option<Step> {
    if !oteryn_gameplay_npc::greeting::words(&intent.text)
        .iter()
        .any(|word| matches!(word.as_str(), "hi" | "hello" | "hail"))
    {
        return None;
    }
    let catalogue = admission.quest_catalogue.as_deref()?;
    let transition = catalogue.transition(CLAIM)?;
    let [effect] = transition.effects.as_slice() else {
        return None;
    };
    use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffectKind};
    if effect.track != TRACK
        || effect.from != QuestComparison::Eq(0)
        || effect.effect != QuestEffectKind::Set(1)
    {
        return None;
    }
    let initial = catalogue.track(TRACK)?.initial;
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, _, _, tracks| {
            if context.2 != NPC {
                return None;
            }
            let first = tracks.get(TRACK).copied().unwrap_or(initial) == 0;
            Some(Choice {
                transition: first.then(|| CLAIM.to_owned()),
                next_topic: None,
                reply: vec![if first { AMBUSH } else { GREETED }.into()],
            })
        },
    )
}

pub(super) fn publish(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    outcome: &mut NpcTalkOutcome,
) {
    let session = command.game_session_id;
    let Some(npc) = service_context::live_npc(runtime, actor, session, intent.npc_actor) else {
        return;
    };
    if npc.key != NPC
        || !command.item_fence.is_some_and(|fence| {
            runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        })
    {
        return;
    }
    if admission.states.chat.end_engagement(session, npc.actor)
        && let Some(conversation) = &mut outcome.conversation
    {
        conversation.open = None;
    }
    let at = crate::foundation::MovementLocalPosition {
        x: npc.position.x,
        y: npc.position.y,
        floor: npc.position.floor,
    };
    let _ = admission.spawn_quest_creatures(runtime, at, &GUARDS);
}
