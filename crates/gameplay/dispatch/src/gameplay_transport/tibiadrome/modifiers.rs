//! A source-qualified context owns modifier policy; the legacy hall is never its proof.
use super::*;
mod actions;
mod hazards;
use actions::apply;
#[cfg(test)]
mod test_context;
use hazards::publish_hazard;
pub(in crate::gameplay_transport) use hazards::snapshot;
use oteryn_gameplay_entities::runtime_actor_carrier::tibiadrome_modifiers::{
    DromeModifier, DromeModifierEffect as Effect, DromeModifierState, DromeParticipant,
};
#[cfg(test)]
pub(in crate::gameplay_transport) use test_context::select as select_test_modifier;
pub(super) struct ModifierOwner {
    state: DromeModifierState,
    pending: Vec<PendingEffect>,
    ordinal: u64,
    melee_ordinal: u64,
    admission_size: Option<usize>,
    hazards: Vec<wire::DromeHazard>,
}
struct PendingEffect {
    effect: Effect,
    death_source: Option<ExactActorRef>,
}
impl PendingEffect {
    fn ordinary(effect: Effect) -> Self {
        Self {
            effect,
            death_source: None,
        }
    }
}
impl ModifierOwner {
    pub(super) fn begin_wave(&mut self, wave: u32, now: u64, players: usize) -> bool {
        let admission = *self.admission_size.get_or_insert(players);
        self.pending.clear();
        self.hazards.clear();
        self.state.begin_wave(wave, now, admission).is_ok()
    }
    pub(super) fn settled(&self) -> bool {
        self.pending.is_empty()
    }
    pub(super) fn selected(&self) -> [DromeModifier; 2] {
        self.state.selected()
    }
}
/// No donor/current native Drome pool and room have yet passed artifact admission.
/// A missing source context must refuse public entry, never substitute Svargrond.
pub(super) fn qualified_context(
    _owner: &ComposedFreshAdmission<'_, '_, '_>,
    _runtime: &ChannelRuntimeV1,
    _run: [u8; 16],
    _players: usize,
    _center: MovementLocalPosition,
) -> Option<ModifierOwner> {
    #[cfg(test)]
    {
        test_context::context(_owner, _runtime, _run, _players, _center)
    }
    #[cfg(not(test))]
    {
        None
    }
}

