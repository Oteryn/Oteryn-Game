use super::*;
use oteryn_gameplay_social_rules::shared_experience;

/// The active content owner, the exact physical definition/revision, and the master relation
/// qualify aggression. Attackable alone is not hostility; summons/familiars are not monsters.
pub(super) fn aggressive_monster(
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    actor: ExactActorRef,
) -> bool {
    if content.source_digest() != runtime.content_pin().server_artifact_digest() {
        return false;
    }
    let Ok(snapshot) = runtime.companion_snapshot_including_dead(actor) else {
        return false;
    };
    if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
        return false;
    }
    content
        .creature_profiles()
        .records
        .iter()
        .find(|record| {
            record.profile.target.key == snapshot.state.policy.definition_key
                && record.profile.target.revision == snapshot.state.policy.definition_revision
        })
        .is_some_and(|record| {
            content.qualifies_current_creature_profile(content.source_digest(), &record.profile)
                && matches!(record.behavior.as_ref().map(|profile| &profile.data),
                Some(crate::content::ProjectV2AuthoringProfileData::Behavior(behavior))
                    if behavior.targeting.hostile && behavior.targeting.can_target)
        })
}

impl ActorAttack {
    pub(super) fn record_shared_experience_activity(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: SemanticTimeMicros,
    ) -> bool {
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(self.actor, self.session)
                .is_ok_and(|control| control.control_loss.is_none())
        {
            return false;
        }
        let Ok(lease) = runtime.bound_attacker_lease(self.actor, self.session) else {
            return false;
        };
        let next = SharedExperienceActivity {
            character: *lease.character_id().as_bytes(),
            lease_generation: lease.generation(),
            at,
        };
        shared_experience::record_shared_experience_activity(
            &mut self.shared_experience_activity,
            next,
        )
    }
}

#[allow(
    dead_code,
    reason = "D #2172 integrates these native activity APIs in its separate draft"
)]
impl ChannelAttackStates {
    /// Retained event data for death capture; reading never refreshes an activity clock.
    pub(in crate::gameplay_transport) fn shared_experience_cells(
        &self,
    ) -> Vec<SharedExperienceActivity> {
        self.entries
            .iter()
            .filter_map(|e| e.shared_experience_activity)
            .collect()
    }

    /// Preallocate the existing per-player entry BEFORE physical spell installation. No
    /// activity is granted here; phase 2's receipt recorder never allocates or creates it.
    pub fn prepare_shared_experience_record(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.entry(runtime, actor, session).is_some()
    }

    /// Native activity data only. D must independently resolve current strong membership,
    /// presence, range, levels and leader PZ at the kill; this read grants no authority.
    pub fn shared_experience_active(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> bool {
        self.committed_shared_experience_activity(runtime, actor, session, now)
            .is_some_and(shared_experience::shared_experience_active)
    }

    /// Read retained native data without renewing the event time. The caller supplies
    /// its independently observed semantic clock; future events refuse this observation.
    pub fn committed_shared_experience_activity(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        observed_at: SemanticTimeMicros,
    ) -> Option<SharedExperienceActivityObservation> {
        let entry = self
            .index(actor, session)
            .map(|index| &self.entries[index])?;
        let activity = entry.shared_experience_activity?;
        if runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|control| control.control_loss.is_none())
        {
            return None;
        }
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        shared_experience::committed_shared_experience_activity(
            activity,
            lease.character_id().as_bytes(),
            lease.generation(),
            observed_at,
        )
    }

