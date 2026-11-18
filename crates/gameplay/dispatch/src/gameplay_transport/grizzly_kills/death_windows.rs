//! Shared scope-local exit windows, opened once by a committed creature death.

use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, DeathWindow};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct DeathWindows {
    exits: BTreeMap<String, Exit>,
    seen: Vec<ExactActorRef>,
}

#[derive(Debug)]
struct Exit {
    rule: DeathWindow,
    count: u8,
    due: u64,
    last: Option<ExactActorRef>,
    opened_by: Option<ExactActorRef>,
}

impl DeathWindows {
    pub(in crate::gameplay_transport) fn install(&mut self, callbacks: &[Callback]) {
        self.exits.retain(|key, exit| {
            callbacks.iter().any(|callback| {
                callback.key == *key && callback.effects.death_window.as_ref() == Some(&exit.rule)
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
    ) {
        if self.seen.contains(&target)
            || !self
                .exits
                .values()
                .any(|exit| exit.rule.creatures.iter().any(|key| key == creature))
            || runtime.owner_fence().is_err()
        {
            return;
        }
        let owner = runtime.borrow_combat_death();
        let Ok((_, point)) = owner.projected_death(target) else {
            return;
        };
        let Ok(when) = owner.projected_death_at_ms(target) else {
            return;
        };
        let when = when.unwrap_or(projection_ms);
        self.seen
            .retain(|actor| runtime.retains_corpse_projection(*actor));
        self.seen.push(target);

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
                exit.opened_by = Some(target);
                exit.due = when.saturating_add(u64::from(exit.rule.duration_ms));
            }
        }
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
