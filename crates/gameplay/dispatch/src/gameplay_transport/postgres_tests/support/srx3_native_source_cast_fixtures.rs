use super::fixture::debug;

use super::TestResult;

/// One current death: at most one detached turn per copied event, its completion,
/// and the retained uncredited corpse ACK. Never advance the source clock here.
pub(super) async fn drain_source_death(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual native death content")?;
    let profile = owner
        .runtime
        .lock()
        .await
        .companion_snapshot_including_dead(actor)
        .map_err(debug)?
        .state
        .policy;
    let definition = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Creature,
        key: profile.definition_key.clone(),
        revision: profile.definition_revision.clone(),
    };
    let event_count = native
        .native_death_events(&definition)
        .map_or(0, |events| events.events().len());
    for _ in 0..event_count + 2 {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let head = runtime.next_source_creature_death().map_err(debug)?;
        if let Some(head) = &head {
            assert_eq!(
                head.actor(),
                actor,
                "only this exact retained death may be drained"
            );
        }
        let pending = head.is_some() || states.source_death_corpse.is_some();
        drop(states);
        drop(runtime);
        if !pending {
            break;
        }
        if definition.key == "oteryn:creature.outburst" {
            diagnose_outburst_death(owner, actor, now, "before_turn").await;
        }
        owner.drain_monster_melee_at(now).await;
        if definition.key == "oteryn:creature.outburst" {
            diagnose_outburst_death(owner, actor, now, "after_turn").await;
        }
    }
    if definition.key == "oteryn:creature.outburst" {
        diagnose_outburst_death(owner, actor, now, "before_assert").await;
    }
    assert!(
        !owner
            .runtime
            .lock()
            .await
            .has_pending_source_creature_deaths(),
        "all copied source events must finish without advancing time"
    );
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .source_death_corpse
            .is_none(),
        "the retained corpse must have a committed or reconciled terminal writer ACK"
    );
    Ok(())
}

#[cfg(test)]
async fn diagnose_outburst_death(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    stage: &str,
) {
    use crate::map::view::MapFacts as _;
    let runtime = owner.runtime.lock().await;
    let Ok(states) = owner.states.spell_states.try_lock() else {
        eprintln!("SRX3_OUTBURST_DEATH_DIAGNOSTIC stage={stage} spell_states_busy=true");
        return;
    };
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay());
    let action = states.source_death_action.as_ref();
    eprintln!(
        "SRX3_OUTBURST_DEATH_DIAGNOSTIC stage={stage} actor={actor:?} now={} owner_now={} owner_current={} scope={:?} generation={} source_work={:?} head={:?} action={action:?} action_current={:?} cursor={:?} ready={} reservation={} reservation_valid={:?} corpse_pending={}",
        now.get(),
        owner.owner_now().get(),
        runtime.owner_fence().is_ok(),
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation().get(),
        native.map(
            |native| crate::gameplay_transport::source_death::next_source_work(
                &runtime, &states, native
            )
        ),
        runtime.next_source_creature_death(),
        action.map(|action| action.current_for(&runtime)),
        action.map(|action| runtime.source_creature_death_event_index(action.begin())),
        states.source_death_action_ready,
        states.source_death_action_reservation.is_some(),
        states
            .source_death_action_reservation
            .as_ref()
            .map(|reservation| runtime.validate_source_death_action_reservation(reservation)),
        states.source_death_corpse.is_some()
    );
    let vortex = (|| {
        let active = owner.active_generation.ok_or("native activation absent")?;
        let room = owner
            .effective_rooms()
            .gate_monster_ai()
            .ok_or("current combat room absent")?;
        let context = active
            .qualify_callback_source_context(
                room,
                runtime.content_pin(),
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst,
            )
            .ok_or("current Outburst source context refused")?;
        let (x, y, floor) = context
            .source_frame()
            .physical_position([32225, 31285, 14])
            .ok_or("source vortex projection refused")?;
        let tile = crate::map::overlay::TilePos {
            x: u16::try_from(x).map_err(|_| "vortex x range")?,
            y: u16::try_from(y).map_err(|_| "vortex y range")?,
            floor: i8::try_from(floor.checked_neg().ok_or("vortex floor negation")?)
                .map_err(|_| "vortex floor range")?,
        };
        let map = owner.bundle_map().ok_or("current Bundle map absent")?;
        let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
            .map_err(|_| "current vortex compose refused")?
            .ok_or("current vortex tile absent")?;
        let entries = stack
            .entries
            .iter()
            .filter_map(|entry| match entry.source {
                crate::map::view::EntrySource::Base { placement_key, .. } => Some((
                    placement_key,
                    map.facts.object_revision(placement_key),
                    entry.facts.appearance_id,
                    map.facts.action_at(placement_key),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        Ok::<_, &str>((tile, entries))
    })();
    eprintln!("SRX3_OUTBURST_DEATH_DIAGNOSTIC stage={stage} vortex={vortex:?}");
}
