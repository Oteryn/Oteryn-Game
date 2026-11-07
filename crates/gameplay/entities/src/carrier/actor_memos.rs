use super::*;
use std::collections::BTreeMap;

/// SPAWN-CEILINGS-1 (Codex 4243039220): replay memos keyed by an exact creature and one of its
/// entries (a summon entry or a defence Ability), deterministic ordered maps throughout. The
/// memos of a creature live under its slot, which holds one creature at a time: a creature's
/// first touch drops what the slot's former occupant left (at most its entries), so a lookup,
/// an insert or that cleanup costs the logarithm of the table plus the entries of one slot,
/// never a pass over every memo. The memos of a dead creature whose slot nobody reuses stay
/// until [`Self::prune`], which the owners run only when the table is full.
#[derive(Debug)]
pub struct ActorMemoTable<K, V> {
    slots: BTreeMap<u32, ActorMemos<K, V>>,
    len: usize,
}

#[derive(Debug)]
struct ActorMemos<K, V> {
    actor: ExactActorRef,
    entries: BTreeMap<K, V>,
}

impl<K, V> Default for ActorMemoTable<K, V> {
    fn default() -> Self {
        Self {
            slots: BTreeMap::new(),
            len: 0,
        }
    }
}

impl<K: Ord, V> ActorMemoTable<K, V> {
    /// Every memo held, a dead creature's included.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the table holds no memos.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The memos of exactly `actor`.
    pub fn actor_len(&self, actor: ExactActorRef) -> usize {
        self.slots
            .get(&actor.actor_local_id())
            .filter(|memos| memos.actor == actor)
            .map_or(0, |memos| memos.entries.len())
    }
    pub fn get<Q>(&self, actor: ExactActorRef, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if self.is_empty() {
            return None;
        }
        self.slots
            .get(&actor.actor_local_id())
            .filter(|memos| memos.actor == actor)?
            .entries
            .get(key)
    }
    /// Drops the memos `actor`'s slot holds for any other actor: its former occupant, dead
    /// because the slot was taken again, or one of another scope. Returns how many it dropped.
    pub fn evict_stale_at(&mut self, actor: ExactActorRef) -> usize {
        let slot = actor.actor_local_id();
        if self
            .slots
            .get(&slot)
            .is_none_or(|memos| memos.actor == actor)
        {
            return 0;
        }
        let dropped = self
            .slots
            .remove(&slot)
            .map_or(0, |memos| memos.entries.len());
        self.len -= dropped;
        dropped
    }
    /// Sets `actor`'s memo of `key`, first dropping its slot's stale memos; returns the memo it
    /// replaced.
    pub fn insert(&mut self, actor: ExactActorRef, key: K, value: V) -> Option<V> {
        self.evict_stale_at(actor);
        let memos = self
            .slots
            .entry(actor.actor_local_id())
            .or_insert_with(|| ActorMemos {
                actor,
                entries: BTreeMap::new(),
            });
        let replaced = memos.entries.insert(key, value);
        if replaced.is_none() {
            self.len += 1;
        }
        replaced
    }
    /// Drops the memos of every actor `live` refuses: one pass over the slots that hold memos,
    /// which the owners run only when the table is full.
    pub fn prune(&mut self, live: impl Fn(ExactActorRef) -> bool) {
        let mut len = 0;
        self.slots.retain(|_, memos| {
            let keep = live(memos.actor);
            if keep {
                len += memos.entries.len();
            }
            keep
        });
        self.len = len;
    }
}
