//! Shared scope-local exit windows, opened once by a committed creature death.

use crate::content::native_cell_lookup::{BundleCollisionIndex, NativeMovementCollisionIndex};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use crate::map::{
    boot::{BundleCollisionIndexExt, BundleMap},
    view::MapFacts,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, DeathWindow};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct DeathWindows {
    exits: BTreeMap<String, Exit>,
    seen: Vec<ExactActorRef>,
    map: Option<BundleCollisionIndex>,
}

#[derive(Debug)]
struct Exit {
    rule: DeathWindow,
    count: u8,
    due: u64,
    last: Option<ExactActorRef>,
    opened_by: Option<ExactActorRef>,
    visual: Option<Callback>,
}

impl DeathWindows {
    pub(in crate::gameplay_transport) fn install(
        &mut self,
        callbacks: &[Callback],
        index: &NativeMovementCollisionIndex,
    ) {
        let map = match index {
            NativeMovementCollisionIndex::Bundle(index)
                if callbacks.iter().any(|callback| {
                    callback
                        .effects
                        .death_window
                        .as_ref()
                        .is_some_and(|rule| rule.open_appearance.is_some())
                }) =>
            {
                Some(index)
            }
            _ => None,
        };
        if self.map.as_ref() != map {
            self.exits.retain(|_, exit| exit.visual.is_none());
            self.map = map.cloned();
        }
        self.exits.retain(|key, exit| {
            callbacks.iter().any(|callback| {
                callback.key == *key
                    && callback.effects.death_window.as_ref() == Some(&exit.rule)
                    && exit.visual.as_ref().is_none_or(|old| old == callback)
            })
        });
        for callback in callbacks {
            if let Some(rule) = &callback.effects.death_window {
                self.exits
                    .entry(callback.key.clone())
                    .or_insert_with(|| Exit {
                        rule: rule.clone(),
                        count: 0,
                        due: 0,
                        last: None,
                        opened_by: None,
                        visual: rule.open_appearance.map(|_| callback.clone()),
                    });
            }
        }
    }

    pub(in crate::gameplay_transport) fn observe(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        creature: &str,
        projection_ms: u64,
    ) -> bool {
        if self.seen.contains(&target)
            || !self
                .exits
                .values()
                .any(|exit| exit.rule.creatures.iter().any(|key| key == creature))
            || runtime.owner_fence().is_err()
        {
            return false;
        }
        let owner = runtime.borrow_combat_death();
        let Ok((_, point)) = owner.projected_death(target) else {
            return false;
        };
        let Ok(when) = owner.projected_death_at_ms(target) else {
            return false;
        };
        let when = when.unwrap_or(projection_ms);
        self.seen
            .retain(|actor| runtime.retains_corpse_projection(*actor));
        self.seen.push(target);

        let mut projected = false;
        let map = self.map.as_ref().map(BundleCollisionIndexExt::map);
        for exit in self.exits.values_mut().filter(|exit| {
            exit.rule.creatures.iter().any(|key| key == creature)
                && exit
                    .rule
                    .area
                    .is_none_or(|area| area.contains([point.x, point.y, i32::from(point.floor)]))
        }) {
            if !exit.current(runtime) || exit.due != 0 && when >= exit.due {
                exit.count = 0;
                exit.due = 0;
                exit.opened_by = None;
            }
            exit.last = Some(target);
            exit.count = if exit.rule.required_deaths == 1 {
                1
            } else {
                exit.count.saturating_add(1)
            };
            if exit.count == exit.rule.required_deaths {
                let due = when.saturating_add(u64::from(exit.rule.duration_ms));
                // A repeat kill cannot extend the original scheduled reset.
                // Crystal201189f creaturescripts_inquisition_boss.lua:25-29
                // and creaturescripts_bosses_kill.lua:126-134.
                let due = if exit.due == 0 {
                    due
                } else {
                    exit.due.min(due)
                };
                if exit.opened_by.is_none()
                    && let Some(callback) = &exit.visual
                {
                    if !map.is_some_and(|map| project_appearance(map, callback, projection_ms, due))
                    {
                        continue;
                    }
                    projected = true;
                }
                exit.opened_by.get_or_insert(target);
                exit.due = due;
            }
        }
        projected
    }

    pub(in crate::gameplay_transport) fn open_source(
        &self,
        runtime: &ChannelRuntimeV1,
        key: &str,
        rule: &DeathWindow,
        now_ms: u64,
    ) -> Option<ExactActorRef> {
        let exit = self.exits.get(key)?;
        if runtime.owner_fence().is_err()
            || &exit.rule != rule
            || !exit.current(runtime)
            || exit.count < rule.required_deaths
            || now_ms >= exit.due
        {
            return None;
        }
        exit.opened_by
    }

    pub(in crate::gameplay_transport) fn consume(
        &mut self,
        key: &str,
        rule: &DeathWindow,
        opened_by: ExactActorRef,
    ) {
        if let Some(exit) = self.exits.get_mut(key)
            && &exit.rule == rule
            && exit.opened_by == Some(opened_by)
        {
            exit.count = 0;
            exit.opened_by = None;
        }
    }
}

impl Exit {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        self.last.is_some_and(|actor| {
            actor.runtime_scope() == binding.runtime_scope()
                && actor.scope_generation() == binding.scope_generation()
        })
    }
}

// Crystal 201189f8 Ferumbras creaturescripts_bosses_kill.lua:126-134.
fn project_appearance(map: &BundleMap, callback: &Callback, now_ms: u64, due_ms: u64) -> bool {
    let Some((position, closed, open)) = callback
        .selector
        .position
        .zip(callback.selector.appearance)
        .zip(
            callback
                .effects
                .death_window
                .as_ref()
                .and_then(|window| window.open_appearance),
        )
        .map(|((position, closed), open)| (position, closed, open))
    else {
        return false;
    };
    let pos = crate::map::overlay::TilePos {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let Some(base) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return false;
    };
    let mut matches = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = crate::map::view::placement_key(pos, ordinal)?;
            (super::super::quest_callbacks::matches(map, callback, key)
                && !map.facts.base_hidden(key))
            .then_some((key, ordinal, *id))
        });
    let Some((key, ordinal, id)) = matches.next() else {
        return false;
    };
    if matches.next().is_some() || now_ms >= due_ms {
        return false;
    }
    // If the previous timer is due on the same owner turn, restore its base
    // prior to selecting the exact closed portal for the next death.
    map.facts
        .expire_movement_items(now_ms.saturating_mul(1_000));
    let Some(before) = map.facts.base_entry(pos, ordinal, id) else {
        return false;
    };
    let Some(after) = map.facts.quest_callback_replacement(key, open) else {
        return false;
    };
    if before.appearance_id != closed || before.pickupable || after.pickupable {
        return false;
    }
    map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        after,
        &[],
        &[],
        false,
        Some(
            &crate::map::facts::QuestCallbackProjection::TimedTransform {
                due: due_ms.saturating_mul(1_000),
                restore: before,
            },
        ),
    )
}
