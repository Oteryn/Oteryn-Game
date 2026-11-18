//! Ferumbras actors adopted from the exact committed native room turn.
use super::*;
use crate::map::view::{EntrySource, MapFacts, compose_tile};
use oteryn_gameplay_encounters::bosses::phase_controller::ferumbras::{
    CRYSTALS, CrystalCharge, Crystals,
};

const BOSS: &str = "oteryn:creature.ascending_ferumbras";
const INVADER: &str = "oteryn:creature.rift_invader";
const SPLINTER: &str = "oteryn:creature.ferumbras_soul_splinter";
const ESSENCE: &str = "oteryn:creature.ferumbras_essence";
const CENTER: MovementLocalPosition = MovementLocalPosition {
    x: 33392,
    y: 31473,
    floor: 14,
};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct Fight {
    opening: ExactActorRef,
    cells: crate::content::NativeEntryMovementCells,
    crystals: Crystals,
    splinters: Vec<ExactActorRef>,
}

impl Fight {
    pub(in crate::gameplay_transport) fn bind(
        runtime: &ChannelRuntimeV1,
        actors: &[ExactActorRef],
        cells: crate::content::NativeEntryMovementCells,
    ) -> Result<Self, CarrierError> {
        let opening = *actors
            .iter()
            .find(|&&actor| runtime.matches_live_creature_identity(actor, BOSS.as_bytes()))
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        Ok(Self {
            opening,
            cells,
            crystals: Crystals::default(),
            splinters: Vec::new(),
        })
    }

    pub(super) fn note_death(&mut self, runtime: &mut ChannelRuntimeV1, actor: ExactActorRef) {
        let Ok(snapshot) = runtime.companion_snapshot_including_dead(actor) else {
            return;
        };
        let Ok(position) = runtime.read_actor_position(actor) else {
            return;
        };
        let at = position.position();
        if let Some(index) = self.splinters.iter().position(|&slot| slot == actor) {
            if let Ok(policy) = runtime.companion_policy(ESSENCE)
                && runtime
                    .commit_quest_creature_spawn(&[(ESSENCE, policy.maximum_health, at)])
                    .is_ok()
            {
                self.splinters.swap_remove(index);
            }
        } else if snapshot.state.policy.definition_key == INVADER
            && snapshot.state.master.is_none()
            && runtime.contains_live_creature(self.opening)
        {
            self.absorb(runtime, at);
        }
    }

    fn absorb(&mut self, runtime: &mut ChannelRuntimeV1, at: MovementLocalPosition) -> Option<()> {
        let mut next = self.crystals.clone();
        let (index, charge) = next.absorb(at)?;
        if let CrystalCharge::Lit { all_lit } = charge {
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(bundle) =
                self.cells.index()
            else {
                return None;
            };
            let map = bundle.map();
            let (x, y) = CRYSTALS[index];
            let position = crate::map::overlay::TilePos {
                x: u16::try_from(x).ok()?,
                y: u16::try_from(y).ok()?,
                floor: -14,
            };
            let tile = compose_tile(&map.overlay, &map.facts, position).ok()??;
            let key = tile.entries.iter().find_map(|entry| match entry.source {
                EntrySource::Base { placement_key, .. } if entry.facts.appearance_id == 14955 => {
                    Some(placement_key)
                }
                _ => None,
            })?;
            let revision = map.facts.object_revision(key);
            let lit = map.facts.quest_callback_replacement(key, 14961)?;
            let publish = || {
                map.facts
                    .apply_quest_callback(key, revision, lit, &[], &[], false, None)
            };
            if all_lit {
                let policy = runtime.companion_policy(SPLINTER).ok()?;
                let before = runtime.encounter_parts().0.clone();
                runtime.remove_encounter_actor(self.opening).ok()?;
                match runtime.commit_quest_creature_spawn_with_projection(
                    &[(SPLINTER, policy.maximum_health, CENTER); CRYSTALS.len()],
                    publish,
                ) {
                    Ok(splinters) => self.splinters = splinters,
                    Err(_) => {
                        *runtime.encounter_parts_mut().0 = before;
                        return None;
                    }
                }
            } else if !publish() {
                return None;
            }
        }
        self.crystals = next;
        Some(())
    }
}
