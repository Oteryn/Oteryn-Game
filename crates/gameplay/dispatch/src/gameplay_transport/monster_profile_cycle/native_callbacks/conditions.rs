//! Canonical native-creature housekeeping; this never creates a monster think.
use super::*;

pub(super) fn expire(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    actors: &[ExactActorRef],
    now: u64,
) -> Result<(), ()> {
    runtime.owner_fence().map_err(|_| ())?;
    // Only real completed native think occurrences own this post-attack phase.
    // A condition deadline or another actor's timer does not create an occurrence.
    for &actor in actors {
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            continue;
        };
        if !native.creature_profiles().records.iter().any(|record| {
            record.profile.target.key == snapshot.state.policy.definition_key
                && record.profile.target.revision == snapshot.state.policy.definition_revision
        }) {
            continue;
        }
        let mut preview = runtime
            .actor_conditions_at(actor, None, now)
            .map_err(|_| ())?
            .clone();
        if preview.expire_non_ticking(now) {
            let digest = Sha256::new()
                .chain_update(b"oteryn:native-creature-condition-expiry:v1")
                .chain_update(actor.placement_identity())
                .chain_update(actor.actor_local_generation().to_be_bytes())
                .chain_update(now.to_be_bytes())
                .finalize();
            let mut id = [0; 16];
            id.copy_from_slice(&digest[..16]);
            let at = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
            let plan = runtime
                .prepare_actor_condition_expiry(
                    actor,
                    None,
                    DecisionOccurrenceId::from_bytes(id),
                    at,
                )
                .map_err(|_| ())?;
            runtime.commit_actor_condition(&plan, at).map_err(|_| ())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_native_fractional_marker_waits_for_actual_actor_postthink_phase() -> Result<(), String>
    {
        std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(),String> {
            use crate::foundation::{ActorConditionTransition, ApplicationFacts, ConditionDefinition,
                ConditionSource, ConditionSourceKind, ConditionValues};
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            use oteryn_simulation_determinism::{GameplayDecisionRoot, SemanticTimeMicros};
            let mut scene=crate::gameplay_transport::wire_position::test_scene::source_scene()?;
            let native=scene.controller.active().and_then(|active|active.native_gameplay()).ok_or("actual native source")?;
            native.install_companion_policies(&mut scene.runtime).map_err(|e|format!("{e:?}"))?;
            let actor=scene.runtime.spawn_arena_opponent("oteryn:creature.rat",MovementLocalPosition{x:scene.physical.x.checked_add(1).ok_or("adjacent source cell")?,..scene.physical},|_,_|Some(true)).map_err(|e|format!("{e:?}"))?;
            let marker=ConditionDefinition::new("source.omrafir.beam.fractional-marker",1,
                ConditionValues::SpellRegeneration{duration_ms:5000,sub_id:88888,health_gain:0,
                    health_interval_ms:5000,mana_gain:0,mana_interval_ms:1000}).ok_or("literal source marker")?;
            let root=GameplayDecisionRoot::from_bytes(native.source_digest());
            let at=SemanticTimeMicros::from_micros(1_250_000);
            let facts=ApplicationFacts{now:at.get(),base_speed:0,mana_shield_capacity:0,
                target_reentry_protected:false,source_reentry_protected:false,target_is_player:false,
                decision_root:&root,occurrence:DecisionOccurrenceId::from_bytes([0x63;16])};
            let plan=scene.runtime.prepare_actor_condition(actor,None,ActorConditionTransition::Apply{
                definition:&marker,source:ConditionSource{actor,session:None,kind:ConditionSourceKind::SelfUse},immunities:&[],facts},at).map_err(|e|format!("{e:?}"))?;
            scene.runtime.commit_actor_condition(&plan,at).map_err(|e|format!("{e:?}"))?;
            // No reserved native think ran on this owner turn (it may have been
            // deferred by the real bounded think budget). A deadline is not one.
            let callbacks=super::super::NativeCallbacks::default();
            tokio::runtime::Builder::new_current_thread().build().map_err(|e|e.to_string())?
                .block_on(callbacks.expire_native_conditions(&mut scene.runtime,&scene.room,native,&[],6_250_000))
                .map_err(|()|"current native housekeeping".to_owned())?;
            let store=scene.runtime.actor_conditions(actor,None).map_err(|e|format!("{e:?}"))?;
            assert!(store.get(oteryn_gameplay_domain::domain::condition::ConflictKey::SpellRegeneration(88888)).is_some(),
                "fractional deadline cannot remove the stored marker before that actor's actual postthink phase");
            Ok(())
        }).map_err(|e|e.to_string())?.join().map_err(|_|"fractional marker test thread".to_owned())?
    }
}
