//! Per-creature think hooks: the Oteryn seam for Crystal `mType.onThink`
//! (monster.cpp:1096-1125). A hook observes one committed think; it never changes or
//! suppresses the think, so the donor `return true` early exit has no equivalent here.

use std::collections::BTreeMap;

use super::{CreatureAiTable, ThinkReport};
use crate::foundation::{ExactActorRef, MovementLocalPosition};

/// One committed think of a creature whose key has a registered hook.
#[derive(Clone, Copy, Debug)]
pub struct CreatureThinkCall<'a> {
    pub actor: ExactActorRef,
    pub creature_key: &'a str,
    pub position: MovementLocalPosition,
    pub now_us: u64,
    pub report: &'a ThinkReport,
}

/// A registered think. Rules modules implement it (or pass a closure); the hook keeps its own
/// state and queues its own consequences for its owner.
pub trait CreatureThinkHook: Send + Sync {
    fn on_think(&mut self, call: &CreatureThinkCall<'_>);
}

impl<F> CreatureThinkHook for F
where
    F: FnMut(&CreatureThinkCall<'_>) + Send + Sync,
{
    fn on_think(&mut self, call: &CreatureThinkCall<'_>) {
        self(call);
    }
}

#[derive(Default)]
pub struct CreatureThinkHooks {
    hooks: BTreeMap<String, Box<dyn CreatureThinkHook>>,
}

impl CreatureThinkHooks {
    /// Registers `hook` for every creature admitted with `creature_key`; returns the hook it
    /// replaced, if any.
    pub fn register(
        &mut self,
        creature_key: impl Into<String>,
        hook: impl CreatureThinkHook + 'static,
    ) -> Option<Box<dyn CreatureThinkHook>> {
        self.hooks.insert(creature_key.into(), Box::new(hook))
    }

    /// Returns whether a hook was removed.
    pub fn unregister(&mut self, creature_key: &str) -> bool {
        self.hooks.remove(creature_key).is_some()
    }

    #[must_use]
    pub fn is_registered(&self, creature_key: &str) -> bool {
        self.hooks.contains_key(creature_key)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Runs the hook registered for `call.creature_key`; returns whether one ran.
    pub fn run(&mut self, call: &CreatureThinkCall<'_>) -> bool {
        let Some(hook) = self.hooks.get_mut(call.creature_key) else {
            return false;
        };
        hook.on_think(call);
        true
    }
}

impl std::fmt::Debug for CreatureThinkHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatureThinkHooks")
            .field("keys", &self.hooks.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl CreatureAiTable {
    /// Runs the think hook of `actor`'s creature key after a committed think; a creature with
    /// no key or no registered hook is untouched. Returns whether a hook ran.
    pub(crate) fn run_think_hook(
        &mut self,
        actor: ExactActorRef,
        position: MovementLocalPosition,
        now_us: u64,
        report: &ThinkReport,
    ) -> bool {
        let Some(key) = self
            .entry(actor)
            .and_then(|entry| entry.state.creature_key.clone())
        else {
            return false;
        };
        let call = CreatureThinkCall {
            actor,
            creature_key: &key,
            position,
            now_us,
            report,
        };
        self.think_hooks.run(&call)
    }
}
