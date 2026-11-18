//! Ferumbras actors adopted from the exact committed native room turn.
use super::*;
use crate::map::view::{EntrySource, MapFacts, compose_tile};
use oteryn_gameplay_encounters::bosses::phase_controller::ferumbras::{
    CRYSTALS, CrystalCharge, Crystals,
};

const VOID: &str = "oteryn:creature.void";
const VOID_POSITION: MovementLocalPosition = MovementLocalPosition {
    x: 33391,
    y: 31461,
    floor: 14,
};
const VORTEX: u16 = 20121;
const VORTEX_LIFETIME_US: u64 = 10_000_000;
const DESTABILIZED: &str = "oteryn:creature.destabilized_ferumbras";
const MORTAL: &str = "oteryn:creature.ferumbras_mortal_shell";
const FRAGMENT: &str = "oteryn:creature.rift_fragment";
const EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 33319,
    y: 32318,
    floor: 13,
};
const SUMMON_INTERVAL_US: u64 = 15_000_000;
const FIGHT_TIMEOUT_US: u64 = 1_200_000_000;

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
struct Vortex {
    position: crate::map::overlay::TilePos,
    key: u64,
    due: u64,
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct Fight {
    opening: ExactActorRef,
    cells: crate::content::NativeEntryMovementCells,
    crystals: Crystals,
    splinters: Vec<ExactActorRef>,
    summons: Vec<ExactActorRef>,
    void: Option<ExactActorRef>,
    invader: Option<ExactActorRef>,
    next_summon: u64,
    exit_due: u64,
    vortices: Vec<Vortex>,
    essences: Vec<ExactActorRef>,
    absorbed: u8,
    destabilized: Option<ExactActorRef>,
    mortal: Option<ExactActorRef>,
    last_tick: u64,
}

impl Fight {
    pub(in crate::gameplay_transport) fn bind(
        runtime: &ChannelRuntimeV1,
        actors: &[ExactActorRef],
        cells: crate::content::NativeEntryMovementCells,
        now: u64,
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
            summons: actors.to_vec(),
            void: None,
            invader: None,
            next_summon: now.saturating_add(SUMMON_INTERVAL_US),
            exit_due: now.saturating_add(FIGHT_TIMEOUT_US),
            vortices: Vec::new(),
            essences: Vec::new(),
            absorbed: 0,
            destabilized: None,
            mortal: None,
            last_tick: now,
        })
    }

    pub(super) fn note_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        now: Option<u64>,
    ) {
        let Ok(snapshot) = runtime.companion_snapshot_including_dead(actor) else {
            return;
        };
        let Ok(position) = runtime.read_actor_position(actor) else {
            return;
        };
        let at = position.position();
        if let Some(index) = self.splinters.iter().position(|&slot| slot == actor) {
            if let Some(essence) = self.spawn(runtime, ESSENCE, at) {
                self.essences.push(essence);
                self.splinters.swap_remove(index);
            }
        } else if self.destabilized == Some(actor) {
            self.mortal = self.spawn(runtime, MORTAL, CENTER);
        } else if self.mortal == Some(actor) {
            self.exit_due = self
                .exit_due
                .min(now.unwrap_or(self.last_tick).saturating_add(60_000_000));
        } else if snapshot.state.policy.definition_key == INVADER && self.invader == Some(actor) {
            self.invader = self.respawn_invader(runtime, actor);
            if runtime.contains_live_creature(self.opening) {
                self.absorb(runtime, at);
            }
            self.create_vortex(at, now.unwrap_or(self.last_tick));
        }
    }

    fn create_vortex(&mut self, at: MovementLocalPosition, now: u64) -> Option<()> {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(bundle) =
            self.cells.index()
        else {
            return None;
        };
        let map = bundle.map();
        let position = source_tile(at)?;
        let existing = compose_tile(&map.overlay, &map.facts, position).ok()??;
        let newest = existing
            .entries
            .iter()
            .filter_map(|entry| match entry.source {
                EntrySource::Added(id) if entry.facts.appearance_id == VORTEX => Some(id.get()),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        if !map.facts.create_movement_item(position, VORTEX, 0, || true) {
            return None;
        }
        let visible = compose_tile(&map.overlay, &map.facts, position).ok()??;
        let key = visible
            .entries
            .iter()
            .find_map(|entry| match entry.source {
                EntrySource::Added(id)
                    if id.get() > newest && entry.facts.appearance_id == VORTEX =>
                {
                    Some(id.get())
                }
                _ => None,
            })?;
        self.vortices.push(Vortex {
            position,
            key,
            due: now.saturating_add(VORTEX_LIFETIME_US),
        });
        Some(())
    }

    fn consume_essences(&mut self, runtime: &mut ChannelRuntimeV1, now: u64) {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(bundle) =
            self.cells.index()
        else {
            return;
        };
        let map = bundle.map();
        self.vortices.retain(|vortex| {
            if now < vortex.due {
                return true;
            }
            map.facts
                .remove_movement_item(vortex.position, vortex.key, VORTEX);
            false
        });
        let mut seventh = None;
        self.essences.retain(|&actor| {
            let Ok(at) = runtime.read_actor_position(actor) else {
                return false;
            };
            let Some(position) = source_tile(at.position()) else {
                return true;
            };
            if !self
                .vortices
                .iter()
                .any(|vortex| vortex.position == position)
            {
                return true;
            }
            if runtime.remove_encounter_actor(actor).is_err() {
                return true;
            }
            map.facts.emit_movement_effect(position, 3);
            self.absorbed += 1;
            if self.absorbed == 7 {
                seventh = Some(actor);
            }
            false
        });
        if self.destabilized.is_none()
            && self.absorbed >= 7
            && let Some(boss) = self.spawn(runtime, DESTABILIZED, CENTER)
        {
            self.destabilized = Some(boss);
            self.spawn_fragments(runtime, seventh.unwrap_or(boss));
        }
    }

    fn spawn_fragments(&mut self, runtime: &mut ChannelRuntimeV1, cause: ExactActorRef) {
        for index in 0..15 {
            let Some(x) = draw(
                runtime,
                cause,
                "encounter.ferumbras.fragments",
                index * 2,
                33381,
                33403,
            ) else {
                continue;
            };
            let Some(y) = draw(
                runtime,
                cause,
                "encounter.ferumbras.fragments",
                index * 2 + 1,
                31462,
                31483,
            ) else {
                continue;
            };
            self.spawn(runtime, FRAGMENT, MovementLocalPosition { x, y, ..CENTER });
        }
    }

    fn respawn_invader(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        dead: ExactActorRef,
    ) -> Option<ExactActorRef> {
        let offset = |axis| draw(runtime, dead, "encounter.ferumbras.invader", axis, -10, 10);
        let at = MovementLocalPosition {
            x: CENTER.x + offset(0)?,
            y: CENTER.y + offset(1)?,
            ..CENTER
        };
        self.spawn(runtime, INVADER, at)
    }

    fn spawn(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        creature: &str,
        at: MovementLocalPosition,
    ) -> Option<ExactActorRef> {
        if !placement(&self.cells, runtime, at) {
            return None;
        }
        let hp = runtime.companion_policy(creature).ok()?.maximum_health;
        let created = runtime
            .commit_quest_creature_spawn(&[(creature, hp, at)])
            .ok()?;
        let actor = *created.first()?;
        self.summons
            .retain(|&summon| runtime.contains_live_creature(summon));
        self.summons.push(actor);
        Some(actor)
    }

    pub(super) fn tick(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        now: u64,
    ) -> Result<bool, CarrierError> {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(bundle) =
            self.cells.index()
        else {
            return Err(CarrierError::ContentPinWorldMismatch);
        };
        self.last_tick = now;
        if now >= self.exit_due {
            let map = bundle.map();
            let closed = crate::map::floor::BundleDoors(&map.facts);
            let mut moves = Vec::new();
            let mut proofs = Vec::new();
            for (actor, position, session) in runtime.positioned_melee_census()? {
                if let Some(session) = session
                    && in_room(position.position())
                {
                    let proof = crate::map::floor::prepare_quest_encounter_ejection(
                        runtime,
                        &self.cells,
                        actor,
                        session,
                        position,
                        EXIT,
                        &closed,
                    )
                    .map_err(|_| CarrierError::PlanConflict)?;
                    proofs.push(proof);
                    moves.push((actor, session, position.position()));
                }
            }
            let landings = if proofs.is_empty() {
                Vec::new()
            } else {
                runtime.commit_quest_relocation(proofs, || true)?
            };
            for ((actor, session, from), landing) in moves.into_iter().zip(landings) {
                states.stage_special_movement_step(
                    runtime,
                    map,
                    actor,
                    session,
                    now,
                    Some(from),
                    landing.position(),
                    None,
                );
            }
            for vortex in self.vortices.drain(..) {
                map.facts
                    .remove_movement_item(vortex.position, vortex.key, VORTEX);
            }
            for actor in self.summons.iter().chain(&self.splinters).copied() {
                if runtime.contains_live_creature(actor) {
                    runtime.remove_encounter_actor(actor)?;
                }
            }
            return Ok(true);
        }
        if now >= self.next_summon
            && self
                .invader
                .is_none_or(|actor| !runtime.contains_live_creature(actor))
        {
            if self
                .void
                .is_none_or(|actor| !runtime.contains_live_creature(actor))
            {
                self.void = self.spawn(runtime, VOID, VOID_POSITION);
            }
            if self.void.is_some() {
                self.invader = self.spawn(runtime, INVADER, VOID_POSITION);
            }
            self.next_summon = now.saturating_add(SUMMON_INTERVAL_US);
        }
        self.consume_essences(runtime, now);
        Ok(false)
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

fn in_room(at: MovementLocalPosition) -> bool {
    at.floor == 14 && (33379..=33405).contains(&at.x) && (31460..=31485).contains(&at.y)
}

fn source_tile(at: MovementLocalPosition) -> Option<crate::map::overlay::TilePos> {
    Some(crate::map::overlay::TilePos {
        x: u16::try_from(at.x).ok()?,
        y: u16::try_from(at.y).ok()?,
        floor: i8::try_from(at.floor).ok()?.checked_neg()?,
    })
}

fn draw(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    domain: &str,
    index: u32,
    low: i64,
    high: i64,
) -> Option<i32> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(actor.placement_identity());
    let occurrence = DecisionOccurrenceId::from_bytes(digest[..16].try_into().ok()?);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let sample = deterministic_decision_u64(&root, occurrence, domain, index.into()).ok()?;
    i32::try_from(crate::spell::uniform_draw(sample, low, high)).ok()
}