pub(super) fn tick(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    battle: &mut Battle,
    players: &[(ExactActorRef, GameSessionId)],
) {
    let Some(modifiers) = &mut battle.modifiers else {
        return;
    };
    let participants = participants(runtime, players);
    let now = owner.owner_now();
    let all_dead = !battle.opponents.is_empty()
        && battle.opponents.iter().all(|target| {
            runtime
                .borrow_combat_death()
                .projected_death(*target)
                .is_ok()
        });
    if all_dead {
        // Death callbacks settle before the clear; future wave timers do not outlive the wave.
        modifiers.pending.retain(|effect| {
            matches!(
                effect.effect,
                Effect::CurrentHealthDamage {
                    percent: 25 | 40,
                    ..
                }
            )
        });
    } else {
        modifiers.pending.extend(
            modifiers
                .state
                .tick(battle.wave, now.get(), &participants, |at| {
                    walkable(owner, runtime, at) && runtime.native_summon_cell_free(at)
                })
                .into_iter()
                .map(PendingEffect::ordinary),
        );
    }
    let mut states = owner.states.spell_states.try_lock().ok();
    modifiers
        .hazards
        .retain(|hazard| hazard.expires_at_micros > now.get());
    let mut retained = Vec::new();
    for pending in std::mem::take(&mut modifiers.pending) {
        match &pending.effect {
            Effect::Mark {
                kind,
                positions,
                resolves_at,
            } => {
                if publish_hazard(owner, runtime, modifiers, *kind, positions, *resolves_at) {
                    continue;
                }
                retained.push(pending);
                continue;
            }
            Effect::Seed {
                kind,
                position,
                resolves_at,
            } => {
                if publish_hazard(owner, runtime, modifiers, *kind, &[*position], *resolves_at) {
                    continue;
                }
                retained.push(pending);
                continue;
            }
            _ => {}
        }
        let applied = apply(
            owner,
            runtime,
            states.as_deref_mut(),
            Recipients {
                opponents: &battle.opponents,
                players,
                death_source: pending.death_source,
            },
            modifiers.ordinal,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            pending.effect.clone(),
        );
        if applied {
            modifiers.ordinal = modifiers.ordinal.saturating_add(1);
        } else {
            retained.push(pending);
        }
    }
    modifiers.pending = retained;
}
pub(super) fn deaths(
    runtime: &mut ChannelRuntimeV1,
    battle: &mut Battle,
    players: &[(ExactActorRef, GameSessionId)],
) {
    let Some(modifiers) = &mut battle.modifiers else {
        return;
    };
    let participants = participants(runtime, players);
    for target in &battle.opponents {
        if runtime
            .borrow_combat_death()
            .projected_death(*target)
            .is_err()
        {
            continue;
        }
        if battle.modifier_deaths.contains(target) {
            continue;
        }
        battle.modifier_deaths.push(*target);
        let Ok(at) = runtime.read_actor_position(*target) else {
            continue;
        };
        let hitter = runtime.tibiadrome_last_hitter(*target).and_then(|actor| {
            runtime
                .read_actor_position(actor)
                .ok()
                .map(|position| DromeParticipant {
                    actor,
                    position: position.position(),
                })
        });
        modifiers.pending.extend(
            modifiers
                .state
                .death_at(at.position(), hitter, &participants)
                .into_iter()
                .map(|effect| PendingEffect {
                    effect,
                    death_source: Some(*target),
                }),
        );
    }
}
fn participants(
    runtime: &ChannelRuntimeV1,
    players: &[(ExactActorRef, GameSessionId)],
) -> Vec<DromeParticipant> {
    let mut participants: Vec<_> = players
        .iter()
        .filter_map(|(actor, session)| {
            runtime.player_control_facts(*actor, *session).ok()?;
            Some(DromeParticipant {
                actor: *actor,
                position: runtime.read_actor_position(*actor).ok()?.position(),
            })
        })
        .collect();
    participants.sort_by_key(|player| player.actor.placement_identity());
    participants
}
struct Recipients<'a> {
    opponents: &'a [ExactActorRef],
    players: &'a [(ExactActorRef, GameSessionId)],
    death_source: Option<ExactActorRef>,
}

pub(in crate::gameplay_transport) fn melee(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    source: ExactActorRef,
) {
    // This callback runs only after the existing native melee owner admitted the attack.
    let _ = runtime.drome_role_attack_started(source, owner.owner_now().get());
    let Ok(live) = owner.states.tibiadrome.live.lock() else {
        return;
    };
    let mut visited = std::collections::HashSet::new();
    for current in live.values() {
        if !visited.insert(current.run) {
            continue;
        }
        let Ok(mut battle) = current.battle.lock() else {
            continue;
        };
        if battle.ended || battle.preparing || !battle.opponents.contains(&source) {
            continue;
        }
        let players: Vec<_> = live
            .iter()
            .filter(|(_, member)| !member.ended && member.run == current.run)
            .map(|(session, member)| (member.actor, *session))
            .collect();
        let participants = participants(runtime, &players);
        let Ok(at) = runtime.read_actor_position(source) else {
            continue;
        };
        let Some(modifiers) = &mut battle.modifiers else {
            continue;
        };
        let Some(draw) = modifiers.state.melee_draw(modifiers.melee_ordinal) else {
            continue;
        };
        modifiers.melee_ordinal = modifiers.melee_ordinal.saturating_add(1);
        modifiers.pending.extend(
            modifiers
                .state
                .melee(source, at.position(), draw, &participants, |at| {
                    walkable(owner, runtime, at) && runtime.native_summon_cell_free(at)
                })
                .into_iter()
                .map(PendingEffect::ordinary),
        );
    }
}
