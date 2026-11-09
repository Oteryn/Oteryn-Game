//! Native actor identities keep an encounter occupied even after its creatures leave the room.
use super::*;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use std::sync::Mutex;

const MAX_PERSONAL_ENCOUNTERS: usize = 64;
type Scope = (RuntimeScopeRefV1, ScopeOwnershipGeneration);

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Origin {
    character: [u8; 16],
    cause: QuestCause,
    definition: [u8; 32],
    quest: String,
    revision: u64,
}

impl Origin {
    pub(super) fn new(
        character: [u8; 16],
        cause: QuestCause,
        definition: [u8; 32],
        quest: &str,
        revision: u64,
    ) -> Self {
        Self {
            character,
            cause,
            definition,
            quest: quest.into(),
            revision,
        }
    }
}

struct Cohort {
    scope: Scope,
    callback: String,
    origin: Origin,
    preparing: bool,
    actors: Vec<ExactActorRef>,
}

#[derive(Default)]
pub(super) struct Cohorts(Mutex<Vec<Cohort>>);

fn scope(runtime: &ChannelRuntimeV1) -> Scope {
    (
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
    )
}

impl Cohorts {
    /// Completion retires only the original character/session's eligible projections.
    pub(super) fn complete(
        &self,
        runtime: &mut ChannelRuntimeV1,
        character: &[u8; 16],
        session: GameSessionId,
        quest: &str,
        observed_revision: u64,
    ) -> Result<usize, UseDisposition> {
        if runtime.owner_fence().is_err() {
            return Err(UseDisposition::Rejected);
        }
        let current_scope = scope(runtime);
        let matches = |cohort: &Cohort| {
            cohort.scope == current_scope
                && &cohort.origin.character == character
                && cohort.origin.quest == quest
                && cohort.origin.revision <= observed_revision
                && matches!(cohort.origin.cause, QuestCause::Use(command) | QuestCause::Step(command)
                    if command.game_session_id() == session)
        };
        let mut cohorts = self.0.lock().map_err(|_| UseDisposition::Rejected)?;
        if cohorts
            .iter()
            .any(|cohort| matches(cohort) && cohort.preparing)
        {
            return Err(UseDisposition::Occupied);
        }
        let mut removed = 0;
        let mut index = 0;
        while index < cohorts.len() {
            if !matches(&cohorts[index]) {
                index += 1;
                continue;
            }
            for actor in cohorts[index].actors.iter().copied() {
                if runtime.contains_live_creature(actor) {
                    runtime
                        .remove_encounter_actor(actor)
                        .map_err(|_| UseDisposition::Rejected)?;
                    removed += 1;
                }
            }
            cohorts.swap_remove(index);
        }
        Ok(removed)
    }

    /// Reserve before the durable write. This is projection bookkeeping, never mutation authority.
    pub(super) fn reserve<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
        callback: &str,
        origin: Origin,
    ) -> Result<Reservation<'a>, UseDisposition> {
        let scope = scope(runtime);
        let mut cohorts = self.0.lock().map_err(|_| UseDisposition::Rejected)?;
        cohorts.retain(|cohort| {
            cohort.scope.0 != scope.0
                || (cohort.scope.1 == scope.1
                    && (cohort.preparing
                        || cohort
                            .actors
                            .iter()
                            .any(|actor| runtime.contains_live_creature(*actor))))
        });
        if cohorts
            .iter()
            .any(|cohort| cohort.scope == scope && cohort.callback == callback)
        {
            return Err(UseDisposition::Occupied);
        }
        if cohorts.len() >= MAX_PERSONAL_ENCOUNTERS {
            return Err(UseDisposition::Exhausted);
        }
        cohorts
            .try_reserve(1)
            .map_err(|_| UseDisposition::Exhausted)?;
        cohorts.push(Cohort {
            scope,
            callback: callback.into(),
            origin: origin.clone(),
            preparing: true,
            actors: Vec::new(),
        });
        Ok(Reservation {
            pool: self,
            scope,
            callback: callback.into(),
            origin,
            published: false,
        })
    }
}

