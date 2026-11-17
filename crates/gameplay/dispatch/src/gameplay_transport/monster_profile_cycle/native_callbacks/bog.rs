//! Crystal201189f plagirath_bog.lua44-58: immediate TARGET outfit, never caster.
use super::*;
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::SourceCallbackCast;
const BOG: &str = "oteryn:ability.spell.plagirath_bog";
pub(super) fn apply(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    source: &SourceCallbackCast,
    proposal: &ProfileAbilityProposal,
    now: u64,
) -> Option<bool> {
    let parent = source.native_parent()?;
    if parent.entry().ability.key != BOG
        || source.parent().key != BOG
        || parent.content() != native.source_digest()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
        || !source.validates_current_native_proposal(runtime, proposal)
    {
        return None;
    }
    use crate::player_lethal::PlayerLethalVitals;
    let session = runtime
        .positioned_actor_census()
        .ok()?
        .into_iter()
        .find(|(actor, _, _)| *actor == proposal.target)?
        .2;
    let (speed, immunities) = if let Some(session) = session {
        if runtime
            .player_control_facts(proposal.target, session)
            .ok()?
            .control_loss
            .is_some()
        {
            return None;
        }
        (
            u16::try_from(
                states
                    .get(runtime, proposal.target, session)?
                    .owned_base_speed(),
            )
            .ok()?,
            vec![],
        )
    } else {
        let current = runtime.companion_snapshot(proposal.target).ok()?;
        if current.health <= 0 {
            return None;
        }
        (
            u16::try_from(current.state.policy.base_speed).ok()?,
            current.state.policy.condition_immunities.clone(),
        )
    };
    let appearance = native
        .spell_appearances()?
        .for_creature("oteryn:creature.bog_raider", "definition-r1")
        .filter(|appearance| appearance.look_type() == 299)?;
    let definition = crate::foundation::ConditionDefinition::new(
        "source:plagirath:bog-target-outfit",
        1,
        crate::foundation::ConditionValues::Outfit {
            duration_ms: 10_000,
            look_type: 299,
        },
    )?
    .with_appearance(appearance.selection())?;
    let digest: [u8; 32] = Sha256::new()
        .chain_update(native.source_digest())
        .chain_update(proposal.occurrence.id().as_str().as_bytes())
        .finalize()
        .into();
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(digest);
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let stamp = runtime.issue_owner_work().ok()?;
    states
        .apply_source_appearance_batch_current(
            runtime,
            stamp,
            proposal.issuer,
            &[definition],
            &[crate::player_lethal::SourceAppearanceTarget {
                actor: proposal.target,
                session,
                immunities,
                facts: crate::foundation::ApplicationFacts {
                    now,
                    base_speed: speed,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: session.is_some(),
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes(occurrence),
                },
            }],
        )
        .map(|_| true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::foundation::{ChannelContentPin, ChannelId, GameSessionId, NodeId, WorldId};
    use crate::source_callback_cast::CallbackCastOwner;
    use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn id(tag: u8) -> [u8; 16] {
        let mut id = [tag; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    #[test]
    fn srx3_plagirath_real_native_bog_applies_target_outfit_once_and_keeps_caster() -> TestResult {
        let world = WorldId::decode(&id(60)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("actual native generation")?;
        assert!(
            native
                .spell_appearances()
                .and_then(|a| a.for_creature("oteryn:creature.bog_raider", "definition-r1"))
                .filter(|a| a.look_type() == 299)
                .is_some()
        );
        let content: ChannelContentPin = pin.into_channel_pin();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(61)).map_err(|e| format!("{e:?}"))?,
            NodeId::decode(&id(62)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            content,
        )?;
        runtime.install_companion_policies(native.policies().clone())?;
        let session = GameSessionId::decode(&id(63)).map_err(|e| format!("{e:?}"))?;
        let reservation = runtime.reserve_fresh_session(session)?;
        let target = runtime.commit_fresh_session(reservation)?;
        runtime.initialize_first_entry_position(target)?;
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("actual player condition initialization")?;
        let other_session = GameSessionId::decode(&id(64)).map_err(|e| format!("{e:?}"))?;
        let reservation = runtime.reserve_fresh_session(other_session)?;
        let other = runtime.commit_fresh_session(reservation)?;
        let origin = runtime.read_actor_position(target)?.position();
        runtime.initialize_pinned_test_position(
            other,
            MovementLocalPosition {
                x: origin.x + 2,
                ..origin
            },
        )?;
        states
            .initialize(
                &runtime,
                other,
                other_session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("actual player condition initialization")?;
        let issuer = runtime
            .spawn_arena_opponent(
                "oteryn:creature.plagirath",
                MovementLocalPosition {
                    x: origin.x + 1,
                    ..origin
                },
                |_, _| Some(true),
            )
            .map_err(|e| format!("{e:?}"))?;
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == "oteryn:creature.plagirath")
            .ok_or("actual source caster")?;
        let source = (0..32)
            .find_map(|index| {
                SourceCallbackCast::qualify_native(
                    native,
                    &record.profile.target,
                    index,
                    ScheduleList::Attack,
                )
                .ok()
                .flatten()
                .filter(|s| s.parent().key == BOG)
            })
            .ok_or("actual source-qualified Bog")?;
        assert!(source.native_parent().is_some());
        // The original Attack proposal comes from the production scheduler,
        // which retains its actual combat target independently of the marker's
        // default needs_target field.
        let mut ai = crate::ai_think::CreatureAiTable::default();
        ai.admit_native_creature(&runtime, native, issuer, &record.profile.target, 0)
            .map_err(|e| format!("actual native Bog admission: {e:?}"))?;
        let Some(crate::content::ProjectV2AuthoringProfileData::Behavior(behavior)) =
            record.behavior.as_ref().map(|profile| &profile.data)
        else {
            return Err("actual native Bog behavior".into());
        };
        let (entry_index, entry) = behavior
            .attacks
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.ability.key == BOG)
            .ok_or("actual native Bog attack slot")?;
        assert_eq!(entry.interval_ms, 20_000);
        assert_eq!(entry.chance_ppm, 250_000);
        let think_ms = crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS;
        let first_due_sequence = entry.interval_ms / think_ms;
        assert_eq!(entry.interval_ms % think_ms, 0);
        let occurrence = crate::ai_think::profile_schedule::decision_occurrence(
            crate::ai_think::ThinkOccurrence {
                actor: issuer,
                sequence: first_due_sequence,
            },
        );
        // Select the test's decision input through the real draw API. Changing unrelated
        // content must not turn six genuine 25% source chances into a no-hit fixture.
        let decision_root = (0_u8..=u8::MAX)
            .map(|seed| oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([seed; 32]))
            .find(|root| {
                oteryn_simulation_determinism::deterministic_decision_u64(
                    root,
                    occurrence,
                    ScheduleList::Attack.purpose(),
                    entry_index as u64,
                )
                .is_ok_and(|draw| draw % 1_000_000 < u64::from(entry.chance_ppm))
            })
            .ok_or("deterministic first-due Bog fixture draw")?;
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .map_err(|e| format!("{e:?}"))?;
        let candidates = [crate::ai_think::targeting::TargetCandidate {
            actor: target,
            position: origin,
            eligible: true,
            invisible: false,
            health: 100,
            damage: 0,
        }];
        let (proposal, now) = (0..=1200)
            .find_map(|tick| {
                ai.run_due_thinks(
                    &mut runtime,
                    tick * 100_000,
                    &decision_root,
                    &revisions,
                    &candidates,
                )
                .into_iter()
                .flat_map(|(_, report)| report.proposals)
                .find(|proposal| proposal.ability.key == BOG)
                .map(|proposal| (proposal, tick * 100_000))
            })
            .ok_or("actual native AI did not select source Bog in 120 seconds")?;
        assert_eq!(proposal.issuer, issuer);
        assert_eq!(proposal.entry_index, entry_index);
        assert_eq!(
            now,
            (first_due_sequence - 1) * think_ms * 1_000,
            "the actual scheduler must select Bog on its first due source slot"
        );
        assert_eq!(
            proposal.target, target,
            "the native Attack scheduler retains its actual selected player"
        );
        assert!(source.validates_current_native_proposal(&runtime, &proposal));
        let mut owner = CallbackCastOwner::new_current(&runtime).map_err(|e| format!("{e:?}"))?;
        let stamp = runtime.issue_owner_work()?;
        assert!(
            owner
                .schedule_current(
                    &runtime,
                    stamp,
                    &source,
                    &proposal,
                    SemanticTimeMicros::from_micros(now)
                )
                .map_err(|e| format!("{e:?}"))?
        );
        let before = runtime.companion_snapshot(issuer)?;
        assert!(apply(&mut runtime, &mut states, native, &source, &proposal, now).is_some());
        assert_eq!(
            states
                .get(&runtime, target, session)
                .ok_or("actual target state")?
                .owned_conditions()
                .outfit_at(now + 9_999_999),
            Some(299)
        );
        assert_eq!(
            states
                .get(&runtime, target, session)
                .ok_or("actual target state")?
                .owned_conditions()
                .outfit_at(now + 10_000_000),
            None
        );
        assert_eq!(
            states
                .get(&runtime, other, other_session)
                .ok_or("other player state")?
                .owned_conditions()
                .outfit_at(now),
            None
        );
        assert_eq!(
            runtime.companion_snapshot(issuer)?,
            before,
            "target outfit must not mutate caster"
        );
        let revision = states
            .get(&runtime, target, session)
            .ok_or("target")?
            .revision();
        let stamp = runtime.issue_owner_work()?;
        assert!(
            !owner
                .schedule_current(
                    &runtime,
                    stamp,
                    &source,
                    &proposal,
                    SemanticTimeMicros::from_micros(now)
                )
                .map_err(|e| format!("{e:?}"))?
        );
        assert_eq!(
            states
                .get(&runtime, target, session)
                .ok_or("target")?
                .revision(),
            revision
        );
        let mut unqualified = source.clone();
        unqualified.content = [0; 32];
        assert!(
            apply(
                &mut runtime,
                &mut states,
                native,
                &unqualified,
                &proposal,
                now
            )
            .is_none()
        );
        assert_eq!(
            states
                .get(&runtime, target, session)
                .ok_or("target")?
                .revision(),
            revision
        );
        let actor_name = format!(
            "actor:{}",
            crate::creature_damage_spell::hex(&issuer.placement_identity())
        );
        let mut retargeted = proposal.clone();
        retargeted.target = other;
        let other_name = format!(
            "actor:{}",
            crate::creature_damage_spell::hex(&other.placement_identity())
        );
        retargeted.intent =
            crate::ability::AiAbilityAdapter::normalize(&actor_name, &[&other_name])
                .map_err(|e| format!("{e:?}"))?;
        let stamp = runtime.issue_owner_work()?;
        assert!(
            owner
                .schedule_current(
                    &runtime,
                    stamp,
                    &source,
                    &retargeted,
                    SemanticTimeMicros::from_micros(now)
                )
                .is_err(),
            "same source occurrence cannot adopt a different current target"
        );
        Ok(())
    }
}
