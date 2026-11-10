//! The copied Outburst HeartBossDeath: transform the current vortex, then its room players.
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::character_progression::ExperienceRewardOccurrence;
use crate::durability::quest_state::{
    QuestCause, QuestTransitionOutcome,
    native_kills::{NativeDeathSource, NativeQuestDeath},
    source_deaths::SourceDeathContext,
};
use crate::map::{overlay::TilePos, view::EntrySource};
use oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal;

const TRANSITION: &str = "oteryn:quest-transition/heart-destruction/outburst-room-death";

#[derive(Debug, Clone, PartialEq, Eq)]
struct Player {
    recipient: super::super::quest_kills::Recipient,
    occurrence: ExperienceRewardOccurrence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Vortex {
    tile: TilePos,
    placement: u64,
    revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HeartAction {
    vortex: Option<Vortex>,
    players: Vec<Player>,
    actors: Vec<ExactActorRef>,
}

impl HeartAction {
    pub(super) fn actors(&self) -> &[ExactActorRef] {
        &self.actors
    }

    pub(super) fn capture(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &mut ChannelRuntimeV1,
        begin: &SourceCreatureDeathSeal,
    ) -> Result<Self, ()> {
        if begin.definition_key() != "oteryn:creature.outburst" || !begin.current_for(runtime) {
            return Err(());
        }
        let active = owner.active_generation.ok_or(())?;
        let room = owner.effective_rooms().gate_monster_ai().ok_or(())?;
        let context = active
            .qualify_callback_source_context(
                room,
                runtime.content_pin(),
                PinnedActivitySourceRoom::CallbackOutburst,
            )
            .ok_or(())?;
        let (x, y, floor) = context
            .source_frame()
            .physical_position([32225, 31285, 14])
            .ok_or(())?;
        let tile = TilePos {
            x: u16::try_from(x).map_err(|_| ())?,
            y: u16::try_from(y).map_err(|_| ())?,
            floor: i8::try_from(floor.checked_neg().ok_or(())?).map_err(|_| ())?,
        };
        let map = owner.bundle_map().ok_or(())?;
        let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
            .map_err(|_| ())?
            .ok_or(())?;
        let vortex = stack
            .entries
            .iter()
            .find(|entry| entry.facts.appearance_id == 23483)
            .map(|entry| {
                let EntrySource::Base { placement_key, .. } = entry.source else {
                    return Err(());
                };
                Ok(Vortex {
                    tile,
                    placement: placement_key,
                    revision: map.facts.object_revision(placement_key),
                })
            })
            .transpose()?;
        let rule = oteryn_gameplay_quests::quest::source_deaths::rule(TRANSITION).ok_or(())?;
        if rule.creature != begin.definition_key() || owner.quest_catalogue.is_none() {
            return Err(());
        }
        // A NULL health synchronization commits the real lethal receipt without a
        // reward producer. Project that same receipt before requesting per-player
        // occurrences; the projection is idempotent and grants no loot or credit.
        let lethal = runtime
            .borrow_combat_death()
            .committed_lethal_receipt(begin.actor())
            .map_err(|_| ())?;
        if lethal.statistics_death_key().map_err(|_| ())? != begin.death_key() {
            return Err(());
        }
        runtime
            .borrow_combat_death()
            .project_committed_lethal(lethal)
            .map_err(|_| ())?;
        let sessions = owner.states.quest_sessions.lock().map_err(|_| ())?;
        let mut players = Vec::new();
        let mut actors = Vec::new();
        for (actor, at, session) in runtime.positioned_actor_census().map_err(|_| ())? {
            let Some(session) = session else { continue };
            let at = at.position();
            let [x, y, z] = context
                .source_frame()
                .source_position((at.x, at.y, at.floor))
                .ok_or(())?;
            let position = [x, y, -z];
            if !rule.recipient(position, false) {
                continue;
            }
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(|_| ())?;
            let copy = sessions
                .get(&session)
                .and_then(|entry| entry.copy.as_ref())
                .ok_or(())?;
            let observed_revision = copy.character_revision().ok_or(())?.get();
            actors.push(actor);
            if !rule.active(copy.tracks()) {
                continue;
            }
            let (occurrence, _) = runtime
                .borrow_combat_death()
                .party_reward_occurrence(begin.actor(), lease.character_id())
                .map_err(|_| ())?;
            players.push(Player {
                recipient: super::super::quest_kills::Recipient {
                    actor,
                    session,
                    character: *lease.character_id().as_bytes(),
                    lease_generation: lease.generation(),
                    death: NativeQuestDeath {
                        transition: TRANSITION.into(),
                        creature: begin.definition_key().into(),
                        observed_revision,
                        definition_hash: None,
                        source: NativeDeathSource::Source(SourceDeathContext {
                            scope: begin.death_key().runtime_scope(),
                            scope_generation: begin.death_key().scope_ownership_generation().get(),
                            character: *lease.character_id().as_bytes(),
                            session,
                            lease_generation: lease.generation(),
                            position,
                            dealt_damage: false,
                        }),
                    },
                },
                occurrence: ExperienceRewardOccurrence::from_bytes(occurrence).map_err(|_| ())?,
            });
        }
        Ok(Self {
            vortex,
            players,
            actors,
        })
    }

    /// Replay accepts only the same retained placement at its exact published revision.
    pub(super) fn publish_vortex(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
    ) -> Result<(), ()> {
        let Some(vortex) = &self.vortex else {
            return Ok(());
        };
        let map = owner.bundle_map().ok_or(())?;
        let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, vortex.tile)
            .map_err(|_| ())?
            .ok_or(())?;
        let current = stack.entries.iter().find(|entry| {
            matches!(entry.source, EntrySource::Base { placement_key, .. } if placement_key == vortex.placement)
        }).ok_or(())?;
        let revision = map.facts.object_revision(vortex.placement);
        if revision == vortex.revision
            && current.facts.appearance_id == 23483
            && map.facts.transform_movement_item_with_action(
                vortex.placement,
                vortex.revision,
                23483,
                23482,
                Some(14325),
            )
        {
            return Ok(());
        }
        if vortex.revision.checked_add(1) == Some(revision)
            && current.facts.appearance_id == 23482
            && map.facts.action_at(vortex.placement) == Some(14325)
        {
            Ok(())
        } else {
            Err(())
        }
    }

    pub(super) async fn execute(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        action: &PreparedSourceDeathAction,
    ) -> Result<(), ()> {
        {
            let runtime = owner.runtime.lock().await;
            if !action.current_for(&runtime) {
                return Err(());
            }
            self.publish_vortex(owner)?;
        }
        let catalogue = owner.quest_catalogue.as_ref().ok_or(())?;
        for player in &self.players {
            let recipient = &player.recipient;
            let fence = match owner.current_quest_fence(recipient.session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => continue,
                Err(()) => return Err(()),
            };
            if fence.runtime_scope != action.begin().death_key().runtime_scope()
                || fence.scope_ownership_generation
                    != action.begin().death_key().scope_ownership_generation()
                || fence.character_id.as_bytes() != &recipient.character
                || fence.character_lease_generation != recipient.lease_generation
            {
                continue;
            }
            {
                let runtime = owner.runtime.lock().await;
                if !action.current_for(&runtime) {
                    return Err(());
                }
                if !runtime
                    .bound_attacker_lease(recipient.actor, recipient.session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == &recipient.character
                            && lease.generation() == recipient.lease_generation
                    })
                {
                    continue;
                }
            }
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let result = slot
                .commit_native_quest_kill_with_admission(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestCause::CreatureDeath(player.occurrence),
                    &recipient.death,
                    std::sync::Arc::clone(catalogue),
                    &owner.premium_coordinator,
                )
                .await;
            drop(slot);
            match result {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_)
                    | QuestTransitionOutcome::Refused(_)
                    | QuestTransitionOutcome::ObligationClosed,
                ) => {}
                Err(_) => return Err(()),
            }
        }
        Ok(())
    }

    pub(super) async fn refresh_after_release(&self, owner: &ComposedFreshAdmission<'_, '_, '_>) {
        // Retain the normal refresh retry before the first await, including
        // recipients not yet reached if this detached turn is cancelled.
        for player in &self.players {
            owner.schedule_quest_refresh(player.recipient.session);
        }
        for player in &self.players {
            owner
                .refresh_quest_session(player.recipient.session, false)
                .await;
        }
    }
}
