//! Crystal 201189f8 arbaziloth_fight.lua:125-274. Each health threshold
//! starts a delayed fire. The exact boss must cross it before the rod can clear
//! the shield, and surviving support is required at the final handoff deadline.
use super::{ChannelRuntimeV1, ExactActorRef};
use crate::foundation::{CarrierError, MovementLocalPosition};
use crate::map::{boot::BundleMap, overlay::TilePos};
use oteryn_gameplay_quests::quest::world_callbacks::ShieldHandoff;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub(super) struct Fight {
    pub(super) rule: ShieldHandoff,
    support: ExactActorRef,
    floors: Vec<i64>,
    stage: usize,
    state: Shield,
    handoff_at: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
enum Shield {
    Vulnerable,
    WaitingForFire(u64),
    Fire { at: TilePos, key: u64 },
    Cracked,
}

impl Fight {
    pub(super) fn bind(
        runtime: &mut ChannelRuntimeV1,
        boss: ExactActorRef,
        actors: &[ExactActorRef],
        rule: &ShieldHandoff,
        maximum_health: i64,
        map: &BundleMap,
    ) -> Result<Self, CarrierError> {
        let support = actors
            .iter()
            .copied()
            .find(|actor| runtime.matches_live_creature_identity(*actor, rule.support.as_bytes()))
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        runtime.companion_policy(&rule.terminal)?;
        if map
            .facts
            .movement_item_facts(rule.fire_appearance)
            .is_none()
        {
            return Err(CarrierError::PlanConflict);
        }
        let floors: Vec<_> = rule
            .thresholds_percent
            .iter()
            .map(|percent| {
                let percent = i64::from(*percent);
                (maximum_health / 100) * percent + (maximum_health % 100) * percent / 100
            })
            .collect();
        if floors.last().is_none_or(|floor| *floor <= 0)
            || floors.windows(2).any(|pair| pair[0] <= pair[1])
        {
            return Err(CarrierError::PlanConflict);
        }
        runtime.set_encounter_shield_threshold(boss, floors.first().copied())?;
        Ok(Self {
            rule: rule.clone(),
            support,
            floors,
            stage: 0,
            state: Shield::Vulnerable,
            handoff_at: None,
        })
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        boss: ExactActorRef,
        map: &BundleMap,
        now: u64,
        placeable: impl Fn(MovementLocalPosition, &ChannelRuntimeV1) -> bool,
    ) -> Option<ExactActorRef> {
        if !runtime.contains_live_creature(boss) {
            if let Shield::Fire { at, key } = self.state
                && map
                    .facts
                    .remove_movement_item(at, key, self.rule.fire_appearance)
            {
                self.state = Shield::Vulnerable;
            }
            self.handoff_at = None;
            return None;
        }
        if self.handoff_at.is_some_and(|due| now >= due) {
            if !runtime.contains_live_creature(self.support) {
                self.handoff_at = None;
                return None;
            }
            return runtime
                .commit_encounter_terminal_handoff(boss, self.support, &self.rule.terminal)
                .ok();
        }
        let position = runtime.read_actor_position(boss).ok()?.position();
        match self.state {
            Shield::Vulnerable if runtime.encounter_shield_protected(boss) => {
                self.state =
                    Shield::WaitingForFire(now.saturating_add(u64::from(self.rule.fire_delay_ms)));
            }
            Shield::WaitingForFire(due) if now >= due => {
                let width = u32::from(self.rule.fire_max.x - self.rule.fire_min.x) + 1;
                let height = u32::from(self.rule.fire_max.y - self.rule.fire_min.y) + 1;
                let count = width * height;
                let offset = draw(b"encounter.shield.fire", boss, now) % count;
                for index in (0..count).map(|index| (index + offset) % count) {
                    let at = TilePos {
                        x: self.rule.fire_min.x + u16::try_from(index % width).ok()?,
                        y: self.rule.fire_min.y + u16::try_from(index / width).ok()?,
                        floor: self.rule.fire_min.floor,
                    };
                    let fire = local(at);
                    if fire != position
                        && placeable(fire, runtime)
                        && let Some(key) = map.facts.create_owned_movement_item(
                            at,
                            self.rule.fire_appearance,
                            0,
                            || true,
                        )
                    {
                        self.state = Shield::Fire { at, key };
                        break;
                    }
                }
            }
            Shield::Fire { at, key } if local(at) == position => {
                if map
                    .facts
                    .remove_movement_item(at, key, self.rule.fire_appearance)
                {
                    self.state = Shield::Cracked;
                }
            }
            _ => {}
        }
        None
    }

    pub(super) fn use_tool(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        boss: ExactActorRef,
        command: u64,
        now: u64,
    ) -> Result<(), CarrierError> {
        if !matches!(self.state, Shield::Cracked)
            || draw(b"encounter.shield.rod", boss, command) % 100
                < u32::from(self.rule.dodge_percent)
        {
            return Ok(());
        }
        let next = self.stage + 1;
        runtime.set_encounter_shield_threshold(boss, self.floors.get(next).copied())?;
        self.stage = next;
        self.state = Shield::Vulnerable;
        if next == self.floors.len() && runtime.contains_live_creature(self.support) {
            self.handoff_at = Some(now.saturating_add(u64::from(self.rule.handoff_delay_ms)));
        }
        Ok(())
    }
}

fn local(at: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

fn draw(label: &[u8], boss: ExactActorRef, input: u64) -> u32 {
    let digest = Sha256::new()
        .chain_update(label)
        .chain_update(boss.placement_identity())
        .chain_update(input.to_be_bytes())
        .finalize();
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}
