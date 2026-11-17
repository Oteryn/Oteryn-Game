//! Read-only display projection from the current runtime owner and durable party members.
use super::super::actor_spell::ChannelSpellStates;
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) fn project(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    observer: ExactActorRef,
    source: &world_party::ingress::PartyIngressView,
) -> BTreeMap<[u8; 16], Vec<PartyListEntity>> {
    let mut result = BTreeMap::new();
    let Ok(own) = runtime.read_actor_position(observer) else {
        return result;
    };
    let own = own.position();
    let Ok(census) = runtime.positioned_actor_census() else {
        return result;
    };
    let creatures = runtime.visible_entities().creatures;
    for (actor, position, session) in census {
        let Some(session) = session else {
            continue;
        };
        let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
            continue;
        };
        let id = *lease.character_id().as_bytes();
        let Some((_, name)) = source.members.iter().find(|(member, _)| *member == id) else {
            continue;
        };
        let Some(state) = states.get(runtime, actor, session) else {
            continue;
        };
        let vitals = state.vitals();
        let distance = distance_from(own, position.position());
        let mut rows = vec![PartyListEntity {
            entity: EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            },
            name: name.clone(),
            vocation: vocation(state.character_facts().vocation),
            summon: false,
            distance,
            health_percent: (distance <= 30)
                .then(|| percent(u64::from(vitals.health), u64::from(vitals.max_health))),
            mana_percent: (distance <= 30)
                .then(|| percent(u64::from(vitals.mana), u64::from(vitals.max_mana))),
            skull: None,
            portrait: None,
            portrait_colours: [[0; 3]; 4],
        }];
        for creature in &creatures {
            let Ok(snapshot) = runtime.companion_snapshot(creature.actor) else {
                continue;
            };
            if !snapshot
                .state
                .master
                .is_some_and(|master| master.actor == actor && master.session == session)
                && runtime.native_summon_master(creature.actor) != Some(actor)
            {
                continue;
            }
            // Tibia's player summon limit is two; never expand the wire from a runtime census.
            if rows.len() >= 3 {
                break;
            }
            rows.push(PartyListEntity {
                entity: EntityRef {
                    identity: creature.actor.placement_identity(),
                    generation: creature.generation,
                },
                name: snapshot.state.policy.display_name.clone(),
                vocation: 0,
                summon: true,
                distance,
                health_percent: (distance <= 30).then(|| {
                    percent(
                        snapshot.health.max(0) as u64,
                        snapshot.maximum_health.max(0) as u64,
                    )
                }),
                mana_percent: None,
                skull: None,
                portrait: None,
                portrait_colours: [[0; 3]; 4],
            });
        }
        result.insert(id, rows);
    }
    result
}
fn distance_from(own: MovementLocalPosition, target: MovementLocalPosition) -> u32 {
    // Canary Party::updatePlayerStatus compares X/Y only, including across floors.
    own.x.abs_diff(target.x).max(own.y.abs_diff(target.y))
}
fn percent(current: u64, maximum: u64) -> u8 {
    if maximum == 0 {
        0
    } else {
        current.saturating_mul(100).div_ceil(maximum).min(100) as u8
    }
}
fn vocation(vocation: oteryn_gameplay_spell::spell::Vocation) -> u8 {
    use oteryn_gameplay_spell::spell::Vocation::*;
    match vocation {
        None => 0,
        Knight | EliteKnight => 1,
        Paladin | RoyalPaladin => 2,
        Druid | ElderDruid => 3,
        Sorcerer | MasterSorcerer => 4,
        Monk | ExaltedMonk => 5,
    }
}
