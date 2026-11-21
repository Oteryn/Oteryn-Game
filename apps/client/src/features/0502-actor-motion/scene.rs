use super::*;
use oteryn_client_simulation::walk::Glide;
use oteryn_session::{ActorPosition, EntityRef, WorldSpatialEntity};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub(super) struct ActorClock {
    position: ActorPosition,
    revision: u64,
    since: Instant,
    glide: Option<Glide>,
}

#[derive(Clone, Debug)]
pub(super) struct ActorClocks {
    pub(super) entries: BTreeMap<EntityRef, ActorClock>,
    pub(super) now: Instant,
    pub(super) own_walk: Option<(Glide, Instant)>,
}
impl Default for ActorClocks {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            now: Instant::now(),
            own_walk: None,
        }
    }
}

impl Scene {
    pub(crate) fn actor_offset(&self, entity: EntityRef) -> [f32; 2] {
        if self.own_identity == Some(entity.identity) {
            return [0.; 2];
        }
        self.actor_clocks
            .entries
            .get(&entity)
            .and_then(|clock| {
                let glide = clock.glide?;
                let elapsed = self.actor_clocks.now.saturating_duration_since(clock.since);
                (!glide.finished(elapsed))
                    .then(|| glide.offset(elapsed, self.view.tile_px() as f32))
            })
            .unwrap_or([0.; 2])
    }

    pub(crate) fn reset_actor_animation(&mut self) {
        self.actor_clocks = ActorClocks::default();
    }

    pub(crate) fn actor_animation(&mut self, now: Instant, walk: Option<(Glide, Instant)>) {
        self.actor_clocks.now = now;
        self.actor_clocks.own_walk = walk;
    }
}

impl ActorClocks {
    pub(super) fn sample(
        &mut self,
        entity: &WorldSpatialEntity,
        own: bool,
    ) -> ((u64, Option<u32>), [f32; 2]) {
        let Some(motion) = entity.motion else {
            return ((0, None), [0.; 2]);
        };
        let clock = self.entries.entry(entity.entity).or_insert(ActorClock {
            position: entity.position,
            revision: motion.translation_revision,
            since: self.now,
            glide: None,
        });
        if clock.revision != motion.translation_revision {
            let from = TileCoord::new(clock.position.x, clock.position.y);
            let to = TileCoord::new(entity.position.x, entity.position.y);
            let previous = clock.glide;
            clock.glide = (clock.position.floor == entity.position.floor)
                .then(|| {
                    Glide::new(
                        from,
                        to,
                        Duration::from_millis(u64::from(motion.step_duration_ms)),
                    )
                })
                .flatten()
                .map(|glide| {
                    previous.map_or(glide, |previous| {
                        glide.continuing(previous, self.now.saturating_duration_since(clock.since))
                    })
                });
            clock.position = entity.position;
            clock.revision = motion.translation_revision;
            clock.since = self.now;
        }
        let walk = if own {
            self.own_walk
        } else {
            clock.glide.map(|glide| (glide, clock.since))
        };
        if let Some((glide, start)) = walk {
            let elapsed = self.now.saturating_duration_since(start);
            if !glide.finished(elapsed) {
                return (
                    (
                        elapsed.as_millis() as u64,
                        Some(glide.duration().as_millis() as u32),
                    ),
                    glide.offset(elapsed, 32.),
                );
            }
        }
        (
            (
                self.now.saturating_duration_since(clock.since).as_millis() as u64,
                None,
            ),
            [0.; 2],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{EntityDetail, EntityKind, StepDirection, actor_motion::ActorMotion};

    #[test]
    fn actor_motion_remote_gaits_use_each_step_duration_and_adjacent_teleports_snap()
    -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let mut clocks = ActorClocks {
            now,
            ..ActorClocks::default()
        };
        let actor = |tag| WorldSpatialEntity {
            motion: Some(ActorMotion::default()),
            display_name: None,
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [tag; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::East,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let (mut fast, mut slow) = (actor(1), actor(2));
        clocks.sample(&fast, false);
        clocks.sample(&slow, false);
        for (entity, ms) in [(&mut fast, 200), (&mut slow, 800)] {
            entity.position.x = 1;
            entity.motion = Some(ActorMotion {
                translation_revision: 1,
                step_duration_ms: ms,
                ..ActorMotion::default()
            });
            clocks.sample(entity, false);
        }
        clocks.now = now + Duration::from_millis(100);
        assert_eq!(clocks.sample(&fast, false), ((100, Some(200)), [-16., 0.]));
        assert_eq!(clocks.sample(&slow, false), ((100, Some(800)), [-28., 0.]));
        // Same adjacent distance, explicitly a relocation, including while a walk is active.
        slow.position.x = 2;
        slow.motion = Some(ActorMotion {
            translation_revision: 2,
            ..ActorMotion::default()
        });
        assert_eq!(clocks.sample(&slow, false), ((0, None), [0.; 2]));
        clocks.now = now + Duration::from_millis(200);
        assert_eq!(clocks.sample(&fast, false).0.1, None);
        fast.position.x = 2;
        fast.motion = Some(ActorMotion {
            translation_revision: 2,
            step_duration_ms: 200,
            ..ActorMotion::default()
        });
        clocks.sample(&fast, false);
        clocks.now = now + Duration::from_millis(300);
        fast.position.x = 3;
        fast.motion = Some(ActorMotion {
            translation_revision: 3,
            step_duration_ms: 200,
            ..ActorMotion::default()
        });
        assert_eq!(clocks.sample(&fast, false).1, [-48., 0.]);
        Ok(())
    }
}
