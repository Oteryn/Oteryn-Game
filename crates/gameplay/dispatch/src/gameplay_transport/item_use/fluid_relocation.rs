//! Crystal 201189f8 data-global/scripts/actions/other/fluids.lua:36-47,151-184.
use super::*;
use crate::foundation::{MovementLocalPosition, MovementPositionSnapshot};
use crate::map::{boot::BundleMap, floor::BundleDoors, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

type Mover = (ExactActorRef, MovementPositionSnapshot, GameSessionId);

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "one carried item fence, source placement, and native group relocation"
    )]
    pub(super) async fn commit_fluid_relocation(
        &self,
        actor: ExactActorRef,
        command: &UseCommand,
        fence: CurrentCharacterItemFence,
        request: ItemFluidRequest,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
    ) -> UseDisposition {
        let Some(relocation) = &callback.effects.fluid_relocation else {
            return UseDisposition::Rejected;
        };
        let Some((target, _)) = crate::map::view::placement_of(key) else {
            return UseDisposition::Rejected;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return UseDisposition::Rejected;
        };
        let session = command.game_session_id;
        use oteryn_gameplay_combat_rules::pvp::WorldPvpType;
        use oteryn_protocol_oteryn::pvp_expert::ExpertMode;
        let policy = {
            let runtime = self.runtime.lock().await;
            self.expert_context(&runtime, actor, session, command.command_id, &permit)
                .await
        };
        let Some((world, _)) = policy else {
            return UseDisposition::Rejected;
        };
        let optional_pvp = Some(world == WorldPvpType::Optional);
        let staging = MovementLocalPosition {
            x: i32::from(target.x) + i32::from(relocation.staging_dx),
            y: i32::from(target.y) + i32::from(relocation.staging_dy),
            floor: -i16::from(target.floor),
        };
        let destination = super::super::quest_callbacks::local(relocation.destination);
        let doors = BundleDoors(&map.facts);
        let (movers, proofs) = {
            let runtime = self.runtime.lock().await;
            let Ok(states) = self.states.spell_states.try_lock() else {
                return UseDisposition::Rejected;
            };
            let current_source = |at: MovementPositionSnapshot| {
                super::super::quest_callbacks::native(at.position()).is_some_and(|from| {
                    callback.reaches(
                        Position {
                            x: from.x,
                            y: from.y,
                            floor: from.floor,
                        },
                        Position {
                            x: target.x,
                            y: target.y,
                            floor: target.floor,
                        },
                    )
                })
            };
            let Ok(use_origin) = runtime.read_actor_position(actor) else {
                return UseDisposition::Rejected;
            };
            if !current_source(use_origin)
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, session)
                || !self.states.player_trades.physical(session, None)
                || map.facts.object_revision(key) != revision
                || !super::super::quest_callbacks::matches(map, callback, key)
            {
                return UseDisposition::StaleState;
            }
            let Ok(census) = runtime.positioned_melee_census() else {
                return UseDisposition::Rejected;
            };
            let mut staged = census
                .iter()
                .filter(|(_, at, _)| at.position() == staging)
                .peekable();
            let movers: Vec<Mover> = if staged.peek().is_none() {
                vec![(actor, use_origin, session)]
            } else {
                staged
                    .filter_map(|&(who, at, member)| member.map(|member| (who, at, member)))
                    .collect()
            };
            // Fandom archive The_Pits_of_Inferno_Quest/Spoiler, Goshnar:
            // Optional PvP and Dove in Open PvP can share the landing.
            let stack = match world {
                WorldPvpType::Optional => true,
                WorldPvpType::Open => {
                    let Ok(attack) = self.states.attack.try_lock() else {
                        return UseDisposition::Rejected;
                    };
                    let dove =
                        |who, member| attack.expert_modes(who, member).expert == ExpertMode::Dove;
                    movers.iter().all(|&(who, _, member)| dove(who, member))
                        && census.iter().all(|&(who, at, member)| {
                            at.position() != destination
                                || member.is_none_or(|member| dove(who, member))
                        })
                }
                WorldPvpType::Hardcore => false,
            };
            if movers.len() > 1 && !stack {
                return UseDisposition::NotPossible;
            }
            let mut proofs = Vec::with_capacity(movers.len());
            for &(who, expected, member) in &movers {
                if states.is_dead(who)
                    || !states.spell_idle(&runtime, who, member)
                    || !self.states.player_trades.physical(member, None)
                    || self.has_pending_quest_callback(member)
                {
                    return UseDisposition::Rejected;
                }
                let blocked =
                    super::super::pvp_live::pz_blocked(&states, who, member, self.owner_now());
                let Ok(proof) = crate::map::floor::prepare_quest_callback_stacked_teleport(
                    &runtime,
                    self.movement_cells(),
                    who,
                    member,
                    expected,
                    destination,
                    blocked,
                    &doors,
                    stack,
                ) else {
                    return UseDisposition::NotPossible;
                };
                proofs.push(proof);
            }
            (movers, proofs)
        };
        let disposition = self
            .commit_fluid(actor, command, fence, request, None)
            .await
            .unwrap_or_else(|d| d);
        if disposition != UseDisposition::Committed || movers.is_empty() {
            return disposition;
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Ok(results) =
            runtime.commit_quest_relocation(proofs, || map.facts.object_revision(key) == revision)
        else {
            return UseDisposition::StaleState;
        };
        for ((who, from, member), arrived) in movers.into_iter().zip(results) {
            let now = self.owner_now().get();
            states.stage_special_movement_step(
                &runtime,
                map,
                who,
                member,
                now,
                Some(from.position()),
                arrived.position(),
                optional_pvp,
            );
            let underwater = super::super::paced_movement::underwater_drown::underwater;
            states.stage_underwater_step(
                &runtime,
                who,
                member,
                now,
                underwater(map, from.position()),
                underwater(map, arrived.position()),
            );
            if let Some(at) = super::super::quest_callbacks::native(from.position()) {
                map.facts
                    .emit_movement_effect(at, u16::from(relocation.source_effect));
            }
            map.facts.emit_movement_effect(
                super::super::quest_callbacks::tile(relocation.destination),
                u16::from(relocation.destination_effect),
            );
            self.states.chat.close_out_of_range(&runtime, member);
            self.states.chat.publish_private_quest_speech(
                &runtime,
                who,
                member,
                &relocation.speech,
            );
        }
        UseDisposition::Committed
    }
}
