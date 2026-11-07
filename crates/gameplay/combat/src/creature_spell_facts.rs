//! Current spell combat facts shared with the lower-layer chain picker.
use crate::creature_auto_attack::{AttackFacts, CurrentConditionPolicy};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp};
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;

#[derive(Clone)]
pub struct SpellCombatFacts {
    pub attack: AttackFacts,
    /// Elemental resistance only. Excludes armor/shield, already-authored base formula and primary damage draw.
    pub multiplier_ppm: u32,
    /// Summed equipment and installed imbuement protection, applied once after blocking.
    pub equipment_protection: Option<oteryn_gameplay_content::content::ReferenceRationalPercent>,
    pub immune: bool,
    pub condition_policy: Option<CurrentConditionPolicy>,
}
impl SpellCombatFacts {
    /// Apply equipment's signed absorb sum before the remaining elemental multiplier.
    pub fn mitigate_damage(&self, damage: u32) -> Option<u32> {
        if self.multiplier_ppm > 10_000_000 {
            return None;
        }
        let damage = match self.equipment_protection {
            Some(protection) => {
                u32::try_from(oteryn_gameplay_items::equipment_effects::apply_protection(
                    i64::from(damage),
                    protection,
                )?)
                .ok()?
            }
            None => damage,
        };
        u32::try_from(u64::from(damage) * u64::from(self.multiplier_ppm) / 1_000_000).ok()
    }
}

pub trait SpellWorldReader {
    /// Actual immutable current loader owner. Missing membership fails closed before
    /// RNG, cast occurrence or condition mutation; source JSON is never this owner.
    fn native_gameplay<'a>(
        &'a self,
        _runtime: &ChannelRuntimeV1,
    ) -> Option<&'a crate::content::native_gameplay::NativeGameplayState> {
        None
    }

    /// Must enumerate current committed player ActorRefs/sessions from the same locked runtime.
    /// None refuses the entire cast. NPC damage belongs to the separate creature HP owner.
    fn current_players(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>>;
    fn current_facing(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<Facing>;
    /// Current tile combat/LOS/floor-change policy, even if this tile has no player.
    fn tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        x: i32,
        y: i32,
        floor: i16,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool>;
    fn combat(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts>;
}

impl<T: SpellWorldReader + ?Sized> crate::creature_chain_attack::ChainWorldReader for T {
    fn current_players(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        SpellWorldReader::current_players(self, runtime, stamp)
    }
    fn attack_facts(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<AttackFacts> {
        self.combat(runtime, issuer, target, session, element, stamp)
            .map(|facts| facts.attack)
    }
}
