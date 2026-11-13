//! Modifier state is carried by exact live actor slots, never an external HP store.
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DromeActorModifiers {
    pub(in crate::carrier) run: [u8; 16],
    pub(super) selected: [DromeModifier; 2],
    incoming: Vec<(DromeModifier, u64)>,
    pub(super) role: Option<super::roles::DromeRoleState>,
    pub(in crate::carrier) last_hitter: Option<ExactActorRef>,
}
impl ChannelRuntimeV1 {
    pub fn bind_tibiadrome_modifiers(
        &mut self,
        actor: ExactActorRef,
        run: [u8; 16],
        selected: [DromeModifier; 2],
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !self.is_tibiadrome(actor) || selected[0] == selected[1] {
            return Err(CarrierError::InvalidDamage);
        }
        let role = self
            .companion_snapshot(actor)
            .ok()
            .and_then(|snapshot| super::roles::DromeRoleState::native(&snapshot));
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let combat = match &mut self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
                spell_combat
            }
            _ => return Err(CarrierError::NotCreature),
        };
        combat.tibiadrome_modifiers = Some(DromeActorModifiers {
            run,
            selected,
            incoming: Vec::new(),
            role,
            last_hitter: None,
        });
        Ok(())
    }
    pub(super) fn drome_actor_modifiers(
        &self,
        actor: ExactActorRef,
    ) -> Option<&DromeActorModifiers> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        match &self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } => spell_combat.tibiadrome_modifiers.as_ref(),
            Slot::CreatureOccupied {
                spell_combat,
                companion,
                ..
            } => spell_combat.tibiadrome_modifiers.as_ref().or_else(|| {
                companion
                    .as_ref()
                    .and_then(|c| c.master)
                    .and_then(|master| self.drome_actor_modifiers(master.actor))
            }),
            _ => None,
        }
    }
    pub fn tibiadrome_incoming_damage(
        &self,
        target: ExactActorRef,
        magnitude: u32,
        now: u64,
    ) -> Result<u32, CarrierError> {
        let Some(policy) = self.drome_actor_modifiers(target) else {
            return Ok(magnitude);
        };
        let percent = policy
            .incoming
            .iter()
            .filter(|(_, until)| *until > now)
            .map(|(kind, _)| match kind {
                DromeModifier::TankedUp => 10,
                DromeModifier::SownSorrow => 15,
                DromeModifier::BadRoots => 25,
                _ => 0,
            })
            .sum::<u64>()
            + 100;
        u32::try_from(u64::from(magnitude) * percent / 100)
            .map_err(|_| CarrierError::DamageOverflow)
    }
    pub fn apply_tibiadrome_incoming_modifier(
        &mut self,
        target: ExactActorRef,
        kind: DromeModifier,
        until: u64,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, target.0)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        let policy = spell_combat
            .tibiadrome_modifiers
            .as_mut()
            .ok_or(CarrierError::InvalidDamage)?;
        if !policy.selected.contains(&kind)
            || !matches!(
                kind,
                DromeModifier::TankedUp | DromeModifier::SownSorrow | DromeModifier::BadRoots
            )
        {
            return Err(CarrierError::InvalidDamage);
        }
        if let Some((_, deadline)) = policy.incoming.iter_mut().find(|(old, _)| *old == kind) {
            *deadline = (*deadline).max(until);
        } else {
            policy.incoming.push((kind, until));
        }
        Ok(())
    }
    pub fn permits_tibiadrome_health_loss(&self, target: ExactActorRef, percent: u32) -> bool {
        let modifier = match percent {
            40 => DromeModifier::GoingDownWithMe,
            25 => DromeModifier::ExplodingCorpses,
            60 => DromeModifier::TheFloorIsLava,
            _ => return false,
        };
        self.drome_actor_modifiers(target)
            .is_some_and(|policy| policy.selected.contains(&modifier))
    }
    pub fn tibiadrome_last_hitter(&self, actor: ExactActorRef) -> Option<ExactActorRef> {
        self.drome_actor_modifiers(actor)?.last_hitter
    }
    pub fn record_tibiadrome_source_hit(&mut self, source: ExactActorRef, target: ExactActorRef) {
        self.carrier
            .record_drome_last_hitter(&self.continuity, source, target);
    }
    pub fn relocate_tibiadrome_creature(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        destination: MovementLocalPosition,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if self.drome_actor_modifiers(actor).is_none() || expected.0.actor_ref != actor.0 {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.companion_snapshot(actor)?;
        self.carrier.compare_commit_position(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: destination.x,
                y: destination.y,
                floor: destination.floor,
            },
        )?;
        Ok(())
    }
    pub(in crate::carrier) fn drome_escalated(
        &self,
        actor: ExactActorRef,
        health: i64,
        maximum: i64,
    ) -> bool {
        self.drome_actor_modifiers(actor).is_some_and(|policy| {
            policy
                .selected
                .contains(&DromeModifier::ThatEscalatedQuickly)
        }) && health > 0
            && i128::from(health) * 4 < i128::from(maximum)
    }
}

impl ChannelActorCarrier {
    pub(in crate::carrier) fn record_drome_last_hitter(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        source: ExactActorRef,
        target: ExactActorRef,
    ) {
        if self.validate_current_continuity(continuity).is_err() {
            return;
        }
        let Ok(source_index) = self.validate_ref(continuity, source.0) else {
            return;
        };
        let run = (|| match &self.slots[source_index] {
            Slot::Occupied {
                game_session_id: Some(_),
                spell_combat,
                ..
            } if spell_combat.tibiadrome => spell_combat
                .tibiadrome_modifiers
                .as_ref()
                .map(|policy| policy.run),
            Slot::CreatureOccupied {
                companion: Some(state),
                spell_combat,
                ..
            } => {
                if state.policy.is_familiar {
                    let master = state.master?;
                    let index = self.validate_ref(continuity, master.actor.0).ok()?;
                    match &self.slots[index] {
                        Slot::Occupied {
                            game_session_id: Some(session),
                            spell_combat,
                            ..
                        } if *session == master.session && spell_combat.tibiadrome => spell_combat
                            .tibiadrome_modifiers
                            .as_ref()
                            .map(|policy| policy.run),
                        _ => None,
                    }
                } else {
                    spell_combat
                        .tibiadrome_modifiers
                        .as_ref()
                        .map(|policy| policy.run)
                }
            }
            _ => None,
        })();
        let Some(run) = run else {
            return;
        };
        let Ok(index) = self.validate_ref(continuity, target.0) else {
            return;
        };
        if let Slot::CreatureOccupied { spell_combat, .. } = &mut self.slots[index]
            && let Some(policy) = &mut spell_combat.tibiadrome_modifiers
            && policy.run == run
        {
            policy.last_hitter = Some(source);
        }
    }
}
