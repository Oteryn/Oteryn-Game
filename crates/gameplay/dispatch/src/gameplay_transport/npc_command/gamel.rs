//! Crystal 201189f8 data-global/npc/gamel.lua:59–69.
use super::quest_dialogue::{self, Choice, Context, Step, Topic};
use super::*;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::UseCommand;
use oteryn_gameplay_entities::carrier::runtime_actor_companion::ReservedQuestCreatures;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use oteryn_gameplay_npc::npc_talk::GAMEL_AMBUSH_NPC as NPC;
pub(super) use oteryn_gameplay_quests::quest::loader::GAMEL_AMBUSH as TRANSITION;
use oteryn_gameplay_quests::quest::loader::GAMEL_AMBUSH_TRACK as MISSION;
pub(super) const LETTER: &str = "oteryn:item.tibia.i402";

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    if !oteryn_gameplay_npc::greeting::words(&intent.text)
        .iter()
        .any(|word| matches!(word.as_str(), "hi" | "hello" | "hail"))
    {
        return None;
    }
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |npc, _, _, tracks| {
            (npc.2 == NPC && tracks.get(MISSION) == Some(&1)).then(|| Choice {
                transition: Some(TRANSITION.into()),
                next_topic: Some(Topic::default()),
                reply: vec!["Bandits, to me!".into()],
            })
        },
    )
}

pub(super) fn reserve(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &NpcTalkIntent,
    context: &Context,
) -> Option<ReservedQuestCreatures> {
    if admission
        .states
        .npc_conversations
        .current_npc(&admission.states.chat, runtime, actor, session, intent)
        .as_ref()
        != Some(context)
    {
        return None;
    }
    let npc = super::service_context::live_npc(runtime, actor, session, intent.npc_actor)?;
    let visible = runtime.visible_entities();
    let occupied = visible
        .players
        .iter()
        .map(|entry| entry.position)
        .chain(visible.npcs.iter().map(|entry| entry.position))
        .chain(visible.creatures.iter().map(|entry| entry.position))
        .collect::<Vec<_>>();
    let movement = admission.movement_cells();
    let pin = runtime.content_pin().server_artifact_digest();
    let cells = [-1, 0, 1]
        .into_iter()
        .flat_map(|dx| [-1, 0, 1].map(|dy| (dx, dy)));
    let positions: Vec<_> = cells
        .filter_map(|(dx, dy)| {
            let at = MovementLocalPosition {
                x: npc.position.x.checked_add(dx)?,
                y: npc.position.y.checked_add(dy)?,
                floor: npc.position.floor,
            };
            (!admission.bundle_map().is_some_and(|map| {
                crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                })
                .is_none_or(|tile| map.facts.house_tile(tile) || map.facts.doors().blocks(tile))
            }) && !occupied
                .iter()
                .any(|pos| pos.x == at.x && pos.y == at.y && pos.floor == at.floor)
                && super::super::respawn_cell_admissible(
                    movement.index(),
                    movement.scope(),
                    admission.world_id,
                    pin,
                    &std::collections::BTreeSet::new(),
                    at,
                ))
            .then_some(at)
        })
        .take(2)
        .collect();
    if positions.len() != 2 {
        return None;
    }
    admission
        .active_generation?
        .native_gameplay()?
        .install_companion_policies(runtime)
        .ok()?;
    runtime
        .reserve_quest_creatures(actor, session, "oteryn:creature.bandit", &positions)
        .ok()
}
