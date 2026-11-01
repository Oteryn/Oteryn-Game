//! The Channel owner's ephemeral Echo portals, raid timers and physical creature variants.
//! Durable first-species rewards and corpse Items remain in their existing owners.
use super::*;
use oteryn_gameplay_domain::domain::echo_warden::{
    AURA_INTERVAL_MS, EchoAuraFacts, EchoDeathEffect, EchoDeathFacts, EchoPortalTiming,
    EchoRaidPlan, EchoVariant, EchoWardenFacts, PORTAL_TTL_MS, SPAWN_STEP_MS, aura_can_enlist,
    can_apply_warden, death_effect, portal_timing,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchoPortal {
    pub occurrence: [u8; 16],
    pub creature: String,
    pub position: MovementLocalPosition,
    pub timing: EchoPortalTiming,
    active: bool,
}

#[cfg(test)]
#[path = "echo_warden/tests.rs"]
mod tests;

fn source_name(registration_name: &str) -> &str {
    oteryn_gameplay_domain::domain::soulpit::source_monster(registration_name)
        .map_or(registration_name, |monster| monster.name)
}

#[derive(Debug)]
struct EchoRaid {
    creature: String,
    position: MovementLocalPosition,
    plan: EchoRaidPlan,
    next_spawn: u8,
    next_due_ms: u64,
}

#[derive(Debug, Default)]
pub(super) struct EchoRuntime {
    portals: Vec<EchoPortal>,
    raids: Vec<EchoRaid>,
    wardens: Vec<(ExactActorRef, u64)>,
    spawned: Vec<ExactActorRef>,
}

/// A captured physical death plus its source facts and owner draw. The typed death key can
/// only be issued by the actual lethal owner; callers cannot synthesize a creature death.
#[derive(Clone, Copy)]
pub struct EchoDeathInput {
    pub death: CreatureDeathOccurrenceKey,
    pub occurrence: [u8; 16],
    pub bestiary_stars: Option<u8>,
    pub has_boss_race: bool,
    pub draw: u32,
    pub now_ms: u64,
}

impl ChannelRuntimeV1 {
    /// The source portal retains Monster:getName(), then createMonster resolves that
    /// exact registration. Renamed Horse/Nomad variants therefore start the base raid.
    /// Resolve only already compiled canonical policies; never invent a native identity
    /// or choose an ambiguous display-name match by catalogue order.
    fn echo_raid_species(
        &self,
        policy: &super::runtime_actor_companion::CompiledCreaturePolicy,
    ) -> Result<String, CarrierError> {
        let name = source_name(&policy.display_name);
        if name == policy.display_name {
            return Ok(policy.definition_key.clone());
        }
        let table = self
            .companion_policies
            .as_ref()
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        let mut candidates = table.policies().iter().filter(|candidate| {
            candidate.definition_key.starts_with("oteryn:creature.")
                && candidate.display_name.eq_ignore_ascii_case(name)
        });
        let candidate = candidates
            .next()
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if candidates.next().is_some() {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        Ok(candidate.definition_key.clone())
    }

    pub fn echo_variant(&self, actor: ExactActorRef) -> Option<EchoVariant> {
        self.owner_fence().ok()?;
        let index = self
            .carrier
            .validate_summon_actor(&self.continuity, actor)
            .ok()?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied { spell_combat, .. } => spell_combat.echo,
            _ => None,
        }
    }

    /// Both native melee and native spell consumers use the same exact-actor variant.
    pub fn echo_attack_percent(&self, actor: ExactActorRef) -> u16 {
        self.echo_variant(actor)
            .map_or(100, EchoVariant::attack_percent)
    }

    /// One death schedules at most one portal, including when its loot settlement retries.
    /// Tagging a converted natural minion prevents it from opening another raid too.
    pub fn record_echo_death(
        &mut self,
        input: EchoDeathInput,
    ) -> Result<EchoDeathEffect, CarrierError> {
        self.owner_fence()?;
        if !self.encounter_death_is_current(input.death) {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let actor = ExactActorRef(input.death.actor);
        let snapshot = self.companion_snapshot_including_dead(actor)?;
        if snapshot.health > 0 {
            return Err(CarrierError::CreatureNotActionable);
        }
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if spell_combat.echo == Some(EchoVariant::Warden) {
            return Ok(EchoDeathEffect::FirstSpeciesReward);
        }
        if spell_combat.echo_death_recorded {
            return Ok(EchoDeathEffect::None);
        }
        let facts = EchoDeathFacts {
            is_warden: false,
            is_raid_spawned: spell_combat.echo.is_some(),
            is_summon: snapshot.state.master.is_some(),
            is_reward_boss: snapshot.state.policy.reward_boss == Some(true),
            has_boss_race: input.has_boss_race,
            forge_stacks: snapshot
                .forge
                .map_or(0, |kind| u16::from(kind.reward_stacks())),
            // Missing source facts never make a creature eligible.
            bestiary_stars: input.bestiary_stars.unwrap_or(u8::MAX),
        };
        let effect = death_effect(facts, input.draw).map_err(|_| CarrierError::InvalidDamage)?;
        if effect == EchoDeathEffect::SchedulePortal {
            let timing = portal_timing(input.now_ms).ok_or(CarrierError::DamageOverflow)?;
            let creature = self.echo_raid_species(&snapshot.state.policy)?;
            self.echo_wardens.portals.push(EchoPortal {
                occurrence: input.occurrence,
                creature,
                position: snapshot.position,
                timing,
                active: false,
            });
        }
        if let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] {
            spell_combat.echo_death_recorded = true;
        }
        Ok(effect)
    }

    pub fn echo_portals(&self, now_ms: u64) -> impl Iterator<Item = &EchoPortal> {
        self.echo_wardens
            .portals
            .iter()
            .filter(move |portal| portal.active && portal.timing.is_visible_at(now_ms))
    }

    pub fn next_echo_due_ms(&self) -> Option<u64> {
        self.echo_wardens
            .portals
            .iter()
            .map(|portal| {
                if portal.active {
                    portal.timing.expires_at_ms
                } else {
                    portal.timing.appears_at_ms
                }
            })
            .chain(self.echo_wardens.raids.iter().map(|raid| raid.next_due_ms))
            .chain(self.echo_wardens.wardens.iter().map(|(_, due)| *due))
            .min()
    }

    /// Called only after a player's committed step. Removal and raid creation share this
    /// runtime work item, so a repeated command or a second player cannot consume it twice.
    pub fn activate_echo_portal(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        now_ms: u64,
        minion_draw: u8,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        self.bound_attacker_lease(actor, session)?;
        let position = self.read_actor_position(actor)?.position();
        let Some(index) = self.echo_wardens.portals.iter().position(|portal| {
            portal.active && portal.timing.is_visible_at(now_ms) && portal.position == position
        }) else {
            return Ok(false);
        };
        let plan = EchoRaidPlan::new(minion_draw).map_err(|_| CarrierError::InvalidDamage)?;
        let portal = self.echo_wardens.portals.remove(index);
        self.echo_wardens.raids.push(EchoRaid {
            creature: portal.creature,
            position,
            plan,
            next_spawn: 0,
            next_due_ms: now_ms,
        });
        Ok(true)
    }

    fn apply_echo_variant(
        &mut self,
        actor: ExactActorRef,
        variant: EchoVariant,
    ) -> Result<bool, CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        let snapshot = self.companion_snapshot(actor)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if spell_combat.echo.is_some() {
            return Ok(false);
        }
        // Wardens are fresh encounter children. The trigger already excluded boss species;
        // fresh children have no Soulpit tag, forge classification or summon master.
        if variant == EchoVariant::Warden
            && !can_apply_warden(EchoWardenFacts {
                already_warden: false,
                is_summon: snapshot.state.master.is_some(),
                is_boss: snapshot.state.policy.reward_boss == Some(true),
                forge_stacks: snapshot
                    .forge
                    .map_or(0, |kind| u16::from(kind.reward_stacks())),
                is_soulpit: false,
            })
        {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let maximum = u64::try_from(snapshot.maximum_health)
            .ok()
            .and_then(|health| variant.health(health))
            .and_then(|health| i64::try_from(health).ok())
            .ok_or(CarrierError::DamageOverflow)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        *health = maximum;
        spell_combat.maximum_health = maximum;
        spell_combat.echo = Some(variant);
        Ok(true)
    }

    fn spawn_echo_creature(
        &mut self,
        creature: &str,
        position: MovementLocalPosition,
        variant: EchoVariant,
        now_ms: u64,
    ) -> Result<ExactActorRef, CarrierError> {
        let health = self.companion_policy(creature)?.maximum_health;
        let aura_due = now_ms
            .checked_add(AURA_INTERVAL_MS)
            .ok_or(CarrierError::DamageOverflow)?;
        // Crystal creates every member with force=true at the portal tile. The carrier still
        // enforces its existing actor capacity and exact incarnation ownership.
        let actor = self.admit_encounter_actor(creature, health, position)?;
        let result = self
            .install_creature_policy(actor, creature)
            .and_then(|()| self.apply_echo_variant(actor, variant));
        if let Err(error) = result {
            self.remove_encounter_actor(actor)?;
            return Err(error);
        }
        if variant == EchoVariant::Warden {
            self.echo_wardens.wardens.push((actor, aura_due));
        }
        self.echo_wardens.spawned.push(actor);
        Ok(actor)
    }

    /// Source timers run without a nearby player. `has_ground` reads the current qualified
    /// map at creation time, matching the donor's delayed Tile/getGround check.
    /// Returns whether the volatile map overlay must be invalidated.
    pub fn drive_echo_wardens(
        &mut self,
        now_ms: u64,
        mut has_ground: impl FnMut(MovementLocalPosition) -> bool,
    ) -> Result<bool, CarrierError> {
        self.owner_fence()?;
        let mut changed = false;
        // Encounter children have no ordinary respawn cell to reclaim their slot. Keep the
        // exact dead actor while its kill is queued, then let the existing corpse projection
        // outlive the removed incarnation just as it does for ordinary respawns.
        for actor in std::mem::take(&mut self.echo_wardens.spawned) {
            match self.companion_snapshot_including_dead(actor) {
                Ok(snapshot)
                    if snapshot.health <= 0
                        && !self.carrier.corpse_projection_pins.contains(&actor) =>
                {
                    if self.remove_encounter_actor(actor).is_err() {
                        self.echo_wardens.spawned.push(actor);
                    }
                }
                Ok(_) => self.echo_wardens.spawned.push(actor),
                Err(_) => {}
            }
        }
        self.echo_wardens.portals.retain_mut(|portal| {
            if portal.active {
                if now_ms >= portal.timing.expires_at_ms {
                    changed = true;
                    return false;
                }
            } else if now_ms >= portal.timing.appears_at_ms {
                let Some(expires_at_ms) = now_ms.checked_add(PORTAL_TTL_MS) else {
                    return false;
                };
                if !has_ground(portal.position) {
                    return false;
                }
                // A delayed owner turn starts TTL when the item really appears, not at its
                // originally scheduled appearance time (echo_warden.lua:203-224).
                portal.timing = EchoPortalTiming {
                    appears_at_ms: now_ms,
                    expires_at_ms,
                };
                portal.active = true;
                changed = true;
            }
            true
        });
        let raids = std::mem::take(&mut self.echo_wardens.raids);
        for mut raid in raids {
            while raid.next_spawn < raid.plan.spawn_count() && now_ms >= raid.next_due_ms {
                if let Some(variant) = raid.plan.variant_at(raid.next_spawn)
                    && has_ground(raid.position)
                {
                    // The donor consumes a failed slot and schedules the remaining minions.
                    let _ =
                        self.spawn_echo_creature(&raid.creature, raid.position, variant, now_ms);
                }
                raid.next_spawn += 1;
                if raid.next_spawn > 1 {
                    raid.next_due_ms = now_ms
                        .checked_add(SPAWN_STEP_MS)
                        .ok_or(CarrierError::DamageOverflow)?;
                }
            }
            if raid.next_spawn < raid.plan.spawn_count() {
                self.echo_wardens.raids.push(raid);
            }
        }
        let wardens = std::mem::take(&mut self.echo_wardens.wardens);
        for (warden, due) in wardens {
            let Ok(source) = self.companion_snapshot(warden) else {
                continue;
            };
            if now_ms < due {
                self.echo_wardens.wardens.push((warden, due));
                continue;
            }
            let actors: Vec<_> = self
                .visible_entities()
                .creatures
                .iter()
                .map(|entry| entry.actor)
                .collect();
            for actor in actors {
                if actor == warden {
                    continue;
                }
                let Ok(candidate) = self.companion_snapshot(actor) else {
                    continue;
                };
                if aura_can_enlist(EchoAuraFacts {
                    same_species: source_name(&candidate.state.policy.display_name)
                        == source_name(&source.state.policy.display_name),
                    is_warden: self.echo_variant(actor) == Some(EchoVariant::Warden),
                    same_floor: candidate.position.floor == source.position.floor,
                    delta_x: u16::try_from(candidate.position.x.abs_diff(source.position.x))
                        .unwrap_or(u16::MAX),
                    delta_y: u16::try_from(candidate.position.y.abs_diff(source.position.y))
                        .unwrap_or(u16::MAX),
                }) {
                    let _ = self.apply_echo_variant(actor, EchoVariant::Minion);
                }
            }
            let due = now_ms
                .checked_add(AURA_INTERVAL_MS)
                .ok_or(CarrierError::DamageOverflow)?;
            self.echo_wardens.wardens.push((warden, due));
        }
        Ok(changed)
    }
}
