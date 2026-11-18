//! Crystal 201189f8 creaturescripts_ghulosh.lua:19–64.
use super::*;

const BASE: &str = "oteryn:creature.ghulosh";
const DEATHGAZE: &str = "oteryn:creature.ghulosh_deathgaze";
const BOOK: &str = "oteryn:creature.the_book_of_death";
const DEATH: &str = "oteryn:creature.concentrated_death";

#[derive(Debug)]
enum Book {
    Tome(ExactActorRef),
    Concentrated(ExactActorRef),
    SpawnDeath(MovementLocalPosition),
    Respawn { next_ms: u64, warnings_left: u8 },
    Gone,
}

#[derive(Debug)]
pub(super) struct Ghulosh {
    boss: ExactActorRef,
    jaw: ExactActorRef,
    book: Book,
    book_position: MovementLocalPosition,
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    stage: usize,
    finished: bool,
}

impl Ghulosh {
    pub(super) fn bind(
        runtime: &ChannelRuntimeV1,
        actors: &[ExactActorRef],
    ) -> Result<Self, CarrierError> {
        let find = |key: &str| {
            actors
                .iter()
                .copied()
                .find(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
                .ok_or(CarrierError::CreatureTargetMismatch)
        };
        for key in [DEATHGAZE, DEATH] {
            runtime.companion_policy(key)?;
        }
        let book = find(BOOK)?;
        Ok(Self {
            boss: find(BASE)?,
            jaw: find("oteryn:creature.bone_jaw")?,
            book: Book::Tome(book),
            book_position: runtime.read_actor_position(book)?.position(),
            scope: runtime.encounter_owner_scope(),
            stage: 0,
            finished: false,
        })
    }

    pub(super) fn is_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.scope == runtime.encounter_owner_scope() && runtime.owner_fence().is_ok()
    }

    pub(super) fn contains_terminal(&self, actor: ExactActorRef) -> bool {
        self.boss == actor
    }

    pub(super) fn contains(&self, actor: ExactActorRef) -> bool {
        self.contains_terminal(actor)
            || matches!(self.book, Book::Tome(book) | Book::Concentrated(book) if book == actor)
    }

    pub(super) fn completed(&self) -> bool {
        self.finished
    }

    pub(super) fn projected_death(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        now_ms: u64,
    ) -> Result<(), CarrierError> {
        runtime.borrow_combat_death().projected_death(actor)?;
        if self.boss == actor {
            self.finished = true;
        } else if matches!(self.book, Book::Tome(book) if book == actor) {
            self.book =
                Book::SpawnDeath(runtime.companion_snapshot_including_dead(actor)?.position);
        } else if matches!(self.book, Book::Concentrated(death) if death == actor) {
            self.book = Book::Respawn {
                next_ms: now_ms.saturating_add(4000),
                warnings_left: 4,
            };
        }
        Ok(())
    }

    pub(super) fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        now_ms: u64,
    ) -> Result<(), CarrierError> {
        if self.finished {
            let book = match self.book {
                Book::Tome(actor) | Book::Concentrated(actor) => Some(actor),
                _ => None,
            };
            for actor in Some(self.jaw).into_iter().chain(book) {
                if runtime.contains_live_creature(actor) {
                    runtime.remove_encounter_actor(actor)?;
                }
            }
            self.book = Book::Gone;
            return Ok(());
        }
        if let Ok(snapshot) = runtime.companion_snapshot(self.boss)
            && snapshot.state.policy.definition_key == BASE
            && let Some(threshold) = [75, 50, 25].get(self.stage)
            && i128::from(snapshot.health) * 100 <= i128::from(snapshot.maximum_health) * threshold
        {
            let change = runtime.prepare_creature_profile_transformation(&snapshot, DEATHGAZE)?;
            runtime.commit_creature_profile_transformation(change)?;
            self.stage += 1;
        }
        let spawn = |runtime: &mut ChannelRuntimeV1, key, at| {
            if !placement(cells, runtime, at) {
                return Err(CarrierError::PlanConflict);
            }
            let policy = runtime.companion_policy(key)?;
            runtime
                .commit_quest_creature_spawn(&[(key, policy.maximum_health, at)])?
                .into_iter()
                .next()
                .ok_or(CarrierError::PlanConflict)
        };
        match self.book {
            Book::SpawnDeath(at) => {
                self.book = Book::Concentrated(spawn(runtime, DEATH, at)?);
            }
            Book::Respawn {
                next_ms,
                warnings_left,
            } if now_ms >= next_ms => {
                if warnings_left == 0 {
                    self.book = Book::Tome(spawn(runtime, BOOK, self.book_position)?);
                } else {
                    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                        index,
                    ) = cells.index()
                    else {
                        return Err(CarrierError::PlanConflict);
                    };
                    let pos = crate::map::overlay::TilePos {
                        x: u16::try_from(self.book_position.x)
                            .map_err(|_| CarrierError::PlanConflict)?,
                        y: u16::try_from(self.book_position.y)
                            .map_err(|_| CarrierError::PlanConflict)?,
                        floor: i8::try_from(-self.book_position.floor)
                            .map_err(|_| CarrierError::PlanConflict)?,
                    };
                    index.map().facts.emit_movement_effect(pos, 11);
                    self.book = Book::Respawn {
                        next_ms: now_ms.saturating_add(2000),
                        warnings_left: warnings_left - 1,
                    };
                }
            }
            _ => {}
        }
        Ok(())
    }
}
