//! Session-owned expert fight modes. World and skull facts always come from fenced SQL reads.
use super::ComposedFreshAdmission;
use crate::durability::{DurabilityError, pvp_live, spell_item_transaction as item_tx};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId};
use oteryn_gameplay_combat_rules::pvp::{ExpertMode as RuleMode, WorldPvpType};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode};

pub(super) fn effective_mode(mode: ExpertMode, black: bool) -> RuleMode {
    match mode {
        ExpertMode::Dove => RuleMode::Dove,
        ExpertMode::WhiteHand => RuleMode::WhiteHand,
        ExpertMode::YellowHand => RuleMode::YellowHand,
        ExpertMode::RedFist if black => RuleMode::Dove,
        ExpertMode::RedFist => RuleMode::RedFist,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn expert_context(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Option<(WorldPvpType, bool)> {
        runtime.player_control_facts(actor, session).ok()?;
        let fence = self.current_quest_fence(session).await.ok()??;
        let command = CommandRef::new(session, CommandId::new(command_id).ok()?);
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, runtime);
        pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime) = ctx;
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence.item_fence(),
                    command,
                    runtime.content_pin().server_artifact_digest(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let facts = pvp_live::read_expert_pvp_context_in_transaction(
                    &mut tx, owner.root, &authority,
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                tx.rollback().await?;
                Ok(facts)
            })
        })
        .await
        .ok()
    }

    pub(super) async fn set_expert_fight_modes(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        mut modes: ExpertFightModes,
    ) -> AttackIntentDisposition {
        let Some(permit) = self.spell_lane_permit().await else {
            return AttackIntentDisposition::Rejected;
        };
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if states.get(&runtime, actor, session).is_none() {
            return AttackIntentDisposition::Rejected;
        }
        let Some((world, black)) = self
            .expert_context(&runtime, actor, session, command_id, &permit)
            .await
        else {
            return AttackIntentDisposition::Rejected;
        };
        // Global fixes Hardcore to Red Fist; ignore the requested expert choice there.
        if world == WorldPvpType::Hardcore {
            modes.expert = ExpertMode::RedFist;
            modes.modes.secure = false;
        } else if world == WorldPvpType::Optional {
            // Expert controls are disabled outside Open PvP; ignore the requested choice.
            modes.expert = ExpertMode::Dove;
        } else if black && modes.expert == ExpertMode::RedFist {
            return AttackIntentDisposition::Rejected;
        }
        // No combat/skull lock on switching to peaceful modes is supported by the references.
        if self
            .states
            .attack
            .lock()
            .await
            .set_expert_modes(&runtime, actor, session, modes)
        {
            AttackIntentDisposition::Ok
        } else {
            AttackIntentDisposition::Rejected
        }
    }

    pub(super) async fn observe_expert_fight_modes(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<ExpertFightModes> {
        let permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        states.get(&runtime, actor, session)?;
        let mut modes = self.states.attack.lock().await.expert_modes(actor, session);
        // Read the current World contract and skull together under the existing deadline.
        let (world, black) = self
            .expert_context(&runtime, actor, session, 1, &permit)
            .await?;
        match world {
            WorldPvpType::Hardcore => {
                modes.expert = ExpertMode::RedFist;
                modes.modes.secure = false;
            }
            WorldPvpType::Optional => modes.expert = ExpertMode::Dove,
            WorldPvpType::Open if black && modes.expert == ExpertMode::RedFist => {
                modes.expert = ExpertMode::Dove;
            }
            WorldPvpType::Open => {}
        }
        self.states
            .attack
            .lock()
            .await
            .set_expert_modes(&runtime, actor, session, modes)
            .then_some(modes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_combat_rules::pvp::{
        PvpActor, PvpRefusal, PvpRelation, PvpTarget, check_pvp_legality,
    };

    #[test]
    fn pvp_expert_mode_matrix_keeps_party_guild_secure_and_skull_rules() {
        let target = PvpTarget {
            level: 20,
            in_protection_zone: false,
            on_no_pvp_tile: Some(false),
            marked: false,
        };
        let relation = PvpRelation {
            war_between: false,
            same_party_or_guild: Some(false),
            friendly_fire: false,
            target_aggressor_of_actor: false,
            target_aggressor_of_actor_group: false,
        };
        for (mode, neutral, group, marked) in [
            (ExpertMode::Dove, false, false, false),
            (ExpertMode::WhiteHand, false, true, false),
            (ExpertMode::YellowHand, false, true, true),
            (ExpertMode::RedFist, true, true, true),
        ] {
            let actor = PvpActor {
                level: 20,
                in_protection_zone: false,
                on_no_pvp_tile: Some(false),
                post_login_immune: false,
                black_skull: false,
                secure_mode: false,
                expert_mode: effective_mode(mode, false),
            };
            let check = |target: PvpTarget, relation: PvpRelation| {
                check_pvp_legality(WorldPvpType::Open, &actor, &target, &relation)
            };
            assert_eq!(
                check(target, relation).is_ok(),
                neutral,
                "{mode:?}: neutral"
            );
            assert!(
                check(
                    target,
                    PvpRelation {
                        target_aggressor_of_actor: true,
                        ..relation
                    }
                )
                .is_ok()
            );
            assert_eq!(
                check(
                    target,
                    PvpRelation {
                        target_aggressor_of_actor_group: true,
                        ..relation
                    }
                )
                .is_ok(),
                group
            );
            assert_eq!(
                check(
                    PvpTarget {
                        marked: true,
                        ..target
                    },
                    relation
                )
                .is_ok(),
                marked
            );
            // The same durable relation represents both party and guild allies.
            assert_eq!(
                check(
                    PvpTarget {
                        marked: true,
                        ..target
                    },
                    PvpRelation {
                        same_party_or_guild: Some(true),
                        target_aggressor_of_actor: true,
                        ..relation
                    }
                ),
                Err(PvpRefusal::PartyMember)
            );
            assert_eq!(
                check(
                    target,
                    PvpRelation {
                        same_party_or_guild: None,
                        ..relation
                    }
                ),
                Err(PvpRefusal::PartyState)
            );
            assert!(
                check_pvp_legality(WorldPvpType::Optional, &actor, &target, &relation).is_err()
            );
            assert!(check_pvp_legality(WorldPvpType::Hardcore, &actor, &target, &relation).is_ok());
            assert_eq!(
                check_pvp_legality(
                    WorldPvpType::Open,
                    &PvpActor {
                        secure_mode: true,
                        ..actor
                    },
                    &target,
                    &relation
                ),
                Err(PvpRefusal::SecureMode)
            );
            assert!(
                check_pvp_legality(
                    WorldPvpType::Open,
                    &PvpActor {
                        black_skull: true,
                        ..actor
                    },
                    &target,
                    &relation
                )
                .is_err()
            );
        }
        assert_eq!(effective_mode(ExpertMode::RedFist, true), RuleMode::Dove);
        assert_eq!(
            effective_mode(ExpertMode::YellowHand, true),
            RuleMode::YellowHand
        );
    }
}
