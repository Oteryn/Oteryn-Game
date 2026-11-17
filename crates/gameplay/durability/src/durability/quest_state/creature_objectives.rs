//! Owner-issued death evidence for quest callbacks with personal contribution rules.
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::creature_objectives::CreditRule;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveDeath {
    death: crate::foundation::CreatureDeathOccurrenceKey,
    character: CharacterId,
    session: GameSessionId,
    lease: u64,
    credit: CreditRule,
    creature: String,
    position: [i32; 3],
}
impl ObjectiveDeath {
    /// Construct only while the native lethal owner can prove this exact recipient.
    pub fn capture(
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        actor: ExactActorRef,
        session: GameSessionId,
        credit: CreditRule,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        let character = lease.character_id();
        let generation = lease.generation();
        let dealt_damage = if credit == CreditRule::DamagingPlayers {
            runtime
                .source_player_damage_shares(target)
                .ok()?
                .iter()
                .any(|share| {
                    share.actor == actor && share.character == character && share.total > 0
                })
        } else {
            false
        };
        let owner = runtime.borrow_combat_death();
        let eligible = match credit {
            CreditRule::MostDamageParty => false,
            CreditRule::MostDamage => owner
                .top_damage_contributor(target)
                .ok()?
                .present()
                .is_some_and(|principal| {
                    principal.actor == actor
                        && principal.session == session
                        && principal.character == character
                        && principal.lease_generation == generation
                }),
            CreditRule::LastHit => {
                owner
                    .lethal_player_source(target)
                    .ok()?
                    .is_some_and(|(id, lease, observed)| {
                        id == character && lease == generation && observed == session
                    })
            }
            CreditRule::DamagingPlayers => dealt_damage,
        };
        if !eligible {
            return None;
        }
        let creature =
            String::from_utf8(owner.creature_target_identity(target).ok()?.to_vec()).ok()?;
        let (death, position) = owner.projected_death(target).ok()?;
        Some(Self {
            death,
            character: CharacterId::from_bytes(*character.as_bytes()).ok()?,
            session,
            lease: generation,
            credit,
            creature,
            position: [position.x, position.y, i32::from(position.floor)],
        })
    }
    pub fn occurrence(
        &self,
    ) -> Result<super::super::character_progression::ExperienceRewardOccurrence> {
        let hash = Sha256::new()
            .chain_update(b"quest-creature-objective-occurrence-v1")
            .chain_update(self.death.native_bytes())
            .chain_update(self.character.as_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        id[6] = (id[6] & 15) | 0x70;
        id[8] = (id[8] & 63) | 0x80;
        super::super::character_progression::ExperienceRewardOccurrence::from_bytes(id)
    }
    pub(super) fn matches_counter(
        &self,
        counter: oteryn_gameplay_quests::quest::native_kills::CounterRef<'_>,
    ) -> bool {
        counter.creature == self.creature
            && counter.credit == self.credit
            && counter
                .death_area
                .is_none_or(|area| area.contains(self.position))
    }
    pub(super) fn credit(&self) -> CreditRule {
        self.credit
    }
    pub(super) fn matches(&self, fence: CurrentCharacterGameplayFence) -> bool {
        self.death.runtime_scope() == fence.runtime_scope
            && self.death.scope_ownership_generation() == fence.scope_ownership_generation
            && self.character == fence.character_id
            && self.session == fence.game_session_id
            && self.lease == fence.character_lease_generation
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) {
        let hash = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"quest-creature-objective-v1")
            .chain_update(self.death.native_bytes())
            .chain_update(self.character.as_bytes())
            .chain_update(self.session.as_bytes())
            .chain_update(self.lease.to_be_bytes())
            .chain_update(self.creature.as_bytes())
            .chain_update(self.position[0].to_be_bytes())
            .chain_update(self.position[1].to_be_bytes())
            .chain_update(self.position[2].to_be_bytes())
            .chain_update([self.credit as u8])
            .finalize();
        binding[1..].copy_from_slice(&hash);
    }
}