    /// Call under the SAME runtime -> states -> attack guards after successful native owner
    /// install, using that attempt's actual batch and transaction-qualified SourcePartyWorld.
    /// Captured party data describes membership when the committed heal happened; it never
    /// authorizes current membership at a later kill. Replay cannot extend the activity clock.
    pub fn record_committed_shared_experience_batch(
        &mut self,
        runtime: &ChannelRuntimeV1,
        content: &NativeGameplayState,
        batch: &crate::foundation::runtime_actor_spell_types::OwnerCombatBatch,
        receipt: &crate::foundation::runtime_actor_spell_types::CombatBatchReceipt,
        party: Option<&crate::gameplay_transport::party_spell_owner::SourcePartyWorld>,
    ) -> bool {
        use crate::foundation::runtime_actor_spell_types::OwnerCombatChange;
        if !receipt.applied
            || batch.caster != receipt.caster
            || batch.command != receipt.command
            || content.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return false;
        }
        let session = batch.command.game_session_id();
        let Ok(lease) = runtime.bound_attacker_lease(batch.caster, session) else {
            return false;
        };
        if lease.character_id() != batch.attacker
            || lease.generation() != batch.current_lease_generation
        {
            return false;
        }
        let eligible = receipt.effects.iter().any(|effect| {
            let Some(health) = effect.health.filter(|health| health.applied) else {
                return false;
            };
            let Some(proposed) = batch.effects.iter().find(|proposed| {
                proposed.target == effect.target && proposed.sub_ordinal == effect.sub_ordinal
            }) else {
                return false;
            };
            match &proposed.change {
                OwnerCombatChange::Damage { .. } | OwnerCombatChange::DamageWithHealing { .. }
                    if health.health_after < health.health_before =>
                {
                    aggressive_monster(runtime, content, effect.target)
                }
                OwnerCombatChange::Heal { .. } if health.health_after > health.health_before => {
                    let Some(party) = party else {
                        return false;
                    };
                    let source = party.source();
                    source.party_id.is_some()
                        && source.world == *runtime.binding().world_id().as_bytes()
                        && source.character == *lease.character_id().as_bytes()
                        && source
                            .members
                            .iter()
                            .any(|member| member.character == source.character)
                        && party
                            .actor_for_id(u64::from(effect.target.actor_local_id()))
                            .is_some_and(|(actor, target_session)| {
                                actor == effect.target
                                    && runtime
                                        .bound_attacker_lease(actor, target_session)
                                        .is_ok_and(|target| {
                                            heal_credits_party_member(
                                                source,
                                                target.character_id().as_bytes(),
                                            )
                                        })
                            })
                }
                _ => false,
            }
        });
        if !eligible {
            return false;
        }
        let Some(original_event_at) = batch.now_us else {
            return false;
        };
        if original_event_at.get() / 1000 != batch.now_ms {
            return false;
        }
        let Some(index) = self.index(batch.caster, session) else {
            return false;
        };
        self.entries[index].record_shared_experience_activity(runtime, original_event_at)
    }
}

/// Shared experience credits a heal only when it heals ANOTHER member of the caster's
/// captured party. A self-heal never counts, including an area heal such as Mass Healing
/// that reaches only the caster.
fn heal_credits_party_member(
    source: &crate::foundation::PartySnapshot,
    target_character: &[u8; 16],
) -> bool {
    shared_experience::heal_credits_party_member(
        &source.character,
        source.members.iter().map(|member| &member.character),
        target_character,
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod shared_experience_activity_tests {
    use super::*;
    use crate::foundation::{CharacterId, CharacterLease};

    #[test]
    fn activity_uses_current_native_lease_and_two_minute_window_without_future_credit() {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x71);
        let mut raw = [1; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        let character = CharacterId::decode(&raw).expect("character");
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(character, 7).expect("lease"),
            )
            .expect("bind");
        let mut states = ChannelAttackStates::default();
        assert!(states.prepare_shared_experience_record(&runtime, actor, session));
        let at = SemanticTimeMicros::from_micros(2_000_000);
        let index = states.index(actor, session).expect("entry");
        assert!(states.entries[index].record_shared_experience_activity(&runtime, at));
        assert!(!states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(1_999_999)
        ));
        assert!(states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(122_000_000)
        ));
        assert!(!states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(122_000_001)
        ));
        let mut successor_bytes = [3; 16];
        successor_bytes[6] = 0x70;
        successor_bytes[8] = 0x80;
        let successor = GameSessionId::decode(&successor_bytes).expect("successor session");
        runtime
            .rebind_player_session(
                actor,
                session,
                successor,
                CharacterLease::new(character, 8).expect("new lease"),
            )
            .expect("native successor rebind");
        assert!(
            !states.shared_experience_active(&runtime, actor, session, at),
            "retained activity is not new lease evidence"
        );
    }

    #[test]
    fn unbound_actor_has_no_activity_and_out_of_order_event_does_not_rewind_it() {
        let (mut runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x72);
        let mut states = ChannelAttackStates::default();
        let entry = states.entry(&runtime, actor, session).expect("entry");
        assert!(
            !entry.record_shared_experience_activity(&runtime, SemanticTimeMicros::from_micros(20))
        );
        let mut raw = [2; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(CharacterId::decode(&raw).expect("character"), 1)
                    .expect("lease"),
            )
            .expect("bind");
        assert!(
            entry.record_shared_experience_activity(&runtime, SemanticTimeMicros::from_micros(20))
        );
        assert!(
            !entry.record_shared_experience_activity(&runtime, SemanticTimeMicros::from_micros(19))
        );
        assert!(states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(20)
        ));
        for observed in [21, 22, 120_000_021] {
            let activity = states
                .committed_shared_experience_activity(
                    &runtime,
                    actor,
                    session,
                    SemanticTimeMicros::from_micros(observed),
                )
                .expect("current native observation");
            assert_eq!(
                activity.original_event_at,
                SemanticTimeMicros::from_micros(20)
            );
            assert_eq!(
                activity.observed_at,
                SemanticTimeMicros::from_micros(observed)
            );
        }
        assert!(!states.shared_experience_active(
            &runtime,
            actor,
            session,
            SemanticTimeMicros::from_micros(120_000_021),
        ));
    }
}