pub(super) struct Reservation<'a> {
    pool: &'a Cohorts,
    scope: Scope,
    callback: String,
    origin: Origin,
    published: bool,
}

impl Reservation<'_> {
    /// Called only after the fenced Quest receipt and atomic carrier/map publication succeeded.
    pub(super) fn publish(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actors: Vec<ExactActorRef>,
    ) -> Result<(), UseDisposition> {
        if self.scope != scope(runtime)
            || actors.is_empty()
            || actors.len() > 8
            || actors.iter().enumerate().any(|(index, actor)| {
                !runtime.contains_live_creature(*actor) || actors[..index].contains(actor)
            })
        {
            return Err(UseDisposition::Rejected);
        }
        let mut cohorts = self.pool.0.lock().map_err(|_| UseDisposition::Rejected)?;
        let cohort = cohorts
            .iter_mut()
            .find(|cohort| {
                cohort.scope == self.scope
                    && cohort.callback == self.callback
                    && cohort.origin == self.origin
                    && cohort.preparing
            })
            .ok_or(UseDisposition::Rejected)?;
        cohort.actors = actors;
        cohort.preparing = false;
        self.published = true;
        Ok(())
    }
}

impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if !self.published
            && let Ok(mut cohorts) = self.pool.0.lock()
        {
            cohorts.retain(|cohort| {
                !(cohort.scope == self.scope
                    && cohort.callback == self.callback
                    && cohort.origin == self.origin
                    && cohort.preparing)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::channel_content_pin::ChannelContentPin;
    use crate::foundation::{ChannelId, NodeId, WorldId};
    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
    fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
        value.map_err(|error| format!("{error:?}").into())
    }
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn runtime(generation: u64) -> TestResult<ChannelRuntimeV1> {
        let world = checked(WorldId::decode(&id(1)))?;
        checked(ChannelRuntimeV1::from_committed_assignment(
            world,
            checked(ChannelId::decode(&id(2)))?,
            checked(NodeId::decode(&id(3)))?,
            1,
            generation,
            generation,
            &format!("runtime-scope-assignment:{generation}"),
            8,
            ChannelContentPin::test(world),
        ))
    }
    fn origin() -> TestResult<Origin> {
        Ok(Origin::new(
            id(41),
            QuestCause::Step(CommandRef::new(
                checked(GameSessionId::decode(&id(5)))?,
                checked(CommandId::new(1))?,
            )),
            [1; 32],
            "oteryn:quest.fixture",
            10,
        ))
    }
    #[test]
    fn worldchg_completion_preserves_foreign_and_newer_cohorts() -> TestResult {
        let mut runtime = runtime(1)?;
        let pool = Cohorts::default();
        let owner = origin()?;
        let mut origins = vec![
            owner.clone(),
            owner.clone(),
            owner.clone(),
            owner.clone(),
            owner.clone(),
        ];
        origins[1].character = id(42);
        origins[2].quest = "oteryn:quest.other".into();
        origins[3].revision = 20;
        origins[4].cause = QuestCause::Step(CommandRef::new(
            checked(GameSessionId::decode(&id(6)))?,
            checked(CommandId::new(1))?,
        ));
        let mut actors = Vec::new();
        for (index, origin) in origins.into_iter().enumerate() {
            let callback = format!("fixture:encounter/{index}");
            let mut reservation = checked(pool.reserve(&runtime, &callback, origin))?;
            let actor = checked(runtime.admit_encounter_actor(
                "creature:rat",
                20,
                MovementLocalPosition {
                    x: 1000 + i32::try_from(index)?,
                    y: 1000,
                    floor: 7,
                },
            ))?;
            checked(reservation.publish(&runtime, vec![actor]))?;
            actors.push(actor);
        }
        let session = checked(GameSessionId::decode(&id(5)))?;
        assert_eq!(
            checked(pool.complete(&mut runtime, &owner.character, session, &owner.quest, 15))?,
            1
        );
        assert!(!runtime.contains_live_creature(actors[0]));
        assert!(
            actors[1..]
                .iter()
                .all(|actor| runtime.contains_live_creature(*actor))
        );
        assert_eq!(
            checked(pool.complete(&mut runtime, &owner.character, session, &owner.quest, 15))?,
            0,
            "a retained completion cannot clear a newer activation"
        );
        assert!(
            actors[1..]
                .iter()
                .all(|actor| runtime.contains_live_creature(*actor))
        );
        let mut replacement = self::runtime(2)?;
        let replacement_actor = checked(replacement.admit_encounter_actor(
            "creature:rat",
            20,
            MovementLocalPosition {
                x: 1000,
                y: 1000,
                floor: 7,
            },
        ))?;
        assert_eq!(
            checked(pool.complete(
                &mut replacement,
                &owner.character,
                session,
                &owner.quest,
                100
            ))?,
            0
        );
        assert!(replacement.contains_live_creature(replacement_actor));
        Ok(())
    }

    #[test]
    fn worldchg_completion_refuses_an_unfinished_projection_before_removing_any_actor() -> TestResult
    {
        let mut runtime = runtime(1)?;
        let pool = Cohorts::default();
        let owner = origin()?;
        let mut published = checked(pool.reserve(&runtime, "fixture:published", owner.clone()))?;
        let actor = checked(runtime.admit_encounter_actor(
            "creature:rat",
            20,
            MovementLocalPosition {
                x: 1000,
                y: 1000,
                floor: 7,
            },
        ))?;
        checked(published.publish(&runtime, vec![actor]))?;
        drop(published);
        let preparing = checked(pool.reserve(&runtime, "fixture:preparing", owner.clone()))?;
        let session = checked(GameSessionId::decode(&id(5)))?;
        assert_eq!(
            pool.complete(&mut runtime, &owner.character, session, &owner.quest, 15),
            Err(UseDisposition::Occupied)
        );
        assert!(runtime.contains_live_creature(actor));
        drop(preparing);
        assert_eq!(
            checked(pool.complete(&mut runtime, &owner.character, session, &owner.quest, 15))?,
            1
        );
        Ok(())
    }

    #[test]
    fn worldchg_encounter_roster_outside_room_still_blocks_until_scope_replacement() -> TestResult {
        let mut runtime = runtime(1)?;
        let pool = Cohorts::default();
        let mut reservation = checked(pool.reserve(&runtime, "fixture:encounter", origin()?))?;
        let actor = checked(runtime.admit_encounter_actor(
            "creature:rat",
            20,
            MovementLocalPosition {
                x: 1000,
                y: 1000,
                floor: 7,
            },
        ))?;
        checked(reservation.publish(&runtime, vec![actor]))?;
        drop(reservation);
        assert!(matches!(
            pool.reserve(&runtime, "fixture:encounter", origin()?),
            Err(UseDisposition::Occupied)
        ));
        let replacement = self::runtime(2)?;
        assert!(
            pool.reserve(&replacement, "fixture:encounter", origin()?)
                .is_ok()
        );
        Ok(())
    }
    #[test]
    fn worldchg_failed_activation_releases_the_bounded_projection_reservation() -> TestResult {
        let runtime = runtime(1)?;
        let pool = Cohorts::default();
        let mut held = Vec::new();
        for n in 0..MAX_PERSONAL_ENCOUNTERS {
            held.push(checked(pool.reserve(
                &runtime,
                &format!("fixture:encounter/{n}"),
                origin()?,
            ))?);
        }
        assert!(matches!(
            pool.reserve(&runtime, "fixture:overflow", origin()?),
            Err(UseDisposition::Exhausted)
        ));
        drop(held);
        assert!(
            pool.reserve(&runtime, "fixture:overflow", origin()?)
                .is_ok()
        );
        Ok(())
    }
}
